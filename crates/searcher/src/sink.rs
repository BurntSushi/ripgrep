use std::error;
use std::fmt;
use std::io;

use grep_matcher::LineTerminator;

use crate::lines::LineIter;
use crate::searcher::{ConfigError, Searcher};

/// A trait that describes errors that can be reported by searchers and
/// implementations of `Sink`.
///
/// Unless you have a specialized use case, you probably don't need to
/// implement this trait explicitly. It's likely that using `io::Error` (which
/// implements this trait) for your error type is good enough, largely because
/// most errors that occur during search will likely be an `io::Error`.
pub trait SinkError: Sized {
    /// A constructor for converting any value that satisfies the
    /// `fmt::Display` trait into an error.
    fn error_message<T: fmt::Display>(message: T) -> Self;

    /// A constructor for converting I/O errors that occur while searching into
    /// an error of this type.
    ///
    /// By default, this is implemented via the `error_message` constructor.
    fn error_io(err: io::Error) -> Self {
        Self::error_message(err)
    }

    /// A constructor for converting configuration errors that occur while
    /// building a searcher into an error of this type.
    ///
    /// By default, this is implemented via the `error_message` constructor.
    fn error_config(err: ConfigError) -> Self {
        Self::error_message(err)
    }
}

/// An `io::Error` can be used as an error for `Sink` implementations out of
/// the box.
impl SinkError for io::Error {
    fn error_message<T: fmt::Display>(message: T) -> io::Error {
        io::Error::new(io::ErrorKind::Other, message.to_string())
    }

    fn error_io(err: io::Error) -> io::Error {
        err
    }
}

/// A `Box<std::error::Error>` can be used as an error for `Sink`
/// implementations out of the box.
impl SinkError for Box<dyn error::Error> {
    fn error_message<T: fmt::Display>(message: T) -> Box<dyn error::Error> {
        Box::<dyn error::Error>::from(message.to_string())
    }
}

/// A trait that defines how results from searchers are handled.
///
/// In this crate, a searcher follows the "push" model. What that means is that
/// the searcher drives execution, and pushes results back to the caller. This
/// is in contrast to a "pull" model where the caller drives execution and
/// takes results as they need them. These are also known as "internal" and
/// "external" iteration strategies, respectively.
///
/// For a variety of reasons, including the complexity of the searcher
/// implementation, this crate chooses the "push" or "internal" model of
/// execution. Thus, in order to act on search results, callers must provide
/// an implementation of this trait to a searcher, and the searcher is then
/// responsible for calling the methods on this trait.
///
/// This trait defines several behaviors:
///
/// * What to do when a match is found. Callers must provide this.
/// * What to do when an error occurs. Callers must provide this via the
///   [`SinkError`](trait.SinkError.html) trait. Generally, callers can just
///   use `io::Error` for this, which already implements `SinkError`.
/// * What to do when a contextual line is found. By default, these are
///   ignored.
/// * What to do when a gap between contextual lines has been found. By
///   default, this is ignored.
/// * What to do when a search has started. By default, this does nothing.
/// * What to do when a search has finished successfully. By default, this does
///   nothing.
///
/// Callers must, at minimum, specify the behavior when an error occurs and
/// the behavior when a match occurs. The rest is optional. For each behavior,
/// callers may report an error (say, if writing the result to another
/// location failed) or simply return `false` if they want the search to stop
/// (e.g., when implementing a cap on the number of search results to show).
///
/// When errors are reported (whether in the searcher or in the implementation
/// of `Sink`), then searchers quit immediately without calling `finish`.
///
/// For simpler uses of `Sink`, callers may elect to use one of
/// the more convenient but less flexible implementations in the
/// [`sinks`](sinks/index.html) module.
pub trait Sink {
    /// The type of an error that should be reported by a searcher.
    ///
    /// Errors of this type are not only returned by the methods on this
    /// trait, but the constructors defined in `SinkError` are also used in
    /// the searcher implementation itself. e.g., When a I/O error occurs when
    /// reading data from a file.
    type Error: SinkError;

