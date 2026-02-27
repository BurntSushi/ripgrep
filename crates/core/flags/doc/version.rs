/*!
Provides routines for generating version strings.

Version strings can be just the digits, an overall short one-line description
or something more verbose that includes things like CPU target feature support.
*/

use std::fmt::Write;

/// Generates just the numerical part of the version of ripgrep.
///
/// This includes the git revision hash.
pub(crate) fn generate_digits() -> String {
    let semver = option_env!("CARGO_PKG_VERSION").unwrap_or("N/A");
    match option_env!("RIPGREP_BUILD_GIT_HASH") {
        None => semver.to_string(),
        Some(hash) => format!("{semver} (rev {hash})"),
    }
}

/// Generates a short version string of the form `ripgrep x.y.z`.
pub(crate) fn generate_short() -> String {
    let digits = generate_digits();
    format!("ripgrep {digits}")
}

/// Generates a longer multi-line version string.
///
/// This includes not only the version of ripgrep but some other information
/// about its build. For example, SIMD support and PCRE2 support.
pub(crate) fn generate_long() -> String {
    let (compile, runtime) = (compile_cpu_features(), runtime_cpu_features());

    let mut out = String::new();
    writeln!(out, "{}", generate_short()).unwrap();
    writeln!(out).unwrap();
    writeln!(out, "features:{}", features().join(",")).unwrap();
    if !compile.is_empty() {
        writeln!(out, "simd(compile):{}", compile.join(",")).unwrap();
    }
    if !runtime.is_empty() {
        writeln!(out, "simd(runtime):{}", runtime.join(",")).unwrap();
    }
    let (pcre2_version, _) = generate_pcre2();
    writeln!(out, "\n{pcre2_version}").unwrap();
    out
}

/// Generates multi-line version string with PCRE2 information.
///
/// This also returns whether PCRE2 is actually available in this build of
/// ripgrep.
pub(crate) fn generate_pcre2() -> (String, bool) {
    let mut out = String::new();

    #[cfg(feature = "pcre2")]
    {
        use grep::pcre2;

        let (major, minor) = pcre2::version();
        write!(out, "PCRE2 {}.{} is available", major, minor).unwrap();
        if cfg!(target_pointer_width = "64") && pcre2::is_jit_available() {
            writeln!(out, " (JIT is available)").unwrap();
        } else {
            writeln!(out, " (JIT is unavailable)").unwrap();
        }
        (out, true)
    }

    #[cfg(not(feature = "pcre2"))]
    {
        writeln!(out, "PCRE2 is not available in this build of ripgrep.")
            .unwrap();
        (out, false)
    }
}

/// Returns the relevant SIMD features supported by the CPU at runtime.
///
/// This is kind of a dirty violation of abstraction, since it assumes
/// knowledge about what specific SIMD features are being used by various
/// components.
fn runtime_cpu_features() -> Vec<String> {
    #[cfg(target_arch = "x86_64")]
    {
        let mut features = vec![];

        let sse2 = is_x86_feature_detected!("sse2");
        features.push(format!("{sign}SSE2", sign = sign(sse2)));

        let ssse3 = is_x86_feature_detected!("ssse3");
        features.push(format!("{sign}SSSE3", sign = sign(ssse3)));

        let avx2 = is_x86_feature_detected!("avx2");
        features.push(format!("{sign}AVX2", sign = sign(avx2)));

        features
    }
    #[cfg(target_arch = "aarch64")]
    {
        let mut features = vec![];

        // memchr and aho-corasick only use NEON when it is available at
        // compile time. This isn't strictly necessary, but NEON is supposed
        // to be available for all aarch64 targets. If this isn't true, please
        // file an issue at https://github.com/BurntSushi/memchr.
        let neon = cfg!(target_feature = "neon");
        features.push(format!("{sign}NEON", sign = sign(neon)));

        features
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        vec![]
    }
}

/// Returns the SIMD features supported while compiling ripgrep.
///
/// In essence, any features listed here are required to run ripgrep correctly.
///
/// This is kind of a dirty violation of abstraction, since it assumes
/// knowledge about what specific SIMD features are being used by various
/// components.
///
/// An easy way to enable everything available on your current CPU is to
/// compile ripgrep with `RUSTFLAGS="-C target-cpu=native"`. But note that
/// the binary produced by this will not be portable.
fn compile_cpu_features() -> Vec<String> {
    #[cfg(target_arch = "x86_64")]
    {
        let mut features = vec![];

        let sse2 = cfg!(target_feature = "sse2");
        features.push(format!("{sign}SSE2", sign = sign(sse2)));

        let ssse3 = cfg!(target_feature = "ssse3");
        features.push(format!("{sign}SSSE3", sign = sign(ssse3)));

        let avx2 = cfg!(target_feature = "avx2");
        features.push(format!("{sign}AVX2", sign = sign(avx2)));

        features
    }
    #[cfg(target_arch = "aarch64")]
    {
        let mut features = vec![];

        let neon = cfg!(target_feature = "neon");
        features.push(format!("{sign}NEON", sign = sign(neon)));

        features
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        vec![]
    }
}

/// Returns a list of "features" supported (or not) by this build of ripgrpe.
fn features() -> Vec<String> {
    let mut features = vec![];

    let pcre2 = cfg!(feature = "pcre2");
    features.push(format!("{sign}pcre2", sign = sign(pcre2)));

    features
}

/// Returns `+` when `enabled` is `true` and `-` otherwise.
fn sign(enabled: bool) -> &'static str {
    if enabled { "+" } else { "-" }
}
