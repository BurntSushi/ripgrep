/*!
A minimal binary for generating ripgrep's man page and shell completions.

This is useful for cross-compilation scenarios where one needs to generate
documentation for the target platform without being able to run the target
binary on the host.

Build with: cargo build --bin rg-gen --no-default-features
*/

// When the `cli` feature is enabled, `cargo test` will compile this binary
// even though it's only meant to be built without the `cli` feature. The
// `flags` module, when compiled with `cli`, includes `hiargs` which depends
// on these modules. So we include them here to satisfy the compiler, but they
// are never actually used by this binary.
#[cfg(feature = "cli")]
#[macro_use]
mod messages;
#[cfg(feature = "cli")]
mod haystack;
#[cfg(feature = "cli")]
mod logger;
#[cfg(feature = "cli")]
mod search;

mod flags;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!(
            "Usage: {} <man|complete-bash|complete-zsh|complete-fish|complete-powershell>",
            args[0]
        );
        std::process::exit(1);
    }

    let output = match args[1].as_str() {
        "man" => crate::flags::generate_man_page(),
        "complete-bash" => crate::flags::generate_complete_bash(),
        "complete-zsh" => crate::flags::generate_complete_zsh(),
        "complete-fish" => crate::flags::generate_complete_fish(),
        "complete-powershell" => crate::flags::generate_complete_powershell(),
        _ => {
            eprintln!("Unknown mode: {}", args[1]);
            std::process::exit(1);
        }
    };

    println!("{}", output.trim_end());
}