    /// This method is called whenever a match is found.
    ///
    /// If multi line is enabled on the searcher, then the match reported here
    /// may span multiple lines and it may include multiple matches. When multi
    /// line is disabled, then the match is guaranteed to span exactly one
    /// non-empty line (where a single line is, at minimum, a line terminator).
    ///
    /// If this returns `true`, then searching continues. If this returns
    /// `false`, then searching is stopped immediately and `finish` is called.
    ///
    /// If this returns an error, then searching is stopped immediately,
    /// `finish` is not called and the error is bubbled back up to the caller
    /// of the searcher.
    fn matched(
        &mut self,
        _searcher: &Searcher,
        _mat: &SinkMatch<'_>,
    ) -> Result<bool, Self::Error>;

    /// This method is called whenever a context line is found, and is optional
    /// to implement. By default, it does nothing and returns `true`.
    ///
    /// In all cases, the context given is guaranteed to span exactly one
    /// non-empty line (where a single line is, at minimum, a line terminator).
    ///
    /// If this returns `true`, then searching continues. If this returns
    /// `false`, then searching is stopped immediately and `finish` is called.
    ///
    /// If this returns an error, then searching is stopped immediately,
    /// `finish` is not called and the error is bubbled back up to the caller
    /// of the searcher.
    #[inline]
    fn context(
        &mut self,
        _searcher: &Searcher,
        _context: &SinkContext<'_>,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }

    /// This method is called whenever a break in contextual lines is found,
    /// and is optional to implement. By default, it does nothing and returns
    /// `true`.
    ///
    /// A break can only occur when context reporting is enabled (that is,
    /// either or both of `before_context` or `after_context` are greater than
    /// `0`). More precisely, a break occurs between non-contiguous groups of
    /// lines.
    ///
    /// If this returns `true`, then searching continues. If this returns
    /// `false`, then searching is stopped immediately and `finish` is called.
    ///
    /// If this returns an error, then searching is stopped immediately,
    /// `finish` is not called and the error is bubbled back up to the caller
    /// of the searcher.
    #[inline]
    fn context_break(
        &mut self,
        _searcher: &Searcher,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }

    /// This method is called whenever binary detection is enabled and binary
    /// data is found. If binary data is found, then this is called at least
    /// once for the first occurrence with the absolute byte offset at which
    /// the binary data begins.
    ///
    /// If this returns `true`, then searching continues. If this returns
    /// `false`, then searching is stopped immediately and `finish` is called.
    ///
    /// If this returns an error, then searching is stopped immediately,
    /// `finish` is not called and the error is bubbled back up to the caller
    /// of the searcher.
    ///
    /// By default, it does nothing and returns `true`.
    #[inline]
    fn binary_data(
        &mut self,
        _searcher: &Searcher,
        _binary_byte_offset: u64,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }

    /// This method is called when a search has begun, before any search is
    /// executed. By default, this does nothing.
    ///
    /// If this returns `true`, then searching continues. If this returns
    /// `false`, then searching is stopped immediately and `finish` is called.
    ///
    /// If this returns an error, then searching is stopped immediately,
    /// `finish` is not called and the error is bubbled back up to the caller
    /// of the searcher.
    #[inline]
    fn begin(&mut self, _searcher: &Searcher) -> Result<bool, Self::Error> {
        Ok(true)
    }

    /// This method is called when a search has completed. By default, this
    /// does nothing.
    ///
    /// If this returns an error, the error is bubbled back up to the caller of
    /// the searcher.
    #[inline]
    fn finish(
        &mut self,
        _searcher: &Searcher,
        _: &SinkFinish,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<'a, S: Sink> Sink for &'a mut S {
    type Error = S::Error;

    #[inline]
    fn matched(
        &mut self,
        searcher: &Searcher,
        mat: &SinkMatch<'_>,
    ) -> Result<bool, S::Error> {
        (**self).matched(searcher, mat)
    }

    #[inline]
    fn context(
        &mut self,
        searcher: &Searcher,
        context: &SinkContext<'_>,
    ) -> Result<bool, S::Error> {
        (**self).context(searcher, context)
    }

    #[inline]
    fn context_break(
        &mut self,
        searcher: &Searcher,
    ) -> Result<bool, S::Error> {
        (**self).context_break(searcher)
    }

    #[inline]
    fn binary_data(
        &mut self,
        searcher: &Searcher,
        binary_byte_offset: u64,
    ) -> Result<bool, S::Error> {
        (**self).binary_data(searcher, binary_byte_offset)
    }

    #[inline]
    fn begin(&mut self, searcher: &Searcher) -> Result<bool, S::Error> {
        (**self).begin(searcher)
    }

    #[inline]
    fn finish(
        &mut self,
        searcher: &Searcher,
        sink_finish: &SinkFinish,
    ) -> Result<(), S::Error> {
        (**self).finish(searcher, sink_finish)
    }
}

impl<S: Sink + ?Sized> Sink for Box<S> {
    type Error = S::Error;

    #[inline]
    fn matched(
        &mut self,
        searcher: &Searcher,
        mat: &SinkMatch<'_>,
    ) -> Result<bool, S::Error> {
        (**self).matched(searcher, mat)
    }

    #[inline]
    fn context(
        &mut self,
        searcher: &Searcher,
        context: &SinkContext<'_>,
    ) -> Result<bool, S::Error> {
        (**self).context(searcher, context)
    }

    #[inline]
    fn context_break(
        &mut self,
        searcher: &Searcher,
    ) -> Result<bool, S::Error> {
        (**self).context_break(searcher)
    }

    #[inline]
    fn binary_data(
        &mut self,
        searcher: &Searcher,
        binary_byte_offset: u64,
    ) -> Result<bool, S::Error> {
        (**self).binary_data(searcher, binary_byte_offset)
    }

    #[inline]
    fn begin(&mut self, searcher: &Searcher) -> Result<bool, S::Error> {
        (**self).begin(searcher)
    }

    #[inline]
    fn finish(
        &mut self,
        searcher: &Searcher,
        sink_finish: &SinkFinish,
    ) -> Result<(), S::Error> {
        (**self).finish(searcher, sink_finish)
    }
}

/// Summary data reported at the end of a search.
///
/// This reports data such as the total number of bytes searched and the
/// absolute offset of the first occurrence of binary data, if any were found.
///
/// A searcher that stops early because of an error does not call `finish`.
/// A searcher that stops early because the `Sink` implementor instructed it
/// to will still call `finish`.
#[derive(Clone, Debug)]
pub struct SinkFinish {
    pub(crate) byte_count: u64,
    pub(crate) binary_byte_offset: Option<u64>,
}

impl SinkFinish {
    /// Return the total number of bytes searched.
    #[inline]
    pub fn byte_count(&self) -> u64 {
        self.byte_count
    }

    /// If binary detection is enabled and if binary data was found, then this
    /// returns the absolute byte offset of the first detected byte of binary
    /// data.
    ///
    /// Note that since this is an absolute byte offset, it cannot be relied
    /// upon to index into any addressable memory.
    #[inline]
    pub fn binary_byte_offset(&self) -> Option<u64> {
        self.binary_byte_offset
    }
}

/// A type that describes a match reported by a searcher.
#[derive(Clone, Debug)]
pub struct SinkMatch<'b> {
    pub(crate) line_term: LineTerminator,
    pub(crate) bytes: &'b [u8],
    pub(crate) absolute_byte_offset: u64,
    pub(crate) line_number: Option<u64>,
    pub(crate) buffer: &'b [u8],
    pub(crate) bytes_range_in_buffer: std::ops::Range<usize>,
}

impl<'b> SinkMatch<'b> {
    /// Returns the bytes for all matching lines, including the line
    /// terminators, if they exist.
    #[inline]
    pub fn bytes(&self) -> &'b [u8] {
        self.bytes
    }

    /// Return an iterator over the lines in this match.
    ///
    /// If multi line search is enabled, then this may yield more than one
    /// line (but always at least one line). If multi line search is disabled,
    /// then this always reports exactly one line (but may consist of just
    /// the line terminator).
    ///
    /// Lines yielded by this iterator include their terminators.
    #[inline]
    pub fn lines(&self) -> LineIter<'b> {
        LineIter::new(self.line_term.as_byte(), self.bytes)
    }

