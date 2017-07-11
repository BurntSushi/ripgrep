extern crate ignore;


use std::path::Path;

use ignore::gitignore::{Gitignore, GitignoreBuilder};


const IGNORE_FILE: &'static str = "tests/gitignore_tests.gitignore";


fn get_gitignore() -> Gitignore {
    let mut builder = GitignoreBuilder::new("ROOT");
    builder.add(IGNORE_FILE);
    builder.build().unwrap()
}


#[test]
fn test_gitignore_files_in_root() {
    let gitignore = get_gitignore();
    let m = |path: &str| gitignore.matched_recursive(Path::new(path), false);

    // 0x
    assert!(m("ROOT/file_root_00").is_ignore());
    assert!(m("ROOT/file_root_01").is_none());
    assert!(m("ROOT/file_root_02").is_none());
    assert!(m("ROOT/file_root_03").is_none());

    // 1x
    assert!(m("ROOT/file_root_10").is_ignore());
    assert!(m("ROOT/file_root_11").is_none());
    assert!(m("ROOT/file_root_12").is_none());
    assert!(m("ROOT/file_root_13").is_none());

    // 2x
    assert!(m("ROOT/file_root_20").is_none());
    assert!(m("ROOT/file_root_21").is_none());
    assert!(m("ROOT/file_root_22").is_none());
    assert!(m("ROOT/file_root_23").is_none());

    // 3x
    assert!(m("ROOT/file_root_30").is_ignore());
    assert!(m("ROOT/file_root_31").is_none());
    assert!(m("ROOT/file_root_32").is_none());
    assert!(m("ROOT/file_root_33").is_none());
}


#[test]
fn test_gitignore_files_in_deep() {
    let gitignore = get_gitignore();
    let m = |path: &str| gitignore.matched_recursive(Path::new(path), false);

    // 0x
    assert!(m("ROOT/parent_dir/file_deep_00").is_ignore());
    assert!(m("ROOT/parent_dir/file_deep_01").is_none());
    assert!(m("ROOT/parent_dir/file_deep_02").is_none());
    assert!(m("ROOT/parent_dir/file_deep_03").is_none());

    // 1x
    assert!(m("ROOT/parent_dir/file_deep_10").is_none());
    assert!(m("ROOT/parent_dir/file_deep_11").is_none());
    assert!(m("ROOT/parent_dir/file_deep_12").is_none());
    assert!(m("ROOT/parent_dir/file_deep_13").is_none());

    // 2x
    assert!(m("ROOT/parent_dir/file_deep_20").is_ignore());
    assert!(m("ROOT/parent_dir/file_deep_21").is_none());
    assert!(m("ROOT/parent_dir/file_deep_22").is_none());
    assert!(m("ROOT/parent_dir/file_deep_23").is_none());

    // 3x
    assert!(m("ROOT/parent_dir/file_deep_30").is_ignore());
    assert!(m("ROOT/parent_dir/file_deep_31").is_none());
    assert!(m("ROOT/parent_dir/file_deep_32").is_none());
    assert!(m("ROOT/parent_dir/file_deep_33").is_none());
}


