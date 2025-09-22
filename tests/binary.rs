use crate::util::{Dir, TestCommand};

// This file contains a smattering of tests specifically for checking ripgrep's
// handling of binary files. There's quite a bit of discussion on this in this
// bug report: https://github.com/BurntSushi/ripgrep/issues/306

// Our haystack is the first 2,133 lines of Gutenberg's copy of "A Study in
// Scarlet," with a NUL byte at line 1870: `abcdef\x00`.
//
// The position and size of the haystack is, unfortunately, significant. In
// particular, the NUL byte is specifically inserted at some point *after* the
// first 65,536 bytes, which corresponds to the initial capacity of the buffer
// that ripgrep uses to read files. (grep for DEFAULT_BUFFER_CAPACITY.) The
// position of the NUL byte ensures that we can execute some search on the
// initial buffer contents without ever detecting any binary data. Moreover,
// when using a memory map for searching, only the first 65,536 bytes are
// scanned for a NUL byte, so no binary bytes are detected at all when using
// a memory map (unless our query matches line 1898).
//
// One last note: in the tests below, we use --no-mmap heavily because binary
// detection with memory maps is a bit different. Namely, NUL bytes are only
// searched for in the first few KB of the file and in a match. Normally, NUL
// bytes are searched for everywhere.
const HAY: &'static [u8] = include_bytes!("./data/sherlock-nul.txt");

// Tests for binary file detection when using memory maps.
// As noted in the original comments, with memory maps binary detection
// works differently - NUL bytes are only searched for in the first few KB
// of the file and in matches.
//
// Note that we don't run these on macOS, which has memory maps forcefully
// disabled because they suck so much.

// Test that matches in a binary file with memory maps work as expected
// with implicit file search (via glob pattern).
#[cfg(not(target_os = "macos"))]
rgtest!(mmap_match_implicit, |dir: Dir, mut cmd: TestCommand| {
    dir.create_bytes("hay", HAY);
    cmd.args(&["--mmap", "-n", "Project Gutenberg EBook", "-g", "hay"]);

    // With mmap, we get a match and a warning about binary content
    let expected = "\
hay:1:The Project Gutenberg EBook of A Study In Scarlet, by Arthur Conan Doyle
";
    eqnice!(expected, cmd.stdout());
});

// Test with an explicit file argument when using memory maps.
#[cfg(not(target_os = "macos"))]
rgtest!(mmap_match_explicit, |dir: Dir, mut cmd: TestCommand| {
    dir.create_bytes("hay", HAY);
    cmd.args(&["--mmap", "-n", "Project Gutenberg EBook", "hay"]);

    let expected = "\
1:The Project Gutenberg EBook of A Study In Scarlet, by Arthur Conan Doyle
";
    eqnice!(expected, cmd.stdout());
});

// Test specifically with a pattern that matches near the NUL byte which should
// trigger binary detection with memory maps.
#[cfg(not(target_os = "macos"))]
rgtest!(mmap_match_near_nul, |dir: Dir, mut cmd: TestCommand| {
    dir.create_bytes("hay", HAY);
    // Pattern that matches around line 1898 where the NUL byte is.
    // Note: Using direct file path instead of glob.
    cmd.args(&["--mmap", "-n", "abcdef", "hay"]);

    let expected = "\
binary file matches (found \"\\0\" byte around offset 77041)
";
    eqnice!(expected, cmd.stdout());
});

// Test with --count option to ensure full file scanning works with mmap.
#[cfg(not(target_os = "macos"))]
rgtest!(mmap_match_count, |dir: Dir, mut cmd: TestCommand| {
    dir.create_bytes("hay", HAY);
    cmd.args(&["--mmap", "-c", "Project Gutenberg EBook|Heaven", "hay"]);

    // With mmap, since we're counting all matches and might not
    // encounter the NUL byte during initial detection, the count
    // should still be reported.
    eqnice!("2\n", cmd.stdout());
});

// Test binary detection with mmap when pattern would match before and after NUL
// byte.
#[cfg(not(target_os = "macos"))]
rgtest!(mmap_match_multiple, |dir: Dir, mut cmd: TestCommand| {
    dir.create_bytes("hay", HAY);
    // Use explicit file path.
    cmd.args(&["--mmap", "-n", "Project Gutenberg EBook|Heaven", "hay"]);

    // With explicit file and memory maps, matches before and after NUL byte
    // are shown.
    let expected = "\
1:The Project Gutenberg EBook of A Study In Scarlet, by Arthur Conan Doyle
1871:\"No. Heaven knows what the objects of his studies are. But here we
";
    eqnice!(expected, cmd.stdout());
});

