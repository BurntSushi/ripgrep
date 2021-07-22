use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::process::{CommandError, CommandReader, CommandReaderBuilder};

/// A builder for a matcher that determines which files get decompressed.
#[derive(Clone, Debug)]
pub struct DecompressionMatcherBuilder {
    /// The commands for each matching glob.
    commands: Vec<DecompressionCommand>,
    /// Whether to include the default matching rules.
    defaults: bool,
}

/// A representation of a single command for decompressing data
/// out-of-process.
#[derive(Clone, Debug)]
struct DecompressionCommand {
    /// The glob that matches this command.
    glob: String,
    /// The command or binary name.
    bin: PathBuf,
    /// The arguments to invoke with the command.
    args: Vec<OsString>,
}

impl Default for DecompressionMatcherBuilder {
    fn default() -> DecompressionMatcherBuilder {
        DecompressionMatcherBuilder::new()
    }
}

impl DecompressionMatcherBuilder {
    /// Create a new builder for configuring a decompression matcher.
    pub fn new() -> DecompressionMatcherBuilder {
        DecompressionMatcherBuilder { commands: vec![], defaults: true }
    }

    /// Build a matcher for determining how to decompress files.
    ///
    /// If there was a problem compiling the matcher, then an error is
    /// returned.
    pub fn build(&self) -> Result<DecompressionMatcher, CommandError> {
        let defaults = if !self.defaults {
            vec![]
        } else {
            default_decompression_commands()
        };
        let mut glob_builder = GlobSetBuilder::new();
        let mut commands = vec![];
        for decomp_cmd in defaults.iter().chain(&self.commands) {
            let glob = Glob::new(&decomp_cmd.glob).map_err(|err| {
                CommandError::io(io::Error::new(io::ErrorKind::Other, err))
            })?;
            glob_builder.add(glob);
            commands.push(decomp_cmd.clone());
        }
        let globs = glob_builder.build().map_err(|err| {
            CommandError::io(io::Error::new(io::ErrorKind::Other, err))
        })?;
        Ok(DecompressionMatcher { globs, commands })
    }

    /// When enabled, the default matching rules will be compiled into this
    /// matcher before any other associations. When disabled, only the
    /// rules explicitly given to this builder will be used.
    ///
    /// This is enabled by default.
    pub fn defaults(&mut self, yes: bool) -> &mut DecompressionMatcherBuilder {
        self.defaults = yes;
        self
    }

    /// Associates a glob with a command to decompress files matching the glob.
    ///
    /// If multiple globs match the same file, then the most recently added
    /// glob takes precedence.
    ///
    /// The syntax for the glob is documented in the
    /// [`globset` crate](https://docs.rs/globset/#syntax).
    ///
    /// The `program` given is resolved with respect to `PATH` and turned
    /// into an absolute path internally before being executed by the current
    /// platform. Notably, on Windows, this avoids a security problem where
    /// passing a relative path to `CreateProcess` will automatically search
    /// the current directory for a matching program. If the program could
    /// not be resolved, then it is silently ignored and the association is
    /// dropped. For this reason, callers should prefer `try_associate`.
    pub fn associate<P, I, A>(
        &mut self,
        glob: &str,
        program: P,
        args: I,
    ) -> &mut DecompressionMatcherBuilder
    where
        P: AsRef<OsStr>,
        I: IntoIterator<Item = A>,
        A: AsRef<OsStr>,
    {
        let _ = self.try_associate(glob, program, args);
        self
    }

    /// Associates a glob with a command to decompress files matching the glob.
    ///
    /// If multiple globs match the same file, then the most recently added
    /// glob takes precedence.
    ///
    /// The syntax for the glob is documented in the
    /// [`globset` crate](https://docs.rs/globset/#syntax).
    ///
    /// The `program` given is resolved with respect to `PATH` and turned
    /// into an absolute path internally before being executed by the current
    /// platform. Notably, on Windows, this avoids a security problem where
    /// passing a relative path to `CreateProcess` will automatically search
    /// the current directory for a matching program. If the program could not
    /// be resolved, then an error is returned.
    pub fn try_associate<P, I, A>(
        &mut self,
        glob: &str,
        program: P,
        args: I,
    ) -> Result<&mut DecompressionMatcherBuilder, CommandError>
    where
        P: AsRef<OsStr>,
        I: IntoIterator<Item = A>,
        A: AsRef<OsStr>,
    {
        let glob = glob.to_string();
        let bin = try_resolve_binary(Path::new(program.as_ref()))?;
        let args =
            args.into_iter().map(|a| a.as_ref().to_os_string()).collect();
        self.commands.push(DecompressionCommand { glob, bin, args });
        Ok(self)
    }
}

