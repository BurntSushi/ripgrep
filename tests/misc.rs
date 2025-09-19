use crate::hay::SHERLOCK;
use crate::util::{Dir, TestCommand, cmd_exists, sort_lines};

// This file contains "miscellaneous" tests that were either written before
// features were tracked more explicitly, or were simply written without
// linking them to a specific issue number. We should try to minimize the
// addition of more tests in this file and instead add them to either the
// regression test suite or the feature test suite (found in regression.rs and
// feature.rs, respectively).

rgtest!(single_file, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);

    let expected = "\
For the Doctor Watsons of this world, as opposed to the Sherlock
be, to a very large extent, the result of luck. Sherlock Holmes
";
    eqnice!(expected, cmd.arg("Sherlock").arg("sherlock").stdout());
});

rgtest!(dir, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);

    let expected = "\
sherlock:For the Doctor Watsons of this world, as opposed to the Sherlock
sherlock:be, to a very large extent, the result of luck. Sherlock Holmes
";
    eqnice!(expected, cmd.arg("Sherlock").stdout());
});

rgtest!(line_numbers, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);

    let expected = "\
1:For the Doctor Watsons of this world, as opposed to the Sherlock
3:be, to a very large extent, the result of luck. Sherlock Holmes
";
    eqnice!(expected, cmd.arg("-n").arg("Sherlock").arg("sherlock").stdout());
});