// Test that --binary flag can have odd results when searching with a memory
// map.
#[cfg(not(target_os = "macos"))]
rgtest!(mmap_binary_flag, |dir: Dir, mut cmd: TestCommand| {
    dir.create_bytes("hay", HAY);
    // Use glob pattern.
    cmd.args(&["--mmap", "-n", "--binary", "Heaven", "-g", "hay"]);

    let expected = "\
hay:1871:\"No. Heaven knows what the objects of his studies are. But here we
";
    eqnice!(expected, cmd.stdout());
});

// Test that using -a/--text flag works as expected with mmap.
#[cfg(not(target_os = "macos"))]
rgtest!(mmap_text_flag, |dir: Dir, mut cmd: TestCommand| {
    dir.create_bytes("hay", HAY);
    cmd.args(&["--mmap", "-n", "--text", "Heaven", "-g", "hay"]);

    // With --text flag, binary detection should be disabled.
    let expected = "\
hay:1871:\"No. Heaven knows what the objects of his studies are. But here we
";
    eqnice!(expected, cmd.stdout());
});

// Test pattern that matches before and after the NUL byte with memory maps.
#[cfg(not(target_os = "macos"))]
rgtest!(mmap_after_nul_match, |dir: Dir, mut cmd: TestCommand| {
    dir.create_bytes("hay", HAY);
    // Use explicit file path.
    cmd.args(&["--mmap", "-n", "medical student", "hay"]);

    // With explicit file and memory maps, all matches are shown
    let expected = "\
176:\"A medical student, I suppose?\" said I.
409:\"A medical student, I suppose?\" said I.
642:\"A medical student, I suppose?\" said I.
875:\"A medical student, I suppose?\" said I.
1108:\"A medical student, I suppose?\" said I.
1341:\"A medical student, I suppose?\" said I.
1574:\"A medical student, I suppose?\" said I.
1807:\"A medical student, I suppose?\" said I.
1867:\"And yet you say he is not a medical student?\"
";
    eqnice!(expected, cmd.stdout());
});

