use std::io;
use std::path::Path;

use grep::printer::{
    ColorSpecs, HyperlinkPattern, HyperlinkSpan, PrinterPath,
};
use termcolor::WriteColor;

/// A configuration for describing how paths should be written.
#[derive(Clone, Debug)]
struct Config {
    colors: ColorSpecs,
    hyperlink_pattern: HyperlinkPattern,
    separator: Option<u8>,
    terminator: u8,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            colors: ColorSpecs::default(),
            hyperlink_pattern: HyperlinkPattern::default(),
            separator: None,
            terminator: b'\n',
        }
    }
}

/// A builder for constructing things to search over.
#[derive(Clone, Debug)]
pub struct PathPrinterBuilder {
    config: Config,
}

impl PathPrinterBuilder {
    /// Return a new subject builder with a default configuration.
    pub fn new() -> PathPrinterBuilder {
        PathPrinterBuilder { config: Config::default() }
    }

    /// Create a new path printer with the current configuration that writes
    /// paths to the given writer.
    pub fn build<W: WriteColor>(&self, wtr: W) -> PathPrinter<W> {
        PathPrinter { config: self.config.clone(), wtr, buf: vec![] }
    }

    /// Set the color specification for this printer.
    ///
    /// Currently, only the `path` component of the given specification is
    /// used.
    pub fn color_specs(
        &mut self,
        specs: ColorSpecs,
    ) -> &mut PathPrinterBuilder {
        self.config.colors = specs;
        self
    }

    /// Set the hyperlink pattern to use for hyperlinks output by this printer.
    ///
    /// Colors need to be enabled for hyperlinks to be output.
    pub fn hyperlink_pattern(
        &mut self,
        pattern: HyperlinkPattern,
    ) -> &mut PathPrinterBuilder {
        self.config.hyperlink_pattern = pattern;
        self
    }

    /// A path separator.
    ///
    /// When provided, the path's default separator will be replaced with
    /// the given separator.
    ///
    /// This is not set by default, and the system's default path separator
    /// will be used.
    pub fn separator(&mut self, sep: Option<u8>) -> &mut PathPrinterBuilder {
        self.config.separator = sep;
        self
    }

    /// A path terminator.
    ///
    /// When printing a path, it will be by terminated by the given byte.
    ///
    /// This is set to `\n` by default.
    pub fn terminator(&mut self, terminator: u8) -> &mut PathPrinterBuilder {
        self.config.terminator = terminator;
        self
    }
}

/// A printer for emitting paths to a writer, with optional color support.
#[derive(Debug)]
pub struct PathPrinter<W> {
    config: Config,
    wtr: W,
    buf: Vec<u8>,
}

impl<W: WriteColor> PathPrinter<W> {
    /// Write the given path to the underlying writer.
    pub fn write_path(&mut self, path: &Path) -> io::Result<()> {
        let ppath = PrinterPath::with_separator(path, self.config.separator);
        if !self.wtr.supports_color() {
            self.wtr.write_all(ppath.as_bytes())?;
        } else {
            let mut hyperlink = self.start_hyperlink_span(&ppath)?;
            self.wtr.set_color(self.config.colors.path())?;
            self.wtr.write_all(ppath.as_bytes())?;
            self.wtr.reset()?;
            hyperlink.end(&mut self.wtr)?;
        }
        self.wtr.write_all(&[self.config.terminator])
    }

    /// Starts a hyperlink span when applicable.
    fn start_hyperlink_span(
        &mut self,
        path: &PrinterPath,
    ) -> io::Result<HyperlinkSpan> {
        if self.wtr.supports_hyperlinks() {
            if let Some(spec) = path.create_hyperlink_spec(
                &self.config.hyperlink_pattern,
                None,
                None,
                &mut self.buf,
            ) {
                return Ok(HyperlinkSpan::start(&mut self.wtr, &spec)?);
            }
        }
        Ok(HyperlinkSpan::default())
    }
}