/// A matcher for determining how to decompress files.
#[derive(Clone, Debug)]
pub struct DecompressionMatcher {
    /// The set of globs to match. Each glob has a corresponding entry in
    /// `commands`. When a glob matches, the corresponding command should be
    /// used to perform out-of-process decompression.
    globs: GlobSet,
    /// The commands for each matching glob.
    commands: Vec<DecompressionCommand>,
}

impl Default for DecompressionMatcher {
    fn default() -> DecompressionMatcher {
        DecompressionMatcher::new()
    }
}

impl DecompressionMatcher {
    /// Create a new matcher with default rules.
    ///
    /// To add more matching rules, build a matcher with
    /// [`DecompressionMatcherBuilder`](struct.DecompressionMatcherBuilder.html).
    pub fn new() -> DecompressionMatcher {
        DecompressionMatcherBuilder::new()
            .build()
            .expect("built-in matching rules should always compile")
    }

    /// Return a pre-built command based on the given file path that can
    /// decompress its contents. If no such decompressor is known, then this
    /// returns `None`.
    ///
    /// If there are multiple possible commands matching the given path, then
    /// the command added last takes precedence.
    pub fn command<P: AsRef<Path>>(&self, path: P) -> Option<Command> {
        for i in self.globs.matches(path).into_iter().rev() {
            let decomp_cmd = &self.commands[i];
            let mut cmd = Command::new(&decomp_cmd.bin);
            cmd.args(&decomp_cmd.args);
            return Some(cmd);
        }
        None
    }

    /// Returns true if and only if the given file path has at least one
    /// matching command to perform decompression on.
    pub fn has_command<P: AsRef<Path>>(&self, path: P) -> bool {
        self.globs.is_match(path)
    }
}

/// Configures and builds a streaming reader for decompressing data.
#[derive(Clone, Debug, Default)]
pub struct DecompressionReaderBuilder {
    matcher: DecompressionMatcher,
    command_builder: CommandReaderBuilder,
}

impl DecompressionReaderBuilder {
    /// Create a new builder with the default configuration.
    pub fn new() -> DecompressionReaderBuilder {
        DecompressionReaderBuilder::default()
    }

    /// Build a new streaming reader for decompressing data.
    ///
    /// If decompression is done out-of-process and if there was a problem
    /// spawning the process, then its error is logged at the debug level and a
    /// passthru reader is returned that does no decompression. This behavior
    /// typically occurs when the given file path matches a decompression
    /// command, but is executing in an environment where the decompression
    /// command is not available.
    ///
    /// If the given file path could not be matched with a decompression
    /// strategy, then a passthru reader is returned that does no
    /// decompression.
    pub fn build<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<DecompressionReader, CommandError> {
        let path = path.as_ref();
        let mut cmd = match self.matcher.command(path) {
            None => return DecompressionReader::new_passthru(path),
            Some(cmd) => cmd,
        };
        cmd.arg(path);

        match self.command_builder.build(&mut cmd) {
            Ok(cmd_reader) => Ok(DecompressionReader { rdr: Ok(cmd_reader) }),
            Err(err) => {
                log::debug!(
                    "{}: error spawning command '{:?}': {} \
                     (falling back to uncompressed reader)",
                    path.display(),
                    cmd,
                    err,
                );
                DecompressionReader::new_passthru(path)
            }
        }
    }

    /// Set the matcher to use to look up the decompression command for each
    /// file path.
    ///
    /// A set of sensible rules is enabled by default. Setting this will
    /// completely replace the current rules.
    pub fn matcher(
        &mut self,
        matcher: DecompressionMatcher,
    ) -> &mut DecompressionReaderBuilder {
        self.matcher = matcher;
        self
    }

    /// Get the underlying matcher currently used by this builder.
    pub fn get_matcher(&self) -> &DecompressionMatcher {
        &self.matcher
    }

    /// When enabled, the reader will asynchronously read the contents of the
    /// command's stderr output. When disabled, stderr is only read after the
    /// stdout stream has been exhausted (or if the process quits with an error
    /// code).
    ///
    /// Note that when enabled, this may require launching an additional
    /// thread in order to read stderr. This is done so that the process being
    /// executed is never blocked from writing to stdout or stderr. If this is
    /// disabled, then it is possible for the process to fill up the stderr
    /// buffer and deadlock.
    ///
    /// This is enabled by default.
    pub fn async_stderr(
        &mut self,
        yes: bool,
    ) -> &mut DecompressionReaderBuilder {
        self.command_builder.async_stderr(yes);
        self
    }
}