// This tests that ripgrep prints a warning message if it finds and prints a
// match in a binary file before detecting that it is a binary file. The point
// here is to notify that user that the search of the file is only partially
// complete.
//
// This applies to files that are *implicitly* searched via a recursive
// directory traversal. In particular, this results in a WARNING message being
// printed. We make our file "implicit" by doing a recursive search with a glob
// that matches our file.
rgtest!(after_match1_implicit, |dir: Dir, mut cmd: TestCommand| {
    dir.create_bytes("hay", HAY);
    cmd.args(&["--no-mmap", "-n", "Project Gutenberg EBook", "-g", "hay"]);

    let expected = "\
hay:1:The Project Gutenberg EBook of A Study In Scarlet, by Arthur Conan Doyle
hay: WARNING: stopped searching binary file after match (found \"\\0\" byte around offset 77041)
";
    eqnice!(expected, cmd.stdout());
});

// Like after_match1_implicit, except we provide a file to search
// explicitly. This results in identical behavior, but a different message.
rgtest!(after_match1_explicit, |dir: Dir, mut cmd: TestCommand| {
    dir.create_bytes("hay", HAY);
    cmd.args(&["--no-mmap", "-n", "Project Gutenberg EBook", "hay"]);

    let expected = "\
1:The Project Gutenberg EBook of A Study In Scarlet, by Arthur Conan Doyle
binary file matches (found \"\\0\" byte around offset 77041)
";
    eqnice!(expected, cmd.stdout());
});

// Like after_match1_explicit, except we feed our content on stdin.
rgtest!(after_match1_stdin, |_: Dir, mut cmd: TestCommand| {
    cmd.args(&["--no-mmap", "-n", "Project Gutenberg EBook"]);

    let expected = "\
1:The Project Gutenberg EBook of A Study In Scarlet, by Arthur Conan Doyle
binary file matches (found \"\\0\" byte around offset 77041)
";
    eqnice!(expected, cmd.pipe(HAY));
});

// Like after_match1_implicit, but provides the --binary flag, which
// disables binary filtering. Thus, this matches the behavior of ripgrep as
// if the file were given explicitly.
rgtest!(after_match1_implicit_binary, |dir: Dir, mut cmd: TestCommand| {
    dir.create_bytes("hay", HAY);
    cmd.args(&[
        "--no-mmap",
        "-n",
        "--binary",
        "Project Gutenberg EBook",
        "-g",
        "hay",
    ]);

    let expected = "\
hay:1:The Project Gutenberg EBook of A Study In Scarlet, by Arthur Conan Doyle
hay: binary file matches (found \"\\0\" byte around offset 77041)
";
    eqnice!(expected, cmd.stdout());
});

// Like after_match1_implicit, but enables -a/--text, so no binary
// detection should be performed.
rgtest!(after_match1_implicit_text, |dir: Dir, mut cmd: TestCommand| {
    dir.create_bytes("hay", HAY);
    cmd.args(&[
        "--no-mmap",
        "-n",
        "--text",
        "Project Gutenberg EBook",
        "-g",
        "hay",
    ]);

    let expected = "\
hay:1:The Project Gutenberg EBook of A Study In Scarlet, by Arthur Conan Doyle
";
    eqnice!(expected, cmd.stdout());
});

// Like after_match1_implicit_text, but enables -a/--text, so no binary
// detection should be performed.
rgtest!(after_match1_explicit_text, |dir: Dir, mut cmd: TestCommand| {
    dir.create_bytes("hay", HAY);
    cmd.args(&["--no-mmap", "-n", "--text", "Project Gutenberg EBook", "hay"]);

    let expected = "\
1:The Project Gutenberg EBook of A Study In Scarlet, by Arthur Conan Doyle
";
    eqnice!(expected, cmd.stdout());
});

// Like after_match1_implicit, except this asks ripgrep to print all matching
// files.
//
// This is an interesting corner case that one might consider a bug, however,
// it's unlikely to be fixed. Namely, ripgrep probably shouldn't print `hay`
// as a matching file since it is in fact a binary file, and thus should be
// filtered out by default. However, the --files-with-matches flag will print
// out the path of a matching file as soon as a match is seen and then stop
// searching completely. Therefore, the NUL byte is never actually detected.
//
// The only way to fix this would be to kill ripgrep's performance in this case
// and continue searching the entire file for a NUL byte. (Similarly if the
// --quiet flag is set. See the next test.)
rgtest!(after_match1_implicit_path, |dir: Dir, mut cmd: TestCommand| {
    dir.create_bytes("hay", HAY);
    cmd.args(&["--no-mmap", "-l", "Project Gutenberg EBook", "-g", "hay"]);
    eqnice!("hay\n", cmd.stdout());
});

// Like after_match1_implicit_path, except this indicates that a match was
// found with no other output. (This is the same bug described above, but
// manifest as an exit code with no output.)
rgtest!(after_match1_implicit_quiet, |dir: Dir, mut cmd: TestCommand| {
    dir.create_bytes("hay", HAY);
    cmd.args(&["--no-mmap", "-q", "Project Gutenberg EBook", "-g", "hay"]);
    eqnice!("", cmd.stdout());
});

// This sets up the same test as after_match1_implicit_path, but instead of
// just printing the matching files, this includes the full count of matches.
// In this case, we need to search the entire file, so ripgrep correctly
// detects the binary data and suppresses output.
rgtest!(after_match1_implicit_count, |dir: Dir, mut cmd: TestCommand| {
    dir.create_bytes("hay", HAY);
    cmd.args(&["--no-mmap", "-c", "Project Gutenberg EBook", "-g", "hay"]);
    cmd.assert_err();
});

// Like after_match1_implicit_count, except the --binary flag is provided,
// which makes ripgrep disable binary data filtering even for implicit files.
rgtest!(
    after_match1_implicit_count_binary,
    |dir: Dir, mut cmd: TestCommand| {
        dir.create_bytes("hay", HAY);
        cmd.args(&[
            "--no-mmap",
            "-c",
            "--binary",
            "Project Gutenberg EBook",
            "-g",
            "hay",
        ]);
        eqnice!("hay:1\n", cmd.stdout());
    }
);

// Like after_match1_implicit_count, except the file path is provided
// explicitly, so binary filtering is disabled and a count is correctly
// reported.
rgtest!(after_match1_explicit_count, |dir: Dir, mut cmd: TestCommand| {
    dir.create_bytes("hay", HAY);
    cmd.args(&["--no-mmap", "-c", "Project Gutenberg EBook", "hay"]);
    eqnice!("1\n", cmd.stdout());
});

// This tests that a match way before the NUL byte is shown, but a match after
// the NUL byte is not.
rgtest!(after_match2_implicit, |dir: Dir, mut cmd: TestCommand| {
    dir.create_bytes("hay", HAY);
    cmd.args(&[
        "--no-mmap",
        "-n",
        "Project Gutenberg EBook|a medical student",
        "-g",
        "hay",
    ]);

    let expected = "\
hay:1:The Project Gutenberg EBook of A Study In Scarlet, by Arthur Conan Doyle
hay: WARNING: stopped searching binary file after match (found \"\\0\" byte around offset 77041)
";
    eqnice!(expected, cmd.stdout());
});

// Like after_match2_implicit, but enables -a/--text, so no binary
// detection should be performed.
rgtest!(after_match2_implicit_text, |dir: Dir, mut cmd: TestCommand| {
    dir.create_bytes("hay", HAY);
    cmd.args(&[
        "--no-mmap",
        "-n",
        "--text",
        "Project Gutenberg EBook|a medical student",
        "-g",
        "hay",
    ]);

    let expected = "\
hay:1:The Project Gutenberg EBook of A Study In Scarlet, by Arthur Conan Doyle
hay:1867:\"And yet you say he is not a medical student?\"
";
    eqnice!(expected, cmd.stdout());
});

// This tests that ripgrep *silently* quits before finding a match that occurs
// after a NUL byte.
rgtest!(before_match1_implicit, |dir: Dir, mut cmd: TestCommand| {
    dir.create_bytes("hay", HAY);
    cmd.args(&["--no-mmap", "-n", "Heaven", "-g", "hay"]);
    cmd.assert_err();
});

// This tests that ripgrep *does not* silently quit before finding a match that
// occurs after a NUL byte when a file is explicitly searched.
rgtest!(before_match1_explicit, |dir: Dir, mut cmd: TestCommand| {
    dir.create_bytes("hay", HAY);
    cmd.args(&["--no-mmap", "-n", "Heaven", "hay"]);

    let expected = "\
binary file matches (found \"\\0\" byte around offset 77041)
";
    eqnice!(expected, cmd.stdout());
});

// Like before_match1_implicit, but enables the --binary flag, which
// disables binary filtering. Thus, this matches the behavior of ripgrep as if
// the file were given explicitly.
rgtest!(before_match1_implicit_binary, |dir: Dir, mut cmd: TestCommand| {
    dir.create_bytes("hay", HAY);
    cmd.args(&["--no-mmap", "-n", "--binary", "Heaven", "-g", "hay"]);

    let expected = "\
hay: binary file matches (found \"\\0\" byte around offset 77041)
";
    eqnice!(expected, cmd.stdout());
});

// Like before_match1_implicit, but enables -a/--text, so no binary
// detection should be performed.
rgtest!(before_match1_implicit_text, |dir: Dir, mut cmd: TestCommand| {
    dir.create_bytes("hay", HAY);
    cmd.args(&["--no-mmap", "-n", "--text", "Heaven", "-g", "hay"]);

    let expected = "\
hay:1871:\"No. Heaven knows what the objects of his studies are. But here we
";
    eqnice!(expected, cmd.stdout());
});

// This tests that ripgrep *silently* quits before finding a match that occurs
// before a NUL byte, but within the same buffer as the NUL byte.
rgtest!(before_match2_implicit, |dir: Dir, mut cmd: TestCommand| {
    dir.create_bytes("hay", HAY);
    cmd.args(&["--no-mmap", "-n", "a medical student", "-g", "hay"]);
    cmd.assert_err();
});

// This tests that ripgrep *does not* silently quit before finding a match that
// occurs before a NUL byte, but within the same buffer as the NUL byte. Even
// though the match occurs before the NUL byte, ripgrep still doesn't print it
// because it has already scanned ahead to detect the NUL byte. (This matches
// the behavior of GNU grep.)
rgtest!(before_match2_explicit, |dir: Dir, mut cmd: TestCommand| {
    dir.create_bytes("hay", HAY);
    cmd.args(&["--no-mmap", "-n", "a medical student", "hay"]);

    let expected = "\
binary file matches (found \"\\0\" byte around offset 77041)
";
    eqnice!(expected, cmd.stdout());
});

// Like before_match1_implicit, but enables -a/--text, so no binary
// detection should be performed.
rgtest!(before_match2_implicit_text, |dir: Dir, mut cmd: TestCommand| {
    dir.create_bytes("hay", HAY);
    cmd.args(&["--no-mmap", "-n", "--text", "a medical student", "-g", "hay"]);

    let expected = "\
hay:1867:\"And yet you say he is not a medical student?\"
";
    eqnice!(expected, cmd.stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/3131
rgtest!(
    matching_files_inconsistent_with_count,
    |dir: Dir, _cmd: TestCommand| {
        let mut file1 = String::new();
        file1.push_str("cat here\n");
        for _ in 0..150_000 {
            file1.push_str("padding line\n");
        }
        file1.push_str("\x00");

        dir.create("file1.txt", &file1);
        dir.create("file2.txt", "cat here");

        let got = dir.command().args(&["--sort=path", "-l", "cat"]).stdout();
        eqnice!("file1.txt\nfile2.txt\n", got);

        // This is the inconsistent result that can't really be avoided without
        // either making `-l/--files-with-matches` much slower or changing
        // what "binary filtering" means.
        let got = dir.command().args(&["--sort=path", "-c", "cat"]).stdout();
        eqnice!("file2.txt:1\n", got);

        let got = dir
            .command()
            .args(&["--sort=path", "-c", "cat", "--binary"])
            .stdout();
        eqnice!("file1.txt:1\nfile2.txt:1\n", got);

        let got = dir
            .command()
            .args(&["--sort=path", "-c", "cat", "--text"])
            .stdout();
        eqnice!("file1.txt:1\nfile2.txt:1\n", got);
    }
);