    /// Returns the absolute byte offset of the start of this match. This
    /// offset is absolute in that it is relative to the very beginning of the
    /// input in a search, and can never be relied upon to be a valid index
    /// into an in-memory slice.
    #[inline]
    pub fn absolute_byte_offset(&self) -> u64 {
        self.absolute_byte_offset
    }

    /// Returns the line number of the first line in this match, if available.
    ///
    /// Line numbers are only available when the search builder is instructed
    /// to compute them.
    #[inline]
    pub fn line_number(&self) -> Option<u64> {
        self.line_number
    }

    /// TODO
    #[inline]
    pub fn buffer(&self) -> &'b [u8] {
        self.buffer
    }

    /// TODO
    #[inline]
    pub fn bytes_range_in_buffer(&self) -> std::ops::Range<usize> {
        self.bytes_range_in_buffer.clone()
    }
}

/// The type of context reported by a searcher.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SinkContextKind {
    /// The line reported occurred before a match.
    Before,
    /// The line reported occurred after a match.
    After,
    /// Any other type of context reported, e.g., as a result of a searcher's
    /// "passthru" mode.
    Other,
}

/// A type that describes a contextual line reported by a searcher.
#[derive(Clone, Debug)]
pub struct SinkContext<'b> {
    #[cfg(test)]
    pub(crate) line_term: LineTerminator,
    pub(crate) bytes: &'b [u8],
    pub(crate) kind: SinkContextKind,
    pub(crate) absolute_byte_offset: u64,
    pub(crate) line_number: Option<u64>,
}

