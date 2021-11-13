use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::process;

use grep::cli;
use grep::printer::{ColorSpecs, StandardBuilder};
use grep::regex::RegexMatcher;
use grep::searcher::{BinaryDetection, SearcherBuilder};
use termcolor::ColorChoice;
use walkdir::WalkDir;

fn main() {
    if let Err(err) = try_main() {
        eprintln!("{}", err);
        process::exit(1);
    }
}

fn try_main() -> Result<(), Box<dyn Error>> {
    let mut args: Vec<OsString> = env::args_os().collect();
    if args.len() < 2 {
        return Err("Usage: simplegrep <pattern> [<path> ...]".into());
    }
    if args.len() == 2 {
        args.push(OsString::from("./"));
    }
    search(cli::pattern_from_os(&args[1])?, &args[2..])
}

fn search(pattern: &str, paths: &[OsString]) -> Result<(), Box<dyn Error>> {
    let matcher = RegexMatcher::new_line_matcher(&pattern)?;
    let mut searcher = SearcherBuilder::new()
        .binary_detection(BinaryDetection::quit(b'\x00'))
        .line_number(false)
        .build();
    let mut printer = StandardBuilder::new()
        .color_specs(ColorSpecs::default_with_color())
        .build(cli::stdout(if cli::is_tty_stdout() {
            ColorChoice::Auto
        } else {
            ColorChoice::Never
        }));

    for path in paths {
        for result in WalkDir::new(path) {
            let dent = match result {
                Ok(dent) => dent,
                Err(err) => {
                    eprintln!("{}", err);
                    continue;
                }
            };
            if !dent.file_type().is_file() {
                continue;
            }
            let result = searcher.search_path(
                &matcher,
                dent.path(),
                printer.sink_with_path(&matcher, dent.path()),
            );
            if let Err(err) = result {
                eprintln!("{}: {}", dent.path().display(), err);
            }
        }
    }
    Ok(())
}
