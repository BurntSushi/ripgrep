use ignore::gitignore::GitignoreBuilder;

/// A directory used as an ignore file is not a regular file and must be
/// reported as an error rather than read.
#[test]
fn gitignore_directory() {
    let mut builder = GitignoreBuilder::new("ROOT");
    let error = builder.add("tests");
    assert!(error.is_some(), "a directory was accepted as a gitignore file");
}

/// A gitignore file that resolves to a non-regular file, such as a symlink
/// to /dev/null, must be reported as an error rather than read. Reading a
/// device or FIFO either grows the buffer without bound or blocks forever.
///
/// Ref: <https://github.com/BurntSushi/ripgrep/issues/3569>
#[cfg(unix)]
#[test]
fn gitignore_symlink_to_device() {
    use std::os::unix::fs::symlink;

    let dir = std::env::temp_dir().join("rg-ignore-not-a-file");
    std::fs::create_dir_all(&dir).unwrap();
    let link = dir.join(".gitignore");
    let _ = std::fs::remove_file(&link);
    symlink("/dev/null", &link).unwrap();

    let mut builder = GitignoreBuilder::new("ROOT");
    let error = builder.add(&link);
    let _ = std::fs::remove_file(&link);
    let _ = std::fs::remove_dir(&dir);
    assert!(error.is_some(), "a device file was accepted as a gitignore file");
}