#[test]
fn test_gitignore_dirs_in_root() {
    let gitignore = get_gitignore();
    let m = |path: &str| gitignore.matched_recursive(Path::new(path), true);

    // 00
    assert!(m("ROOT/dir_root_00").is_ignore());
    assert!(m("ROOT/dir_root_00/file").is_ignore());
    assert!(m("ROOT/dir_root_00/child_dir/file").is_ignore());

    // 01
    assert!(m("ROOT/dir_root_01").is_ignore());
    assert!(m("ROOT/dir_root_01/file").is_ignore());
    assert!(m("ROOT/dir_root_01/child_dir/file").is_ignore());

    // 02
    assert!(m("ROOT/dir_root_02").is_none()); // dir itself doesn't match
    assert!(m("ROOT/dir_root_02/file").is_ignore());
    assert!(m("ROOT/dir_root_02/child_dir/file").is_ignore());

    // 03
    assert!(m("ROOT/dir_root_03").is_none()); // dir itself doesn't match
    assert!(m("ROOT/dir_root_03/file").is_ignore());
    assert!(m("ROOT/dir_root_03/child_dir/file").is_ignore());

    // 10
    assert!(m("ROOT/dir_root_10").is_ignore());
    assert!(m("ROOT/dir_root_10/file").is_ignore());
    assert!(m("ROOT/dir_root_10/child_dir/file").is_ignore());

    // 11
    assert!(m("ROOT/dir_root_11").is_ignore());
    assert!(m("ROOT/dir_root_11/file").is_ignore());
    assert!(m("ROOT/dir_root_11/child_dir/file").is_ignore());

    // 12
    assert!(m("ROOT/dir_root_12").is_none()); // dir itself doesn't match
    assert!(m("ROOT/dir_root_12/file").is_ignore());
    assert!(m("ROOT/dir_root_12/child_dir/file").is_ignore());

    // 13
    assert!(m("ROOT/dir_root_13").is_none());
    assert!(m("ROOT/dir_root_13/file").is_ignore());
    assert!(m("ROOT/dir_root_13/child_dir/file").is_ignore());

    // 20
    assert!(m("ROOT/dir_root_20").is_none());
    assert!(m("ROOT/dir_root_20/file").is_none());
    assert!(m("ROOT/dir_root_20/child_dir/file").is_none());

    // 21
    assert!(m("ROOT/dir_root_21").is_none());
    assert!(m("ROOT/dir_root_21/file").is_none());
    assert!(m("ROOT/dir_root_21/child_dir/file").is_none());

    // 22
    assert!(m("ROOT/dir_root_22").is_none());
    assert!(m("ROOT/dir_root_22/file").is_none());
    assert!(m("ROOT/dir_root_22/child_dir/file").is_none());

    // 23
    assert!(m("ROOT/dir_root_23").is_none());
    assert!(m("ROOT/dir_root_23/file").is_none());
    assert!(m("ROOT/dir_root_23/child_dir/file").is_none());

    // 30
    assert!(m("ROOT/dir_root_30").is_ignore());
    assert!(m("ROOT/dir_root_30/file").is_ignore());
    assert!(m("ROOT/dir_root_30/child_dir/file").is_ignore());

    // 31
    assert!(m("ROOT/dir_root_31").is_ignore());
    assert!(m("ROOT/dir_root_31/file").is_ignore());
    assert!(m("ROOT/dir_root_31/child_dir/file").is_ignore());

    // 32
    assert!(m("ROOT/dir_root_32").is_none()); // dir itself doesn't match
    assert!(m("ROOT/dir_root_32/file").is_ignore());
    assert!(m("ROOT/dir_root_32/child_dir/file").is_ignore());

    // 33
    assert!(m("ROOT/dir_root_33").is_none()); // dir itself doesn't match
    assert!(m("ROOT/dir_root_33/file").is_ignore());
    assert!(m("ROOT/dir_root_33/child_dir/file").is_ignore());
}


