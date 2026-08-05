use globset::GlobBuilder;

fn main() {
    let patterns = vec!["hello.*.rld", "hello.*.*.rld", "hello.**.rld"];
    let targets = vec!["hello.b.rld", "hello.w..rld"];

    println!("--- Testing Custom Separator ('.') ---\n");

    for pattern in patterns {
        println!("Pattern: \"{}\"", pattern);

        let glob = GlobBuilder::new(pattern)
            .separator('.')
            .literal_separator(true)
            .build()
            .unwrap()
            .compile_matcher();

        for target in &targets {
            let matches = glob.is_match(target);
            let status = if matches { "MATCH" } else { "FAIL " };
            println!("  [{}] Target: \"{}\"", status, target);
        }
        println!();
    }

    println!("--- Testing Default Behavior (Standard Paths) ---\n");
    let default_glob = GlobBuilder::new("src/*.rs")
        .literal_separator(true)
        .build()
        .unwrap()
        .compile_matcher();

    println!("Pattern: \"src/*.rs\" (Default separator '/')");
    println!(
        "  [{}] Target: \"src/lib.rs\"",
        if default_glob.is_match("src/lib.rs") { "MATCH" } else { "FAIL " }
    );
    println!(
        "  [{}] Target: \"src/path/to.rs\"",
        if default_glob.is_match("src/path/to.rs") {
            "MATCH"
        } else {
            "FAIL "
        }
    );
}