/// A streaming reader for decompressing the contents of a file.
///
/// The purpose of this reader is to provide a seamless way to decompress the
/// contents of file using existing tools in the current environment. This is
/// meant to be an alternative to using decompression libraries in favor of the
/// simplicity and portability of using external commands such as `gzip` and
/// `xz`. This does impose the overhead of spawning a process, so other means
/// for performing decompression should be sought if this overhead isn't
/// acceptable.
///
/// A decompression reader comes with a default set of matching rules that are
/// meant to associate file paths with the corresponding command to use to
/// decompress them. For example, a glob like `*.gz` matches gzip compressed
/// files with the command `gzip -d -c`. If a file path does not match any
/// existing rules, or if it matches a rule whose command does not exist in the
/// current environment, then the decompression reader passes through the
/// contents of the underlying file without doing any decompression.
///
/// The default matching rules are probably good enough for most cases, and if
/// they require revision, pull requests are welcome. In cases where they must
/// be changed or extended, they can be customized through the use of
/// [`DecompressionMatcherBuilder`](struct.DecompressionMatcherBuilder.html)
/// and
/// [`DecompressionReaderBuilder`](struct.DecompressionReaderBuilder.html).
///
/// By default, this reader will asynchronously read the processes' stderr.
/// This prevents subtle deadlocking bugs for noisy processes that write a lot
/// to stderr. Currently, the entire contents of stderr is read on to the heap.
///
/// # Example
///
/// This example shows how to read the decompressed contents of a file without
/// needing to explicitly choose the decompression command to run.
///
/// Note that if you need to decompress multiple files, it is better to use
/// `DecompressionReaderBuilder`, which will amortize the cost of compiling the
/// matcher.
///
/// ```no_run
/// use std::io::Read;
/// use std::process::Command;
/// use grep_cli::DecompressionReader;
///
/// # fn example() -> Result<(), Box<::std::error::Error>> {
/// let mut rdr = DecompressionReader::new("/usr/share/man/man1/ls.1.gz")?;
/// let mut contents = vec![];
/// rdr.read_to_end(&mut contents)?;
/// # Ok(()) }
/// ```
#[derive(Debug)]
pub struct DecompressionReader {
    rdr: Result<CommandReader, File>,
}

impl DecompressionReader {
    /// Build a new streaming reader for decompressing data.
    ///
    /// If decompression is done out-of-process and if there was a problem
    /// spawning the process, then its error is returned.
    ///
    /// If the given file path could not be matched with a decompression
    /// strategy, then a passthru reader is returned that does no
    /// decompression.
    ///
    /// This uses the default matching rules for determining how to decompress
    /// the given file. To change those matching rules, use
    /// [`DecompressionReaderBuilder`](struct.DecompressionReaderBuilder.html)
    /// and
    /// [`DecompressionMatcherBuilder`](struct.DecompressionMatcherBuilder.html).
    ///
    /// When creating readers for many paths. it is better to use the builder
    /// since it will amortize the cost of constructing the matcher.
    pub fn new<P: AsRef<Path>>(
        path: P,
    ) -> Result<DecompressionReader, CommandError> {
        DecompressionReaderBuilder::new().build(path)
    }

    /// Creates a new "passthru" decompression reader that reads from the file
    /// corresponding to the given path without doing decompression and without
    /// executing another process.
    fn new_passthru(path: &Path) -> Result<DecompressionReader, CommandError> {
        let file = File::open(path)?;
        Ok(DecompressionReader { rdr: Err(file) })
    }

    /// Closes this reader, freeing any resources used by its underlying child
    /// process, if one was used. If the child process exits with a nonzero
    /// exit code, the returned Err value will include its stderr.
    ///
    /// `close` is idempotent, meaning it can be safely called multiple times.
    /// The first call closes the CommandReader and any subsequent calls do
    /// nothing.
    ///
    /// This method should be called after partially reading a file to prevent
    /// resource leakage. However there is no need to call `close` explicitly
    /// if your code always calls `read` to EOF, as `read` takes care of
    /// calling `close` in this case.
    ///
    /// `close` is also called in `drop` as a last line of defense against
    /// resource leakage. Any error from the child process is then printed as a
    /// warning to stderr. This can be avoided by explicitly calling `close`
    /// before the CommandReader is dropped.
    pub fn close(&mut self) -> io::Result<()> {
        match self.rdr {
            Ok(ref mut rdr) => rdr.close(),
            Err(_) => Ok(()),
        }
    }
}

impl io::Read for DecompressionReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.rdr {
            Ok(ref mut rdr) => rdr.read(buf),
            Err(ref mut rdr) => rdr.read(buf),
        }
    }
}

/// Resolves a path to a program to a path by searching for the program in
/// `PATH`.
///
/// If the program could not be resolved, then an error is returned.
///
/// The purpose of doing this instead of passing the path to the program
/// directly to Command::new is that Command::new will hand relative paths
/// to CreateProcess on Windows, which will implicitly search the current
/// working directory for the executable. This could be undesirable for
/// security reasons. e.g., running ripgrep with the -z/--search-zip flag on an
/// untrusted directory tree could result in arbitrary programs executing on
/// Windows.
///
/// Note that this could still return a relative path if PATH contains a
/// relative path. We permit this since it is assumed that the user has set
/// this explicitly, and thus, desires this behavior.
///
/// On non-Windows, this is a no-op.
pub fn resolve_binary<P: AsRef<Path>>(
    prog: P,
) -> Result<PathBuf, CommandError> {
    if !cfg!(windows) {
        return Ok(prog.as_ref().to_path_buf());
    }
    try_resolve_binary(prog)
}

