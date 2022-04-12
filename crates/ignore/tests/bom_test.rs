use std::path::Path;

use ignore::gitignore::{Gitignore, GitignoreBuilder};

const IGNORE_FILE: &'static str = "tests/bom_test.gitignore";

fn get_gitignore() -> Gitignore {
    let mut builder = GitignoreBuilder::new("ROOT");
    let error = builder.add(IGNORE_FILE);
    assert!(error.is_none(), "failed to open gitignore file");
    builder.build().unwrap()
}

// First entry should be ignored even when the ignore file starts with a BOM. See
// https://github.com/BurntSushi/ripgrep/issues/2177
#[test]
fn test_match_first_entry() {
    let gitignore = get_gitignore();
    let path = "ignoreme";
    assert!(gitignore
        .matched_path_or_any_parents(Path::new(path), true)
        .is_ignore());
}