impl<'b> SinkContext<'b> {
    /// Returns the context bytes, including line terminators.
    #[inline]
    pub fn bytes(&self) -> &'b [u8] {
        self.bytes
    }

    /// Returns the type of context.
    #[inline]
    pub fn kind(&self) -> &SinkContextKind {
        &self.kind
    }

    /// Return an iterator over the lines in this match.
    ///
    /// This always yields exactly one line (and that one line may contain just
    /// the line terminator).
    ///
    /// Lines yielded by this iterator include their terminators.
    #[cfg(test)]
    pub(crate) fn lines(&self) -> LineIter<'b> {
        LineIter::new(self.line_term.as_byte(), self.bytes)
    }

    /// Returns the absolute byte offset of the start of this context. This
    /// offset is absolute in that it is relative to the very beginning of the
    /// input in a search, and can never be relied upon to be a valid index
    /// into an in-memory slice.
    #[inline]
    pub fn absolute_byte_offset(&self) -> u64 {
        self.absolute_byte_offset
    }

    /// Returns the line number of the first line in this context, if
    /// available.
    ///
    /// Line numbers are only available when the search builder is instructed
    /// to compute them.
    #[inline]
    pub fn line_number(&self) -> Option<u64> {
        self.line_number
    }
}

/// A collection of convenience implementations of `Sink`.
///
/// Each implementation in this module makes some kind of sacrifice in the name
/// of making common cases easier to use. Most frequently, each type is a
/// wrapper around a closure specified by the caller that provides limited
/// access to the full suite of information available to implementors of
/// `Sink`.
///
/// For example, the `UTF8` sink makes the following sacrifices:
///
/// * All matches must be UTF-8. An arbitrary `Sink` does not have this
///   restriction and can deal with arbitrary data. If this sink sees invalid
///   UTF-8, then an error is returned and searching stops. (Use the `Lossy`
///   sink instead to suppress this error.)
/// * The searcher must be configured to report line numbers. If it isn't,
///   an error is reported at the first match and searching stops.
/// * Context lines, context breaks and summary data reported at the end of
///   a search are all ignored.
/// * Implementors are forced to use `io::Error` as their error type.
///
/// If you need more flexibility, then you're advised to implement the `Sink`
/// trait directly.
pub mod sinks {
    use std::io;
    use std::str;

    use super::{Sink, SinkError, SinkMatch};
    use crate::searcher::Searcher;

    /// A sink that provides line numbers and matches as strings while ignoring
    /// everything else.
    ///
    /// This implementation will return an error if a match contains invalid
    /// UTF-8 or if the searcher was not configured to count lines. Errors
    /// on invalid UTF-8 can be suppressed by using the `Lossy` sink instead
    /// of this one.
    ///
    /// The closure accepts two parameters: a line number and a UTF-8 string
    /// containing the matched data. The closure returns a
    /// `Result<bool, io::Error>`. If the `bool` is `false`, then the search
    /// stops immediately. Otherwise, searching continues.
    ///
    /// If multi line mode was enabled, the line number refers to the line
    /// number of the first line in the match.
    #[derive(Clone, Debug)]
    pub struct UTF8<F>(pub F)
    where
        F: FnMut(u64, &str) -> Result<bool, io::Error>;

