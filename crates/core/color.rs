use std::io::{self, Write};

use termcolor::{ColorSpec, HyperlinkSpec, WriteColor};

/// Clear background colors left in blank cells when a colored line scrolls.
#[derive(Clone, Debug)]
pub(crate) struct ColorWriter<W> {
    wtr: W,
    width: Option<usize>,
    column: Option<usize>,
    check_resize: bool,
    background: bool,
}

impl ColorWriter<grep::cli::StandardStream> {
    pub(crate) fn stdout(
        wtr: grep::cli::StandardStream,
        width: Option<usize>,
    ) -> Self {
        // These concrete termcolor writers support hyperlinks only when they
        // emit ANSI escapes, not when using the native Windows console API.
        let ansi = wtr.supports_hyperlinks();
        Self::new(wtr, ansi, width).check_resize()
    }
}

impl ColorWriter<termcolor::Buffer> {
    pub(crate) fn buffer(
        wtr: termcolor::Buffer,
        width: Option<usize>,
    ) -> Self {
        let ansi = wtr.supports_hyperlinks();
        Self::new(wtr, ansi, width).check_resize()
    }

    pub(crate) fn clear(&mut self) {
        self.wtr.clear();
        self.background = false;
        self.refresh_width();
        self.column = self.width.map(|_| 0);
    }
}

impl<W> ColorWriter<W> {
    fn new(wtr: W, ansi: bool, width: Option<usize>) -> Self {
        let width = width.filter(|&width| ansi && width > 0);
        Self {
            wtr,
            width,
            column: width.map(|_| 0),
            check_resize: false,
            background: false,
        }
    }

    fn check_resize(mut self) -> Self {
        self.check_resize = true;
        self
    }

    fn refresh_width(&mut self) {
        if self.check_resize && self.width.is_some() {
            let width = terminal_size::terminal_size_of(std::io::stdout())
                .map(|(width, _)| usize::from(width.0))
                .filter(|&width| width > 0);
            self.update_width(width);
        }
    }

    fn update_width(&mut self, width: Option<usize>) {
        if self.width != width {
            self.width = width;
            self.column = None;
        }
    }

    fn advance(&mut self, buf: &[u8]) {
        let Some(width) = self.width else { return };
        for &byte in buf {
            match byte {
                b'\n' | b'\r' => self.column = Some(0),
                b' '..=b'~' => {
                    if let Some(column) = self.column {
                        self.column =
                            Some(if column == width { 1 } else { column + 1 });
                    }
                }
                b'\x1B' => {
                    // Raw terminal controls can change subsequent cursor
                    // positions in ways that a line-oriented writer cannot know.
                    self.width = None;
                    self.column = None;
                    return;
                }
                // Tabs depend on user-defined stops, and Unicode cell widths
                // vary by terminal. Resume tracking at the next line boundary.
                _ => self.column = None,
            }
        }
    }

    pub(crate) fn get_ref(&self) -> &W {
        &self.wtr
    }
}

impl<W: Write> Write for ColorWriter<W> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let written = self.wtr.write(buf)?;
        self.advance(&buf[..written]);
        Ok(written)
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.wtr.write_all(buf)?;
        self.advance(buf);
        Ok(())
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.wtr.flush()
    }
}

impl<W: WriteColor> WriteColor for ColorWriter<W> {
    fn supports_color(&self) -> bool {
        self.wtr.supports_color()
    }

    fn supports_hyperlinks(&self) -> bool {
        self.wtr.supports_hyperlinks()
    }

    fn set_color(&mut self, spec: &ColorSpec) -> io::Result<()> {
        self.wtr.set_color(spec)?;
        let background = self.width.is_some()
            && (spec.bg().is_some() || (!spec.reset() && self.background));
        if self.background && !background {
            self.clear_background()?;
        }
        self.background = background;
        Ok(())
    }

    fn set_hyperlink(&mut self, link: &HyperlinkSpec) -> io::Result<()> {
        self.wtr.set_hyperlink(link)
    }

    fn reset(&mut self) -> io::Result<()> {
        self.wtr.reset()?;
        if self.background {
            // Scrolling while the background is set fills the new row with
            // that color. Resetting SGR does not repaint its remaining cells.
            self.clear_background()?;
        }
        self.background = false;
        Ok(())
    }