rgtest!(columns, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.arg("--column").arg("Sherlock").arg("sherlock");

    let expected = "\
1:57:For the Doctor Watsons of this world, as opposed to the Sherlock
3:49:be, to a very large extent, the result of luck. Sherlock Holmes
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(with_filename, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.arg("-H").arg("Sherlock").arg("sherlock");

    let expected = "\
sherlock:For the Doctor Watsons of this world, as opposed to the Sherlock
sherlock:be, to a very large extent, the result of luck. Sherlock Holmes
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(with_heading, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.args(&[
        // This forces the issue since --with-filename is disabled by default
        // when searching one file.
        "--with-filename",
        "--heading",
        "Sherlock",
        "sherlock",
    ]);

    let expected = "\
sherlock
For the Doctor Watsons of this world, as opposed to the Sherlock
be, to a very large extent, the result of luck. Sherlock Holmes
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(with_heading_default, |dir: Dir, mut cmd: TestCommand| {
    // Search two or more and get --with-filename enabled by default.
    // Use -j1 to get deterministic results.
    dir.create("sherlock", SHERLOCK);
    dir.create("foo", "Sherlock Holmes lives on Baker Street.");
    cmd.arg("-j1").arg("--heading").arg("Sherlock");

    let expected = "\
foo
Sherlock Holmes lives on Baker Street.

sherlock
For the Doctor Watsons of this world, as opposed to the Sherlock
be, to a very large extent, the result of luck. Sherlock Holmes
";
    eqnice!(sort_lines(expected), sort_lines(&cmd.stdout()));
});

rgtest!(inverted, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.arg("-v").arg("Sherlock").arg("sherlock");

    let expected = "\
Holmeses, success in the province of detective work must always
can extract a clew from a wisp of straw or a flake of cigar ash;
but Doctor Watson has to have it taken out for him and dusted,
and exhibited clearly, with a label attached.
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(inverted_line_numbers, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.arg("-n").arg("-v").arg("Sherlock").arg("sherlock");

    let expected = "\
2:Holmeses, success in the province of detective work must always
4:can extract a clew from a wisp of straw or a flake of cigar ash;
5:but Doctor Watson has to have it taken out for him and dusted,
6:and exhibited clearly, with a label attached.
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(case_insensitive, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.arg("-i").arg("sherlock").arg("sherlock");

    let expected = "\
For the Doctor Watsons of this world, as opposed to the Sherlock
be, to a very large extent, the result of luck. Sherlock Holmes
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(word, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.arg("-w").arg("as").arg("sherlock");

    let expected = "\
For the Doctor Watsons of this world, as opposed to the Sherlock
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(word_period, |dir: Dir, mut cmd: TestCommand| {
    dir.create("haystack", "...");
    cmd.arg("-ow").arg(".").arg("haystack");

    let expected = "\
.
.
.
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(line, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.args(&[
        "-x",
        "Watson|and exhibited clearly, with a label attached.",
        "sherlock",
    ]);

    let expected = "\
and exhibited clearly, with a label attached.
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(literal, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    dir.create("file", "blib\n()\nblab\n");
    cmd.arg("-F").arg("()").arg("file");

    eqnice!("()\n", cmd.stdout());
});

rgtest!(quiet, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.arg("-q").arg("Sherlock").arg("sherlock");

    assert!(cmd.stdout().is_empty());
});

rgtest!(replace, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.arg("-r").arg("FooBar").arg("Sherlock").arg("sherlock");

    let expected = "\
For the Doctor Watsons of this world, as opposed to the FooBar
be, to a very large extent, the result of luck. FooBar Holmes
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(replace_groups, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.args(&["-r", "$2, $1", "([A-Z][a-z]+) ([A-Z][a-z]+)", "sherlock"]);

    let expected = "\
For the Watsons, Doctor of this world, as opposed to the Sherlock
be, to a very large extent, the result of luck. Holmes, Sherlock
but Watson, Doctor has to have it taken out for him and dusted,
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(replace_named_groups, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.args(&[
        "-r",
        "$last, $first",
        "(?P<first>[A-Z][a-z]+) (?P<last>[A-Z][a-z]+)",
        "sherlock",
    ]);

    let expected = "\
For the Watsons, Doctor of this world, as opposed to the Sherlock
be, to a very large extent, the result of luck. Holmes, Sherlock
but Watson, Doctor has to have it taken out for him and dusted,
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(replace_with_only_matching, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.arg("-o").arg("-r").arg("$1").arg(r"of (\w+)").arg("sherlock");

    let expected = "\
this
detective
luck
straw
cigar
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(file_types, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    dir.create("file.py", "Sherlock");
    dir.create("file.rs", "Sherlock");
    cmd.arg("-t").arg("rust").arg("Sherlock");

    eqnice!("file.rs:Sherlock\n", cmd.stdout());
});

rgtest!(file_types_all, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    dir.create("file.py", "Sherlock");
    cmd.arg("-t").arg("all").arg("Sherlock");

    eqnice!("file.py:Sherlock\n", cmd.stdout());
});

rgtest!(file_types_negate, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    dir.remove("sherlock");
    dir.create("file.py", "Sherlock");
    dir.create("file.rs", "Sherlock");
    cmd.arg("-T").arg("rust").arg("Sherlock");

    eqnice!("file.py:Sherlock\n", cmd.stdout());
});

rgtest!(file_types_negate_all, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    dir.create("file.py", "Sherlock");
    cmd.arg("-T").arg("all").arg("Sherlock");

    let expected = "\
sherlock:For the Doctor Watsons of this world, as opposed to the Sherlock
sherlock:be, to a very large extent, the result of luck. Sherlock Holmes
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(file_type_clear, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    dir.create("file.py", "Sherlock");
    dir.create("file.rs", "Sherlock");
    cmd.arg("--type-clear").arg("rust").arg("-t").arg("rust").arg("Sherlock");

    cmd.assert_non_empty_stderr();
});

rgtest!(file_type_add, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    dir.create("file.py", "Sherlock");
    dir.create("file.rs", "Sherlock");
    dir.create("file.wat", "Sherlock");
    cmd.args(&["--type-add", "wat:*.wat", "-t", "wat", "Sherlock"]);

    eqnice!("file.wat:Sherlock\n", cmd.stdout());
});

rgtest!(file_type_add_compose, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    dir.create("file.py", "Sherlock");
    dir.create("file.rs", "Sherlock");
    dir.create("file.wat", "Sherlock");
    cmd.args(&[
        "--type-add",
        "wat:*.wat",
        "--type-add",
        "combo:include:wat,py",
        "-t",
        "combo",
        "Sherlock",
    ]);

    let expected = "\
file.py:Sherlock
file.wat:Sherlock
";
    eqnice!(expected, sort_lines(&cmd.stdout()));
});

rgtest!(glob, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    dir.create("file.py", "Sherlock");
    dir.create("file.rs", "Sherlock");
    cmd.arg("-g").arg("*.rs").arg("Sherlock");

    eqnice!("file.rs:Sherlock\n", cmd.stdout());
});

rgtest!(glob_negate, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    dir.remove("sherlock");
    dir.create("file.py", "Sherlock");
    dir.create("file.rs", "Sherlock");
    cmd.arg("-g").arg("!*.rs").arg("Sherlock");

    eqnice!("file.py:Sherlock\n", cmd.stdout());
});

rgtest!(glob_case_insensitive, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    dir.create("file.HTML", "Sherlock");
    cmd.arg("--iglob").arg("*.html").arg("Sherlock");

    eqnice!("file.HTML:Sherlock\n", cmd.stdout());
});

rgtest!(glob_case_sensitive, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    dir.create("file1.HTML", "Sherlock");
    dir.create("file2.html", "Sherlock");
    cmd.arg("--glob").arg("*.html").arg("Sherlock");

    eqnice!("file2.html:Sherlock\n", cmd.stdout());
});

rgtest!(glob_always_case_insensitive, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    dir.create("file.HTML", "Sherlock");
    cmd.args(&["--glob-case-insensitive", "--glob", "*.html", "Sherlock"]);

    eqnice!("file.HTML:Sherlock\n", cmd.stdout());
});

rgtest!(byte_offset_only_matching, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.arg("-b").arg("-o").arg("Sherlock");

    let expected = "\
sherlock:56:Sherlock
sherlock:177:Sherlock
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(count, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.arg("--count").arg("Sherlock");

    let expected = "sherlock:2\n";
    eqnice!(expected, cmd.stdout());
});

rgtest!(count_matches, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.arg("--count-matches").arg("the");

    let expected = "sherlock:4\n";
    eqnice!(expected, cmd.stdout());
});

rgtest!(count_matches_inverted, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.arg("--count-matches").arg("--invert-match").arg("Sherlock");

    let expected = "sherlock:4\n";
    eqnice!(expected, cmd.stdout());
});