    impl<F> Sink for UTF8<F>
    where
        F: FnMut(u64, &str) -> Result<bool, io::Error>,
    {
        type Error = io::Error;

        fn matched(
            &mut self,
            _searcher: &Searcher,
            mat: &SinkMatch<'_>,
        ) -> Result<bool, io::Error> {
            let matched = match str::from_utf8(mat.bytes()) {
                Ok(matched) => matched,
                Err(err) => return Err(io::Error::error_message(err)),
            };
            let line_number = match mat.line_number() {
                Some(line_number) => line_number,
                None => {
                    let msg = "line numbers not enabled";
                    return Err(io::Error::error_message(msg));
                }
            };
            (self.0)(line_number, &matched)
        }
    }

    /// A sink that provides line numbers and matches as (lossily converted)
    /// strings while ignoring everything else.
    ///
    /// This is like `UTF8`, except that if a match contains invalid UTF-8,
    /// then it will be lossily converted to valid UTF-8 by substituting
    /// invalid UTF-8 with Unicode replacement characters.
    ///
    /// This implementation will return an error on the first match if the
    /// searcher was not configured to count lines.
    ///
    /// The closure accepts two parameters: a line number and a UTF-8 string
    /// containing the matched data. The closure returns a
    /// `Result<bool, io::Error>`. If the `bool` is `false`, then the search
    /// stops immediately. Otherwise, searching continues.
    ///
    /// If multi line mode was enabled, the line number refers to the line
    /// number of the first line in the match.
    #[derive(Clone, Debug)]
    pub struct Lossy<F>(pub F)
    where
        F: FnMut(u64, &str) -> Result<bool, io::Error>;

    impl<F> Sink for Lossy<F>
    where
        F: FnMut(u64, &str) -> Result<bool, io::Error>,
    {
        type Error = io::Error;

        fn matched(
            &mut self,
            _searcher: &Searcher,
            mat: &SinkMatch<'_>,
        ) -> Result<bool, io::Error> {
            use std::borrow::Cow;

            let matched = match str::from_utf8(mat.bytes()) {
                Ok(matched) => Cow::Borrowed(matched),
                // TODO: In theory, it should be possible to amortize
                // allocation here, but `std` doesn't provide such an API.
                // Regardless, this only happens on matches with invalid UTF-8,
                // which should be pretty rare.
                Err(_) => String::from_utf8_lossy(mat.bytes()),
            };
            let line_number = match mat.line_number() {
                Some(line_number) => line_number,
                None => {
                    let msg = "line numbers not enabled";
                    return Err(io::Error::error_message(msg));
                }
            };
            (self.0)(line_number, &matched)
        }
    }

    /// A sink that provides line numbers and matches as raw bytes while
    /// ignoring everything else.
    ///
    /// This implementation will return an error on the first match if the
    /// searcher was not configured to count lines.
    ///
    /// The closure accepts two parameters: a line number and a raw byte string
    /// containing the matched data. The closure returns a `Result<bool,
    /// io::Error>`. If the `bool` is `false`, then the search stops
    /// immediately. Otherwise, searching continues.
    ///
    /// If multi line mode was enabled, the line number refers to the line
    /// number of the first line in the match.
    #[derive(Clone, Debug)]
    pub struct Bytes<F>(pub F)
    where
        F: FnMut(u64, &[u8]) -> Result<bool, io::Error>;

    impl<F> Sink for Bytes<F>
    where
        F: FnMut(u64, &[u8]) -> Result<bool, io::Error>,
    {
        type Error = io::Error;

        fn matched(
            &mut self,
            _searcher: &Searcher,
            mat: &SinkMatch<'_>,
        ) -> Result<bool, io::Error> {
            let line_number = match mat.line_number() {
                Some(line_number) => line_number,
                None => {
                    let msg = "line numbers not enabled";
                    return Err(io::Error::error_message(msg));
                }
            };
            (self.0)(line_number, mat.bytes())
        }
    }
}