    fn is_synchronous(&self) -> bool {
        self.wtr.is_synchronous()
    }
}

impl<W: Write> ColorWriter<W> {
    fn clear_background(&mut self) -> io::Result<()> {
        if self.column.is_none() {
            return Ok(());
        }
        self.refresh_width();
        if let (Some(width), Some(column)) = (self.width, self.column) {
            // At the right margin, EL can erase the character just printed and
            // cancel pending wrap. There are no unused cells to clear there.
            if column < width {
                self.wtr.write_all(b"\x1B[K")?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use termcolor::{Ansi, Color, NoColor};

    fn colored_output(
        prefix: &[u8],
        matched: &[u8],
        width: Option<usize>,
    ) -> Vec<u8> {
        let mut wtr = ColorWriter::new(Ansi::new(vec![]), true, width);
        wtr.write_all(prefix).unwrap();
        let mut spec = ColorSpec::new();
        spec.set_bg(Some(Color::Blue));
        wtr.set_color(&spec).unwrap();
        wtr.write_all(matched).unwrap();
        wtr.reset().unwrap();
        wtr.wtr.into_inner()
    }

    #[test]
    fn exact_right_margin_does_not_clear() {
        assert_eq!(
            colored_output(b"12345678", b"ME", Some(10)),
            b"12345678\x1B[0m\x1B[44mME\x1B[0m",
        );
    }

    #[test]
    fn wrapped_match_clears_unused_cells() {
        assert_eq!(
            colored_output(b"123456789", b"ME", Some(10)),
            b"123456789\x1B[0m\x1B[44mME\x1B[0m\x1B[K",
        );
    }

    #[test]
    fn unknown_width_does_not_clear() {
        assert_eq!(
            colored_output(b"before ", b"ME", None),
            b"before \x1B[0m\x1B[44mME\x1B[0m",
        );
    }

    #[test]
    fn unicode_and_tabs_do_not_guess_columns() {
        for prefix in ["12345678界", "123\t"] {
            let output = colored_output(prefix.as_bytes(), b"ME", Some(10));
            assert!(!output.windows(3).any(|window| window == b"\x1B[K"));
        }
    }

    #[test]
    fn line_boundary_recovers_unknown_columns() {
        let output =
            colored_output("界\t\n123456789".as_bytes(), b"ME", Some(10));
        assert!(output.ends_with(b"\x1B[0m\x1B[K"));
    }

    #[test]
    fn raw_terminal_controls_disable_clearing() {
        let output = colored_output(b"\x1B[10C\n123456789", b"ME", Some(10));
        assert!(!output.windows(3).any(|window| window == b"\x1B[K"));
    }

    #[test]
    fn resize_waits_for_a_line_boundary() {
        let mut wtr = ColorWriter::new(Ansi::new(vec![]), true, Some(10));
        wtr.write_all(b"123456789").unwrap();
        wtr.update_width(Some(20));
        let mut spec = ColorSpec::new();
        spec.set_bg(Some(Color::Blue));
        wtr.set_color(&spec).unwrap();
        wtr.write_all(b"ME").unwrap();
        wtr.reset().unwrap();
        assert!(
            !wtr.get_ref()
                .get_ref()
                .windows(3)
                .any(|window| window == b"\x1B[K")
        );
        wtr.write_all(b"\n").unwrap();
        wtr.set_color(&spec).unwrap();
        wtr.write_all(b"ME").unwrap();
        wtr.reset().unwrap();
        assert!(wtr.get_ref().get_ref().ends_with(b"\x1B[0m\x1B[K"));
    }

    #[test]
    fn partial_writes_count_only_written_columns() {
        #[derive(Default)]
        struct ShortWriter(Vec<u8>);

        impl Write for ShortWriter {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                let written = buf.len().min(2);
                self.0.extend_from_slice(&buf[..written]);
                Ok(written)
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let mut wtr = ColorWriter::new(
            Ansi::new(ShortWriter::default()),
            true,
            Some(10),
        );
        assert_eq!(wtr.write(b"12345678").unwrap(), 2);
        wtr.write_all(b"345678").unwrap();
        let mut spec = ColorSpec::new();
        spec.set_bg(Some(Color::Blue));
        wtr.set_color(&spec).unwrap();
        wtr.write_all(b"ME").unwrap();
        wtr.reset().unwrap();
        assert_eq!(
            wtr.get_ref().get_ref().0,
            b"12345678\x1B[0m\x1B[44mME\x1B[0m"
        );
    }

    #[test]
    fn clears_after_background_reset() {
        let mut wtr = ColorWriter::new(Ansi::new(vec![]), true, Some(80));
        let mut spec = ColorSpec::new();
        spec.set_bg(Some(Color::Blue));
        wtr.set_color(&spec).unwrap();
        wtr.write_all(b"match").unwrap();
        wtr.reset().unwrap();
        wtr.write_all(b" tail\n").unwrap();
        assert_eq!(
            wtr.get_ref().get_ref(),
            b"\x1B[0m\x1B[44mmatch\x1B[0m\x1B[K tail\n"
        );
    }

    #[test]
    fn foreground_colors_do_not_clear() {
        let mut wtr = ColorWriter::new(Ansi::new(vec![]), true, Some(80));
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(Color::Blue));
        wtr.set_color(&spec).unwrap();
        wtr.write_all(b"match").unwrap();
        wtr.reset().unwrap();
        assert_eq!(wtr.get_ref().get_ref(), b"\x1B[0m\x1B[34mmatch\x1B[0m");
    }

    #[test]
    fn nonresetting_color_preserves_background() {
        let mut wtr = ColorWriter::new(Ansi::new(vec![]), true, Some(80));
        let mut spec = ColorSpec::new();
        spec.set_bg(Some(Color::Blue));
        wtr.set_color(&spec).unwrap();
        spec.clear();
        spec.set_reset(false).set_fg(Some(Color::White));
        wtr.set_color(&spec).unwrap();
        wtr.write_all(b"match").unwrap();
        wtr.reset().unwrap();
        assert_eq!(
            wtr.get_ref().get_ref(),
            b"\x1B[0m\x1B[44m\x1B[37mmatch\x1B[0m\x1B[K"
        );
    }

    #[test]
    fn replacing_background_with_foreground_clears() {
        let mut wtr = ColorWriter::new(Ansi::new(vec![]), true, Some(80));
        let mut spec = ColorSpec::new();
        spec.set_bg(Some(Color::Blue));
        wtr.set_color(&spec).unwrap();
        wtr.write_all(b"match").unwrap();
        spec.clear();
        spec.set_fg(Some(Color::White));
        wtr.set_color(&spec).unwrap();
        wtr.write_all(b" tail").unwrap();
        wtr.reset().unwrap();
        assert_eq!(
            wtr.get_ref().get_ref(),
            b"\x1B[0m\x1B[44mmatch\x1B[0m\x1B[37m\x1B[K tail\x1B[0m"
        );
    }

    #[test]
    fn disabled_colors_do_not_clear() {
        let mut wtr = ColorWriter::new(NoColor::new(vec![]), false, Some(80));
        let mut spec = ColorSpec::new();
        spec.set_bg(Some(Color::Blue));
        wtr.set_color(&spec).unwrap();
        wtr.write_all(b"match").unwrap();
        wtr.reset().unwrap();
        assert_eq!(wtr.get_ref().get_ref(), b"match");
    }

    #[test]
    fn native_console_colors_do_not_emit_escapes() {
        // Like termcolor's Windows Buffer, colors are stored separately and
        // is_synchronous() is false. Neither property implies ANSI support.
        #[derive(Default)]
        struct NativeColors(Vec<u8>);

        impl Write for NativeColors {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                self.0.write(buf)
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        impl WriteColor for NativeColors {
            fn supports_color(&self) -> bool {
                true
            }
            fn set_color(&mut self, _: &ColorSpec) -> io::Result<()> {
                Ok(())
            }
            fn reset(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let mut wtr =
            ColorWriter::new(NativeColors::default(), false, Some(80));
        let mut spec = ColorSpec::new();
        spec.set_bg(Some(Color::Blue));
        wtr.set_color(&spec).unwrap();
        wtr.write_all(b"match").unwrap();
        wtr.reset().unwrap();
        assert_eq!(wtr.get_ref().0, b"match");
    }
}
