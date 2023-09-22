use std::{io, path::Path};

use termcolor::WriteColor;

use crate::{
    color::ColorSpecs,
    hyperlink::{self, HyperlinkConfig},
    util::PrinterPath,
};

/// A configuration for describing how paths should be written.
#[derive(Clone, Debug)]
struct Config {
    colors: ColorSpecs,
    hyperlink: HyperlinkConfig,
    separator: Option<u8>,
    terminator: u8,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            colors: ColorSpecs::default(),
            hyperlink: HyperlinkConfig::default(),
            separator: None,
            terminator: b'\n',
        }
    }
}

/// A builder for a printer that emits file paths.
#[derive(Clone, Debug)]
pub struct PathPrinterBuilder {
    config: Config,
}

impl PathPrinterBuilder {
    /// Return a new path printer builder with a default configuration.
    pub fn new() -> PathPrinterBuilder {
        PathPrinterBuilder { config: Config::default() }
    }

    /// Create a new path printer with the current configuration that writes
    /// paths to the given writer.
    pub fn build<W: WriteColor>(&self, wtr: W) -> PathPrinter<W> {
        let interpolator =
            hyperlink::Interpolator::new(&self.config.hyperlink);
        PathPrinter { config: self.config.clone(), wtr, interpolator }
    }

    /// Set the user color specifications to use for coloring in this printer.
    ///
    /// A [`UserColorSpec`](crate::UserColorSpec) can be constructed from
    /// a string in accordance with the color specification format. See
    /// the `UserColorSpec` type documentation for more details on the
    /// format. A [`ColorSpecs`] can then be generated from zero or more
    /// `UserColorSpec`s.
    ///
    /// Regardless of the color specifications provided here, whether color
    /// is actually used or not is determined by the implementation of
    /// `WriteColor` provided to `build`. For example, if `termcolor::NoColor`
    /// is provided to `build`, then no color will ever be printed regardless
    /// of the color specifications provided here.
    ///
    /// This completely overrides any previous color specifications. This does
    /// not add to any previously provided color specifications on this
    /// builder.
    ///
    /// The default color specifications provide no styling.
    pub fn color_specs(
        &mut self,
        specs: ColorSpecs,
    ) -> &mut PathPrinterBuilder {
        self.config.colors = specs;
        self
    }

    /// Set the configuration to use for hyperlinks output by this printer.
    ///
    /// Regardless of the hyperlink format provided here, whether hyperlinks
    /// are actually used or not is determined by the implementation of
    /// `WriteColor` provided to `build`. For example, if `termcolor::NoColor`
    /// is provided to `build`, then no hyperlinks will ever be printed
    /// regardless of the format provided here.
    ///
    /// This completely overrides any previous hyperlink format.
    ///
    /// The default configuration results in not emitting any hyperlinks.
    pub fn hyperlink(
        &mut self,
        config: HyperlinkConfig,
    ) -> &mut PathPrinterBuilder {
        self.config.hyperlink = config;
        self
    }

    /// Set the path separator used when printing file paths.
    ///
    /// Typically, printing is done by emitting the file path as is. However,
    /// this setting provides the ability to use a different path separator
    /// from what the current environment has configured.
    ///
    /// A typical use for this option is to permit cygwin users on Windows to
    /// set the path separator to `/` instead of using the system default of
    /// `\`.
    ///
    /// This is disabled by default.
    pub fn separator(&mut self, sep: Option<u8>) -> &mut PathPrinterBuilder {
        self.config.separator = sep;
        self
    }

    /// Set the path terminator used.
    ///
    /// The path terminator is a byte that is printed after every file path
    /// emitted by this printer.
    ///
    /// The default path terminator is `\n`.
    pub fn terminator(&mut self, terminator: u8) -> &mut PathPrinterBuilder {
        self.config.terminator = terminator;
        self
    }
}

/// A printer file paths, with optional color and hyperlink support.
///
/// This printer is very similar to [`Summary`](crate::Summary) in that it
/// principally only emits file paths. The main difference is that this printer
/// doesn't actually execute any search via a `Sink` implementation, and instead
/// just provides a way for the caller to print paths.
///
/// A caller could just print the paths themselves, but this printer handles
/// a few details:
///
/// * It can normalize path separators.
/// * It permits configuring the terminator.
/// * It allows setting the color configuration in a way that is consistent
/// with the other printers in this crate.
/// * It allows setting the hyperlink format in a way that is consistent
/// with the other printers in this crate.
#[derive(Debug)]
pub struct PathPrinter<W> {
    config: Config,
    wtr: W,
    interpolator: hyperlink::Interpolator,
}

impl<W: WriteColor> PathPrinter<W> {
    /// Write the given path to the underlying writer.
    pub fn write(&mut self, path: &Path) -> io::Result<()> {
        let ppath = PrinterPath::new(path.as_ref())
            .with_separator(self.config.separator);
        if !self.wtr.supports_color() {
            self.wtr.write_all(ppath.as_bytes())?;
        } else {
            let status = self.start_hyperlink(&ppath)?;
            self.wtr.set_color(self.config.colors.path())?;
            self.wtr.write_all(ppath.as_bytes())?;
            self.wtr.reset()?;
            self.interpolator.finish(status, &mut self.wtr)?;
        }
        self.wtr.write_all(&[self.config.terminator])
    }

    /// Starts a hyperlink span when applicable.
    fn start_hyperlink(
        &mut self,
        path: &PrinterPath,
    ) -> io::Result<hyperlink::InterpolatorStatus> {
        let Some(hyperpath) = path.as_hyperlink() else {
            return Ok(hyperlink::InterpolatorStatus::inactive());
        };
        let values = hyperlink::Values::new(hyperpath);
        self.interpolator.begin(&values, &mut self.wtr)
    }
}