#[test]
fn test_gitignore_dirs_in_deep() {
    let gitignore = get_gitignore();
    let m = |path: &str| gitignore.matched_recursive(Path::new(path), true);

    // 00
    assert!(m("ROOT/parent_dir/dir_deep_00").is_ignore());
    assert!(m("ROOT/parent_dir/dir_deep_00/file").is_ignore());
    assert!(m("ROOT/parent_dir/dir_deep_00/child_dir/file").is_ignore());

    // 01
    assert!(m("ROOT/parent_dir/dir_deep_01").is_ignore());
    assert!(m("ROOT/parent_dir/dir_deep_01/file").is_ignore());
    assert!(m("ROOT/parent_dir/dir_deep_01/child_dir/file").is_ignore());

    // 02
    assert!(m("ROOT/parent_dir/dir_deep_02").is_none()); // dir itself doesn't match
    assert!(m("ROOT/parent_dir/dir_deep_02/file").is_ignore());
    assert!(m("ROOT/parent_dir/dir_deep_02/child_dir/file").is_ignore());

    // 03
    assert!(m("ROOT/parent_dir/dir_deep_03").is_none()); // dir itself doesn't match
    assert!(m("ROOT/parent_dir/dir_deep_03/file").is_ignore());
    assert!(m("ROOT/parent_dir/dir_deep_03/child_dir/file").is_ignore());

    // 10
    assert!(m("ROOT/parent_dir/dir_deep_10").is_none());
    assert!(m("ROOT/parent_dir/dir_deep_10/file").is_none());
    assert!(m("ROOT/parent_dir/dir_deep_10/child_dir/file").is_none());

    // 11
    assert!(m("ROOT/parent_dir/dir_deep_11").is_none());
    assert!(m("ROOT/parent_dir/dir_deep_11/file").is_none());
    assert!(m("ROOT/parent_dir/dir_deep_11/child_dir/file").is_none());

    // 12
    assert!(m("ROOT/parent_dir/dir_deep_12").is_none());
    assert!(m("ROOT/parent_dir/dir_deep_12/file").is_none());
    assert!(m("ROOT/parent_dir/dir_deep_12/child_dir/file").is_none());

    // 13
    assert!(m("ROOT/parent_dir/dir_deep_13").is_none());
    assert!(m("ROOT/parent_dir/dir_deep_13/file").is_none());
    assert!(m("ROOT/parent_dir/dir_deep_13/child_dir/file").is_none());

    // 20
    assert!(m("ROOT/parent_dir/dir_deep_20").is_ignore());
    assert!(m("ROOT/parent_dir/dir_deep_20/file").is_ignore());
    assert!(m("ROOT/parent_dir/dir_deep_20/child_dir/file").is_ignore());

    // 21
    assert!(m("ROOT/parent_dir/dir_deep_21").is_ignore());
    assert!(m("ROOT/parent_dir/dir_deep_21/file").is_ignore());
    assert!(m("ROOT/parent_dir/dir_deep_21/child_dir/file").is_ignore());

    // 22
    assert!(m("ROOT/parent_dir/dir_deep_22").is_none()); // dir itself doesn't match
    assert!(m("ROOT/parent_dir/dir_deep_22/file").is_ignore());
    assert!(m("ROOT/parent_dir/dir_deep_22/child_dir/file").is_ignore());

    // 23
    assert!(m("ROOT/parent_dir/dir_deep_23").is_none()); // dir itself doesn't match
    assert!(m("ROOT/parent_dir/dir_deep_23/file").is_ignore());
    assert!(m("ROOT/parent_dir/dir_deep_23/child_dir/file").is_ignore());

    // 30
    assert!(m("ROOT/parent_dir/dir_deep_30").is_ignore());
    assert!(m("ROOT/parent_dir/dir_deep_30/file").is_ignore());
    assert!(m("ROOT/parent_dir/dir_deep_30/child_dir/file").is_ignore());

    // 31
    assert!(m("ROOT/parent_dir/dir_deep_31").is_ignore());
    assert!(m("ROOT/parent_dir/dir_deep_31/file").is_ignore());
    assert!(m("ROOT/parent_dir/dir_deep_31/child_dir/file").is_ignore());

    // 32
    assert!(m("ROOT/parent_dir/dir_deep_32").is_none()); // dir itself doesn't match
    assert!(m("ROOT/parent_dir/dir_deep_32/file").is_ignore());
    assert!(m("ROOT/parent_dir/dir_deep_32/child_dir/file").is_ignore());

    // 33
    assert!(m("ROOT/parent_dir/dir_deep_33").is_none()); // dir itself doesn't match
    assert!(m("ROOT/parent_dir/dir_deep_33/file").is_ignore());
    assert!(m("ROOT/parent_dir/dir_deep_33/child_dir/file").is_ignore());
}