rgtest!(count_matches_via_only, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.arg("--count").arg("--only-matching").arg("the");

    let expected = "sherlock:4\n";
    eqnice!(expected, cmd.stdout());
});

rgtest!(include_zero, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.args(&["--count", "--include-zero", "nada"]);
    cmd.assert_err();

    let output = cmd.raw_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let expected = "sherlock:0\n";

    eqnice!(expected, stdout);
});

rgtest!(include_zero_override, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.args(&["--count", "--include-zero", "--no-include-zero", "nada"]);
    cmd.assert_err();

    let output = cmd.raw_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.is_empty());
});

rgtest!(files_with_matches, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.arg("--files-with-matches").arg("Sherlock");

    let expected = "sherlock\n";
    eqnice!(expected, cmd.stdout());
});

rgtest!(files_without_match, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    dir.create("file.py", "foo");
    cmd.arg("--files-without-match").arg("Sherlock");

    let expected = "file.py\n";
    eqnice!(expected, cmd.stdout());
});

rgtest!(after_context, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.arg("-A").arg("1").arg("Sherlock").arg("sherlock");

    let expected = "\
For the Doctor Watsons of this world, as opposed to the Sherlock
Holmeses, success in the province of detective work must always
be, to a very large extent, the result of luck. Sherlock Holmes
can extract a clew from a wisp of straw or a flake of cigar ash;
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(after_context_line_numbers, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.arg("-A").arg("1").arg("-n").arg("Sherlock").arg("sherlock");

    let expected = "\
1:For the Doctor Watsons of this world, as opposed to the Sherlock
2-Holmeses, success in the province of detective work must always
3:be, to a very large extent, the result of luck. Sherlock Holmes
4-can extract a clew from a wisp of straw or a flake of cigar ash;
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(before_context, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.arg("-B").arg("1").arg("Sherlock").arg("sherlock");

    let expected = "\
For the Doctor Watsons of this world, as opposed to the Sherlock
Holmeses, success in the province of detective work must always
be, to a very large extent, the result of luck. Sherlock Holmes
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(before_context_line_numbers, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.arg("-B").arg("1").arg("-n").arg("Sherlock").arg("sherlock");

    let expected = "\
1:For the Doctor Watsons of this world, as opposed to the Sherlock
2-Holmeses, success in the province of detective work must always
3:be, to a very large extent, the result of luck. Sherlock Holmes
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(context, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.arg("-C").arg("1").arg("world|attached").arg("sherlock");

    let expected = "\
For the Doctor Watsons of this world, as opposed to the Sherlock
Holmeses, success in the province of detective work must always
--
but Doctor Watson has to have it taken out for him and dusted,
and exhibited clearly, with a label attached.
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(context_line_numbers, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.arg("-C").arg("1").arg("-n").arg("world|attached").arg("sherlock");

    let expected = "\
1:For the Doctor Watsons of this world, as opposed to the Sherlock
2-Holmeses, success in the province of detective work must always
--
5-but Doctor Watson has to have it taken out for him and dusted,
6:and exhibited clearly, with a label attached.
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(max_filesize_parse_error_length, |_: Dir, mut cmd: TestCommand| {
    cmd.arg("--max-filesize").arg("44444444444444444444");
    cmd.assert_non_empty_stderr();
});

rgtest!(max_filesize_parse_error_suffix, |_: Dir, mut cmd: TestCommand| {
    cmd.arg("--max-filesize").arg("45k");
    cmd.assert_non_empty_stderr();
});

rgtest!(max_filesize_parse_no_suffix, |dir: Dir, mut cmd: TestCommand| {
    dir.create_size("foo", 40);
    dir.create_size("bar", 60);
    cmd.arg("--max-filesize").arg("50").arg("--files");

    eqnice!("foo\n", cmd.stdout());
});

rgtest!(max_filesize_parse_k_suffix, |dir: Dir, mut cmd: TestCommand| {
    dir.create_size("foo", 3048);
    dir.create_size("bar", 4100);
    cmd.arg("--max-filesize").arg("4K").arg("--files");

    eqnice!("foo\n", cmd.stdout());
});

rgtest!(max_filesize_parse_m_suffix, |dir: Dir, mut cmd: TestCommand| {
    dir.create_size("foo", 1000000);
    dir.create_size("bar", 1400000);
    cmd.arg("--max-filesize").arg("1M").arg("--files");

    eqnice!("foo\n", cmd.stdout());
});

rgtest!(max_filesize_suffix_overflow, |dir: Dir, mut cmd: TestCommand| {
    dir.create_size("foo", 1000000);

    // 2^35 * 2^30 would otherwise overflow
    cmd.arg("--max-filesize").arg("34359738368G").arg("--files");
    cmd.assert_non_empty_stderr();
});

rgtest!(ignore_hidden, |dir: Dir, mut cmd: TestCommand| {
    dir.create(".sherlock", SHERLOCK);
    cmd.arg("Sherlock").assert_err();
});

rgtest!(no_ignore_hidden, |dir: Dir, mut cmd: TestCommand| {
    dir.create(".sherlock", SHERLOCK);
    cmd.arg("--hidden").arg("Sherlock");

    let expected = "\
.sherlock:For the Doctor Watsons of this world, as opposed to the Sherlock
.sherlock:be, to a very large extent, the result of luck. Sherlock Holmes
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(ignore_git, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    dir.create_dir(".git");
    dir.create(".gitignore", "sherlock\n");
    cmd.arg("Sherlock");

    cmd.assert_err();
});

rgtest!(ignore_generic, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    dir.create(".ignore", "sherlock\n");
    cmd.arg("Sherlock");

    cmd.assert_err();
});

rgtest!(ignore_ripgrep, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    dir.create(".rgignore", "sherlock\n");
    cmd.arg("Sherlock");

    cmd.assert_err();
});

rgtest!(no_ignore, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    dir.create(".gitignore", "sherlock\n");
    cmd.arg("--no-ignore").arg("Sherlock");

    let expected = "\
sherlock:For the Doctor Watsons of this world, as opposed to the Sherlock
sherlock:be, to a very large extent, the result of luck. Sherlock Holmes
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(ignore_git_parent, |dir: Dir, mut cmd: TestCommand| {
    dir.create_dir(".git");
    dir.create(".gitignore", "sherlock\n");
    dir.create_dir("foo");
    dir.create("foo/sherlock", SHERLOCK);
    cmd.arg("Sherlock");

    // Even though we search in foo/, which has no .gitignore, ripgrep will
    // traverse parent directories and respect the gitignore files found.
    cmd.current_dir(dir.path().join("foo"));
    cmd.assert_err();
});

rgtest!(ignore_git_parent_stop, |dir: Dir, mut cmd: TestCommand| {
    // This tests that searching parent directories for .gitignore files stops
    // after it sees a .git directory. To test this, we create this directory
    // hierarchy:
    //
    // .gitignore (contains `sherlock`)
    // foo/
    //   .git/
    //   bar/
    //      sherlock
    //
    // And we perform the search inside `foo/bar/`. ripgrep will stop looking
    // for .gitignore files after it sees `foo/.git/`, and therefore not
    // respect the top-level `.gitignore` containing `sherlock`.
    dir.create(".gitignore", "sherlock\n");
    dir.create_dir("foo");
    dir.create_dir("foo/.git");
    dir.create_dir("foo/bar");
    dir.create("foo/bar/sherlock", SHERLOCK);
    cmd.arg("Sherlock");
    cmd.current_dir(dir.path().join("foo").join("bar"));

    let expected = "\
sherlock:For the Doctor Watsons of this world, as opposed to the Sherlock
sherlock:be, to a very large extent, the result of luck. Sherlock Holmes
";
    eqnice!(expected, cmd.stdout());
});

// Like ignore_git_parent_stop, but with a .git file instead of a .git
// directory.
rgtest!(ignore_git_parent_stop_file, |dir: Dir, mut cmd: TestCommand| {
    // This tests that searching parent directories for .gitignore files stops
    // after it sees a .git *file*. A .git file is used for submodules. To test
    // this, we create this directory hierarchy:
    //
    // .gitignore (contains `sherlock`)
    // foo/
    //   .git
    //   bar/
    //      sherlock
    //
    // And we perform the search inside `foo/bar/`. ripgrep will stop looking
    // for .gitignore files after it sees `foo/.git`, and therefore not
    // respect the top-level `.gitignore` containing `sherlock`.
    dir.create(".gitignore", "sherlock\n");
    dir.create_dir("foo");
    dir.create("foo/.git", "");
    dir.create_dir("foo/bar");
    dir.create("foo/bar/sherlock", SHERLOCK);
    cmd.arg("Sherlock");
    cmd.current_dir(dir.path().join("foo").join("bar"));

    let expected = "\
sherlock:For the Doctor Watsons of this world, as opposed to the Sherlock
sherlock:be, to a very large extent, the result of luck. Sherlock Holmes
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(ignore_ripgrep_parent_no_stop, |dir: Dir, mut cmd: TestCommand| {
    // This is like the `ignore_git_parent_stop` test, except it checks that
    // ripgrep *doesn't* stop checking for .rgignore files.
    dir.create(".rgignore", "sherlock\n");
    dir.create_dir("foo");
    dir.create_dir("foo/.git");
    dir.create_dir("foo/bar");
    dir.create("foo/bar/sherlock", SHERLOCK);
    cmd.arg("Sherlock");
    cmd.current_dir(dir.path().join("foo").join("bar"));

    // The top-level .rgignore applies.
    cmd.assert_err();
});

rgtest!(no_parent_ignore_git, |dir: Dir, mut cmd: TestCommand| {
    // Set up a directory hierarchy like this:
    //
    // .git/
    // .gitignore
    // foo/
    //   .gitignore
    //   sherlock
    //   watson
    //
    // Where `.gitignore` contains `sherlock` and `foo/.gitignore` contains
    // `watson`.
    //
    // Now *do the search* from the foo directory. By default, ripgrep will
    // search parent directories for .gitignore files. The --no-ignore-parent
    // flag should prevent that. At the same time, the `foo/.gitignore` file
    // will still be respected (since the search is happening in `foo/`).
    //
    // In other words, we should only see results from `sherlock`, not from
    // `watson`.
    dir.create_dir(".git");
    dir.create(".gitignore", "sherlock\n");
    dir.create_dir("foo");
    dir.create("foo/.gitignore", "watson\n");
    dir.create("foo/sherlock", SHERLOCK);
    dir.create("foo/watson", SHERLOCK);
    cmd.arg("--no-ignore-parent").arg("Sherlock");
    cmd.current_dir(dir.path().join("foo"));

    let expected = "\
sherlock:For the Doctor Watsons of this world, as opposed to the Sherlock
sherlock:be, to a very large extent, the result of luck. Sherlock Holmes
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(symlink_nofollow, |dir: Dir, mut cmd: TestCommand| {
    dir.create_dir("foo");
    dir.create_dir("foo/bar");
    dir.link_dir("foo/baz", "foo/bar/baz");
    dir.create_dir("foo/baz");
    dir.create("foo/baz/sherlock", SHERLOCK);
    cmd.arg("Sherlock");
    cmd.current_dir(dir.path().join("foo/bar"));

    cmd.assert_err();
});

#[cfg(not(windows))]
rgtest!(symlink_follow, |dir: Dir, mut cmd: TestCommand| {
    dir.create_dir("foo");
    dir.create_dir("foo/bar");
    dir.create_dir("foo/baz");
    dir.create("foo/baz/sherlock", SHERLOCK);
    dir.link_dir("foo/baz", "foo/bar/baz");
    cmd.arg("-L").arg("Sherlock");
    cmd.current_dir(dir.path().join("foo/bar"));

    let expected = "\
baz/sherlock:For the Doctor Watsons of this world, as opposed to the Sherlock
baz/sherlock:be, to a very large extent, the result of luck. Sherlock Holmes
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(unrestricted1, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    dir.create(".gitignore", "sherlock\n");
    cmd.arg("-u").arg("Sherlock");

    let expected = "\
sherlock:For the Doctor Watsons of this world, as opposed to the Sherlock
sherlock:be, to a very large extent, the result of luck. Sherlock Holmes
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(unrestricted2, |dir: Dir, mut cmd: TestCommand| {
    dir.create(".sherlock", SHERLOCK);
    cmd.arg("-uu").arg("Sherlock");

    let expected = "\
.sherlock:For the Doctor Watsons of this world, as opposed to the Sherlock
.sherlock:be, to a very large extent, the result of luck. Sherlock Holmes
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(unrestricted3, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    dir.create("hay", "foo\x00bar\nfoo\x00baz\n");
    cmd.arg("-uuu").arg("foo");

    let expected = "\
hay: binary file matches (found \"\\0\" byte around offset 3)
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(vimgrep, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.arg("--vimgrep").arg("Sherlock|Watson");

    let expected = "\
sherlock:1:16:For the Doctor Watsons of this world, as opposed to the Sherlock
sherlock:1:57:For the Doctor Watsons of this world, as opposed to the Sherlock
sherlock:3:49:be, to a very large extent, the result of luck. Sherlock Holmes
sherlock:5:12:but Doctor Watson has to have it taken out for him and dusted,
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(vimgrep_no_line, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.arg("--vimgrep").arg("-N").arg("Sherlock|Watson");

    let expected = "\
sherlock:16:For the Doctor Watsons of this world, as opposed to the Sherlock
sherlock:57:For the Doctor Watsons of this world, as opposed to the Sherlock
sherlock:49:be, to a very large extent, the result of luck. Sherlock Holmes
sherlock:12:but Doctor Watson has to have it taken out for him and dusted,
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(vimgrep_no_line_no_column, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.arg("--vimgrep").arg("-N").arg("--no-column").arg("Sherlock|Watson");

    let expected = "\
sherlock:For the Doctor Watsons of this world, as opposed to the Sherlock
sherlock:For the Doctor Watsons of this world, as opposed to the Sherlock
sherlock:be, to a very large extent, the result of luck. Sherlock Holmes
sherlock:but Doctor Watson has to have it taken out for him and dusted,
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(preprocessing, |dir: Dir, mut cmd: TestCommand| {
    if !cmd_exists("xzcat") {
        return;
    }

    dir.create_bytes("sherlock.xz", include_bytes!("./data/sherlock.xz"));
    cmd.arg("--pre").arg("xzcat").arg("Sherlock").arg("sherlock.xz");

    let expected = "\
For the Doctor Watsons of this world, as opposed to the Sherlock
be, to a very large extent, the result of luck. Sherlock Holmes
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(preprocessing_glob, |dir: Dir, mut cmd: TestCommand| {
    if !cmd_exists("xzcat") {
        return;
    }

    dir.create("sherlock", SHERLOCK);
    dir.create_bytes("sherlock.xz", include_bytes!("./data/sherlock.xz"));
    cmd.args(&["--pre", "xzcat", "--pre-glob", "*.xz", "Sherlock"]);

    let expected = "\
sherlock.xz:For the Doctor Watsons of this world, as opposed to the Sherlock
sherlock.xz:be, to a very large extent, the result of luck. Sherlock Holmes
sherlock:For the Doctor Watsons of this world, as opposed to the Sherlock
sherlock:be, to a very large extent, the result of luck. Sherlock Holmes
";
    eqnice!(sort_lines(expected), sort_lines(&cmd.stdout()));
});

rgtest!(compressed_gzip, |dir: Dir, mut cmd: TestCommand| {
    if !cmd_exists("gzip") {
        return;
    }

    dir.create_bytes("sherlock.gz", include_bytes!("./data/sherlock.gz"));
    cmd.arg("-z").arg("Sherlock").arg("sherlock.gz");

    let expected = "\
For the Doctor Watsons of this world, as opposed to the Sherlock
be, to a very large extent, the result of luck. Sherlock Holmes
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(compressed_bzip2, |dir: Dir, mut cmd: TestCommand| {
    if !cmd_exists("bzip2") {
        return;
    }

    dir.create_bytes("sherlock.bz2", include_bytes!("./data/sherlock.bz2"));
    cmd.arg("-z").arg("Sherlock").arg("sherlock.bz2");

    let expected = "\
For the Doctor Watsons of this world, as opposed to the Sherlock
be, to a very large extent, the result of luck. Sherlock Holmes
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(compressed_xz, |dir: Dir, mut cmd: TestCommand| {
    if !cmd_exists("xz") {
        return;
    }

    dir.create_bytes("sherlock.xz", include_bytes!("./data/sherlock.xz"));
    cmd.arg("-z").arg("Sherlock").arg("sherlock.xz");

    let expected = "\
For the Doctor Watsons of this world, as opposed to the Sherlock
be, to a very large extent, the result of luck. Sherlock Holmes
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(compressed_lz4, |dir: Dir, mut cmd: TestCommand| {
    if !cmd_exists("lz4") {
        return;
    }

    dir.create_bytes("sherlock.lz4", include_bytes!("./data/sherlock.lz4"));
    cmd.arg("-z").arg("Sherlock").arg("sherlock.lz4");

    let expected = "\
For the Doctor Watsons of this world, as opposed to the Sherlock
be, to a very large extent, the result of luck. Sherlock Holmes
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(compressed_lzma, |dir: Dir, mut cmd: TestCommand| {
    if !cmd_exists("xz") {
        return;
    }

    dir.create_bytes("sherlock.lzma", include_bytes!("./data/sherlock.lzma"));
    cmd.arg("-z").arg("Sherlock").arg("sherlock.lzma");

    let expected = "\
For the Doctor Watsons of this world, as opposed to the Sherlock
be, to a very large extent, the result of luck. Sherlock Holmes
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(compressed_brotli, |dir: Dir, mut cmd: TestCommand| {
    if !cmd_exists("brotli") {
        return;
    }

    dir.create_bytes("sherlock.br", include_bytes!("./data/sherlock.br"));
    cmd.arg("-z").arg("Sherlock").arg("sherlock.br");

    let expected = "\
For the Doctor Watsons of this world, as opposed to the Sherlock
be, to a very large extent, the result of luck. Sherlock Holmes
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(compressed_zstd, |dir: Dir, mut cmd: TestCommand| {
    if !cmd_exists("zstd") {
        return;
    }

    dir.create_bytes("sherlock.zst", include_bytes!("./data/sherlock.zst"));
    cmd.arg("-z").arg("Sherlock").arg("sherlock.zst");

    let expected = "\
For the Doctor Watsons of this world, as opposed to the Sherlock
be, to a very large extent, the result of luck. Sherlock Holmes
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(compressed_uncompress, |dir: Dir, mut cmd: TestCommand| {
    if !cmd_exists("uncompress") {
        return;
    }

    dir.create_bytes("sherlock.Z", include_bytes!("./data/sherlock.Z"));
    cmd.arg("-z").arg("Sherlock").arg("sherlock.Z");

    let expected = "\
    For the Doctor Watsons of this world, as opposed to the Sherlock
be, to a very large extent, the result of luck. Sherlock Holmes
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(compressed_failing_gzip, |dir: Dir, mut cmd: TestCommand| {
    if !cmd_exists("gzip") {
        return;
    }

    dir.create("sherlock.gz", SHERLOCK);
    cmd.arg("-z").arg("Sherlock").arg("sherlock.gz");

    cmd.assert_non_empty_stderr();
});

rgtest!(binary_convert, |dir: Dir, mut cmd: TestCommand| {
    dir.create("file", "foo\x00bar\nfoo\x00baz\n");
    cmd.arg("--no-mmap").arg("foo").arg("file");

    let expected = "\
binary file matches (found \"\\0\" byte around offset 3)
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(binary_convert_mmap, |dir: Dir, mut cmd: TestCommand| {
    dir.create("file", "foo\x00bar\nfoo\x00baz\n");
    cmd.arg("--mmap").arg("foo").arg("file");

    let expected = "\
binary file matches (found \"\\0\" byte around offset 3)
";
    eqnice!(expected, cmd.stdout());
});

rgtest!(binary_quit, |dir: Dir, mut cmd: TestCommand| {
    dir.create("file", "foo\x00bar\nfoo\x00baz\n");
    cmd.arg("--no-mmap").arg("foo").arg("-gfile");
    cmd.assert_err();
});

rgtest!(binary_quit_mmap, |dir: Dir, mut cmd: TestCommand| {
    dir.create("file", "foo\x00bar\nfoo\x00baz\n");
    cmd.arg("--mmap").arg("foo").arg("-gfile");
    cmd.assert_err();
});

// The following two tests show a discrepancy in search results between
// searching with memory mapped files and stream searching. Stream searching
// uses a heuristic (that GNU grep also uses) where NUL bytes are replaced with
// the EOL terminator, which tends to avoid allocating large amounts of memory
// for really long "lines." The memory map searcher has no need to worry about
// such things, and more than that, it would be pretty hard for it to match the
// semantics of streaming search in this case.
//
// Binary files with lots of NULs aren't really part of the use case of ripgrep
// (or any other grep-like tool for that matter), so we shouldn't feel too bad
// about it.
rgtest!(binary_search_mmap, |dir: Dir, mut cmd: TestCommand| {
    dir.create("file", "foo\x00bar\nfoo\x00baz\n");
    cmd.arg("-a").arg("--mmap").arg("foo").arg("file");
    eqnice!("foo\x00bar\nfoo\x00baz\n", cmd.stdout());
});

rgtest!(binary_search_no_mmap, |dir: Dir, mut cmd: TestCommand| {
    dir.create("file", "foo\x00bar\nfoo\x00baz\n");
    cmd.arg("-a").arg("--no-mmap").arg("foo").arg("file");
    eqnice!("foo\x00bar\nfoo\x00baz\n", cmd.stdout());
});

rgtest!(files, |dir: Dir, mut cmd: TestCommand| {
    dir.create("file", "");
    dir.create_dir("dir");
    dir.create("dir/file", "");
    cmd.arg("--files");

    eqnice!(sort_lines("file\ndir/file\n"), sort_lines(&cmd.stdout()));
});

rgtest!(type_list, |_: Dir, mut cmd: TestCommand| {
    cmd.arg("--type-list");
    // This can change over time, so just make sure we print something.
    assert!(!cmd.stdout().is_empty());
});

// The following series of tests seeks to test all permutations of ripgrep's
// sorted queries.
//
// They all rely on this setup function, which sets up this particular file
// structure with a particular creation order:
//  ├── a             # 1
//  ├── b             # 4
//  └── dir           # 2
//     ├── c          # 3
//     └── d          # 5
//
// This order is important when sorting them by system time-stamps.
fn sort_setup(dir: Dir) {
    use std::{thread::sleep, time::Duration};
    // As reported in https://github.com/BurntSushi/ripgrep/issues/3071
    // this test fails if sufficient delay is not given on Windows/Aarch64.
    let delay = if cfg!(all(windows, target_arch = "aarch64")) {
        Duration::from_millis(1000)
    } else {
        Duration::from_millis(100)
    };
    let sub_dir = dir.path().join("dir");
    dir.create("a", "test");
    sleep(delay);
    dir.create_dir(&sub_dir);
    sleep(delay);
    dir.create(sub_dir.join("c"), "test");
    sleep(delay);
    dir.create("b", "test");
    sleep(delay);
    dir.create(sub_dir.join("d"), "test");
}

rgtest!(sort_files, |dir: Dir, mut cmd: TestCommand| {
    sort_setup(dir);
    let expected = "a:test\nb:test\ndir/c:test\ndir/d:test\n";
    eqnice!(expected, cmd.args(["--sort", "path", "test"]).stdout());
});

rgtest!(sort_accessed, |dir: Dir, mut cmd: TestCommand| {
    if crate::util::is_cross() {
        return;
    }
    sort_setup(dir);
    let expected = "a:test\ndir/c:test\nb:test\ndir/d:test\n";
    eqnice!(expected, cmd.args(["--sort", "accessed", "test"]).stdout());
});

rgtest!(sortr_accessed, |dir: Dir, mut cmd: TestCommand| {
    if crate::util::is_cross() {
        return;
    }
    sort_setup(dir);
    let expected = "dir/d:test\nb:test\ndir/c:test\na:test\n";
    eqnice!(expected, cmd.args(["--sortr", "accessed", "test"]).stdout());
});
