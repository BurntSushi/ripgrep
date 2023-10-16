fn main() {
    set_git_revision_hash();
    set_windows_exe_options();
}

/// Embed a Windows manifest and set some linker options.
///
/// The main reason for this is to enable long path support on Windows. This
/// still, I believe, requires enabling long path support in the registry. But
/// if that's enabled, then this will let ripgrep use C:\... style paths that
/// are longer than 260 characters.
fn set_windows_exe_options() {
    static MANIFEST: &str = "pkg/windows/Manifest.xml";

    let Ok(target_os) = std::env::var("CARGO_CFG_TARGET_OS") else { return };
    let Ok(target_env) = std::env::var("CARGO_CFG_TARGET_ENV") else { return };
    if !(target_os == "windows" && target_env == "msvc") {
        return;
    }

    let Ok(mut manifest) = std::env::current_dir() else { return };
    manifest.push(MANIFEST);
    let Some(manifest) = manifest.to_str() else { return };

    println!("cargo:rerun-if-changed={}", MANIFEST);
    // Embed the Windows application manifest file.
    println!("cargo:rustc-link-arg-bin=rg=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg-bin=rg=/MANIFESTINPUT:{manifest}");
    // Turn linker warnings into errors. Helps debugging, otherwise the
    // warnings get squashed (I believe).
    println!("cargo:rustc-link-arg-bin=rg=/WX");
}

/// Make the current git hash available to the build as the environment
/// variable `RIPGREP_BUILD_GIT_HASH`.
fn set_git_revision_hash() {
    use std::process::Command;

    let args = &["rev-parse", "--short=10", "HEAD"];
    let Ok(output) = Command::new("git").args(args).output() else { return };
    let rev = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if rev.is_empty() {
        return;
    }
    println!("cargo:rustc-env=RIPGREP_BUILD_GIT_HASH={}", rev);
}