/// Resolves a path to a program to a path by searching for the program in
/// `PATH`.
///
/// If the program could not be resolved, then an error is returned.
///
/// The purpose of doing this instead of passing the path to the program
/// directly to Command::new is that Command::new will hand relative paths
/// to CreateProcess on Windows, which will implicitly search the current
/// working directory for the executable. This could be undesirable for
/// security reasons. e.g., running ripgrep with the -z/--search-zip flag on an
/// untrusted directory tree could result in arbitrary programs executing on
/// Windows.
///
/// Note that this could still return a relative path if PATH contains a
/// relative path. We permit this since it is assumed that the user has set
/// this explicitly, and thus, desires this behavior.
///
/// If `check_exists` is false or the path is already an absolute path this
/// will return immediately.
fn try_resolve_binary<P: AsRef<Path>>(
    prog: P,
) -> Result<PathBuf, CommandError> {
    use std::env;

    fn is_exe(path: &Path) -> bool {
        let md = match path.metadata() {
            Err(_) => return false,
            Ok(md) => md,
        };
        !md.is_dir()
    }

    let prog = prog.as_ref();
    if prog.is_absolute() {
        return Ok(prog.to_path_buf());
    }
    let syspaths = match env::var_os("PATH") {
        Some(syspaths) => syspaths,
        None => {
            let msg = "system PATH environment variable not found";
            return Err(CommandError::io(io::Error::new(
                io::ErrorKind::Other,
                msg,
            )));
        }
    };
    for syspath in env::split_paths(&syspaths) {
        if syspath.as_os_str().is_empty() {
            continue;
        }
        let abs_prog = syspath.join(prog);
        if is_exe(&abs_prog) {
            return Ok(abs_prog.to_path_buf());
        }
        if abs_prog.extension().is_none() {
            for extension in ["com", "exe"] {
                let abs_prog = abs_prog.with_extension(extension);
                if is_exe(&abs_prog) {
                    return Ok(abs_prog.to_path_buf());
                }
            }
        }
    }
    let msg = format!("{}: could not find executable in PATH", prog.display());
    return Err(CommandError::io(io::Error::new(io::ErrorKind::Other, msg)));
}

fn default_decompression_commands() -> Vec<DecompressionCommand> {
    const ARGS_GZIP: &[&str] = &["gzip", "-d", "-c"];
    const ARGS_BZIP: &[&str] = &["bzip2", "-d", "-c"];
    const ARGS_XZ: &[&str] = &["xz", "-d", "-c"];
    const ARGS_LZ4: &[&str] = &["lz4", "-d", "-c"];
    const ARGS_LZMA: &[&str] = &["xz", "--format=lzma", "-d", "-c"];
    const ARGS_BROTLI: &[&str] = &["brotli", "-d", "-c"];
    const ARGS_ZSTD: &[&str] = &["zstd", "-q", "-d", "-c"];
    const ARGS_UNCOMPRESS: &[&str] = &["uncompress", "-c"];

    fn add(glob: &str, args: &[&str], cmds: &mut Vec<DecompressionCommand>) {
        let bin = match resolve_binary(Path::new(args[0])) {
            Ok(bin) => bin,
            Err(err) => {
                log::debug!("{}", err);
                return;
            }
        };
        cmds.push(DecompressionCommand {
            glob: glob.to_string(),
            bin,
            args: args
                .iter()
                .skip(1)
                .map(|s| OsStr::new(s).to_os_string())
                .collect(),
        });
    }
    let mut cmds = vec![];
    add("*.gz", ARGS_GZIP, &mut cmds);
    add("*.tgz", ARGS_GZIP, &mut cmds);
    add("*.bz2", ARGS_BZIP, &mut cmds);
    add("*.tbz2", ARGS_BZIP, &mut cmds);
    add("*.xz", ARGS_XZ, &mut cmds);
    add("*.txz", ARGS_XZ, &mut cmds);
    add("*.lz4", ARGS_LZ4, &mut cmds);
    add("*.lzma", ARGS_LZMA, &mut cmds);
    add("*.br", ARGS_BROTLI, &mut cmds);
    add("*.zst", ARGS_ZSTD, &mut cmds);
    add("*.zstd", ARGS_ZSTD, &mut cmds);
    add("*.Z", ARGS_UNCOMPRESS, &mut cmds);
    cmds
}
