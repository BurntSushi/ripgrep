use crate::hay::SHERLOCK;
use crate::util::{Dir, TestCommand, sort_lines};

// See: https://github.com/BurntSushi/ripgrep/issues/16
rgtest!(r16, |dir: Dir, mut cmd: TestCommand| {
    dir.create_dir(".git");
    dir.create(".gitignore", "ghi/");
    dir.create_dir("ghi");
    dir.create_dir("def/ghi");
    dir.create("ghi/toplevel.txt", "xyz");
    dir.create("def/ghi/subdir.txt", "xyz");

    cmd.arg("xyz").assert_err();
});

// See: https://github.com/BurntSushi/ripgrep/issues/25
rgtest!(r25, |dir: Dir, mut cmd: TestCommand| {
    dir.create_dir(".git");
    dir.create(".gitignore", "/llvm/");
    dir.create_dir("src/llvm");
    dir.create("src/llvm/foo", "test");

    cmd.arg("test");
    eqnice!("src/llvm/foo:test\n", cmd.stdout());

    cmd.current_dir(dir.path().join("src"));
    eqnice!("llvm/foo:test\n", cmd.stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/30
rgtest!(r30, |dir: Dir, mut cmd: TestCommand| {
    dir.create(".gitignore", "vendor/**\n!vendor/manifest");
    dir.create_dir("vendor");
    dir.create("vendor/manifest", "test");

    eqnice!("vendor/manifest:test\n", cmd.arg("test").stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/49
rgtest!(r49, |dir: Dir, mut cmd: TestCommand| {
    dir.create(".gitignore", "foo/bar");
    dir.create_dir("test/foo/bar");
    dir.create("test/foo/bar/baz", "test");

    cmd.arg("xyz").assert_err();
});

// See: https://github.com/BurntSushi/ripgrep/issues/50
rgtest!(r50, |dir: Dir, mut cmd: TestCommand| {
    dir.create(".gitignore", "XXX/YYY/");
    dir.create_dir("abc/def/XXX/YYY");
    dir.create_dir("ghi/XXX/YYY");
    dir.create("abc/def/XXX/YYY/bar", "test");
    dir.create("ghi/XXX/YYY/bar", "test");

    cmd.arg("xyz").assert_err();
});

// See: https://github.com/BurntSushi/ripgrep/issues/64
rgtest!(r64, |dir: Dir, mut cmd: TestCommand| {
    dir.create_dir("dir");
    dir.create_dir("foo");
    dir.create("dir/abc", "");
    dir.create("foo/abc", "");

    eqnice!("foo/abc\n", cmd.arg("--files").arg("foo").stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/65
rgtest!(r65, |dir: Dir, mut cmd: TestCommand| {
    dir.create_dir(".git");
    dir.create(".gitignore", "a/");
    dir.create_dir("a");
    dir.create("a/foo", "xyz");
    dir.create("a/bar", "xyz");

    cmd.arg("xyz").assert_err();
});

// See: https://github.com/BurntSushi/ripgrep/issues/67
rgtest!(r67, |dir: Dir, mut cmd: TestCommand| {
    dir.create_dir(".git");
    dir.create(".gitignore", "/*\n!/dir");
    dir.create_dir("dir");
    dir.create_dir("foo");
    dir.create("foo/bar", "test");
    dir.create("dir/bar", "test");

    eqnice!("dir/bar:test\n", cmd.arg("test").stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/87
rgtest!(r87, |dir: Dir, mut cmd: TestCommand| {
    dir.create_dir(".git");
    dir.create(".gitignore", "foo\n**no-vcs**");
    dir.create("foo", "test");

    cmd.arg("test").assert_err();
});

// See: https://github.com/BurntSushi/ripgrep/issues/90
rgtest!(r90, |dir: Dir, mut cmd: TestCommand| {
    dir.create_dir(".git");
    dir.create(".gitignore", "!.foo");
    dir.create(".foo", "test");

    eqnice!(".foo:test\n", cmd.arg("test").stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/93
rgtest!(r93, |dir: Dir, mut cmd: TestCommand| {
    dir.create("foo", "192.168.1.1");

    eqnice!("foo:192.168.1.1\n", cmd.arg(r"(\d{1,3}\.){3}\d{1,3}").stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/99
rgtest!(r99, |dir: Dir, mut cmd: TestCommand| {
    dir.create("foo1", "test");
    dir.create("foo2", "zzz");
    dir.create("bar", "test");

    eqnice!(
        sort_lines("bar\ntest\n\nfoo1\ntest\n"),
        sort_lines(&cmd.arg("-j1").arg("--heading").arg("test").stdout())
    );
});

// See: https://github.com/BurntSushi/ripgrep/issues/105
rgtest!(r105_part1, |dir: Dir, mut cmd: TestCommand| {
    dir.create("foo", "zztest");

    eqnice!("foo:1:3:zztest\n", cmd.arg("--vimgrep").arg("test").stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/105
rgtest!(r105_part2, |dir: Dir, mut cmd: TestCommand| {
    dir.create("foo", "zztest");

    eqnice!("foo:1:3:zztest\n", cmd.arg("--column").arg("test").stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/127
rgtest!(r127, |dir: Dir, mut cmd: TestCommand| {
    // Set up a directory hierarchy like this:
    //
    // .gitignore
    // foo/
    //   sherlock
    //   watson
    //
    // Where `.gitignore` contains `foo/sherlock`.
    //
    // ripgrep should ignore 'foo/sherlock' giving us results only from
    // 'foo/watson' but on Windows ripgrep will include both 'foo/sherlock' and
    // 'foo/watson' in the search results.
    dir.create_dir(".git");
    dir.create(".gitignore", "foo/sherlock\n");
    dir.create_dir("foo");
    dir.create("foo/sherlock", SHERLOCK);
    dir.create("foo/watson", SHERLOCK);

    let expected = "\
foo/watson:For the Doctor Watsons of this world, as opposed to the Sherlock
foo/watson:be, to a very large extent, the result of luck. Sherlock Holmes
";
    assert_eq!(expected, cmd.arg("Sherlock").stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/128
rgtest!(r128, |dir: Dir, mut cmd: TestCommand| {
    dir.create_bytes("foo", b"01234567\x0b\n\x0b\n\x0b\n\x0b\nx");

    eqnice!("foo:5:x\n", cmd.arg("-n").arg("x").stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/131
//
// TODO(burntsushi): Darwin doesn't like this test for some reason. Probably
// due to the weird file path.
#[cfg(not(target_os = "macos"))]
rgtest!(r131, |dir: Dir, mut cmd: TestCommand| {
    dir.create_dir(".git");
    dir.create(".gitignore", "TopÑapa");
    dir.create("TopÑapa", "test");

    cmd.arg("test").assert_err();
});

// See: https://github.com/BurntSushi/ripgrep/issues/137
//
// TODO(burntsushi): Figure out how to make this test work on Windows. Right
// now it gives "access denied" errors when trying to create a file symlink.
// For now, disable test on Windows.
#[cfg(not(windows))]
rgtest!(r137, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    dir.link_file("sherlock", "sym1");
    dir.link_file("sherlock", "sym2");

    let expected = "\
./sherlock:For the Doctor Watsons of this world, as opposed to the Sherlock
./sherlock:be, to a very large extent, the result of luck. Sherlock Holmes
sym1:For the Doctor Watsons of this world, as opposed to the Sherlock
sym1:be, to a very large extent, the result of luck. Sherlock Holmes
sym2:For the Doctor Watsons of this world, as opposed to the Sherlock
sym2:be, to a very large extent, the result of luck. Sherlock Holmes
";
    cmd.arg("-j1").arg("Sherlock").arg("./").arg("sym1").arg("sym2");
    eqnice!(expected, cmd.stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/156
rgtest!(r156, |dir: Dir, mut cmd: TestCommand| {
    let expected = r#"#parse('widgets/foo_bar_macros.vm')
#parse ( 'widgets/mobile/foo_bar_macros.vm' )
#parse ("widgets/foobarhiddenformfields.vm")
#parse ( "widgets/foo_bar_legal.vm" )
#include( 'widgets/foo_bar_tips.vm' )
#include('widgets/mobile/foo_bar_macros.vm')
#include ("widgets/mobile/foo_bar_resetpw.vm")
#parse('widgets/foo-bar-macros.vm')
#parse ( 'widgets/mobile/foo-bar-macros.vm' )
#parse ("widgets/foo-bar-hiddenformfields.vm")
#parse ( "widgets/foo-bar-legal.vm" )
#include( 'widgets/foo-bar-tips.vm' )
#include('widgets/mobile/foo-bar-macros.vm')
#include ("widgets/mobile/foo-bar-resetpw.vm")
"#;
    dir.create("testcase.txt", expected);

    cmd.arg("-N");
    cmd.arg(r#"#(?:parse|include)\s*\(\s*(?:"|')[./A-Za-z_-]+(?:"|')"#);
    cmd.arg("testcase.txt");
    eqnice!(expected, cmd.stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/184
rgtest!(r184, |dir: Dir, mut cmd: TestCommand| {
    dir.create(".gitignore", ".*");
    dir.create_dir("foo/bar");
    dir.create("foo/bar/baz", "test");

    cmd.arg("test");
    eqnice!("foo/bar/baz:test\n", cmd.stdout());

    cmd.current_dir(dir.path().join("./foo/bar"));
    eqnice!("baz:test\n", cmd.stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/199
rgtest!(r199, |dir: Dir, mut cmd: TestCommand| {
    dir.create("foo", "tEsT");

    eqnice!("foo:tEsT\n", cmd.arg("--smart-case").arg(r"\btest\b").stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/206
rgtest!(r206, |dir: Dir, mut cmd: TestCommand| {
    dir.create_dir("foo");
    dir.create("foo/bar.txt", "test");

    cmd.arg("test").arg("-g").arg("*.txt");
    eqnice!("foo/bar.txt:test\n", cmd.stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/210
#[cfg(unix)]
rgtest!(r210, |dir: Dir, mut cmd: TestCommand| {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let badutf8 = OsStr::from_bytes(&b"foo\xffbar"[..]);

    // APFS does not support creating files with invalid UTF-8 bytes.
    // https://github.com/BurntSushi/ripgrep/issues/559
    if dir.try_create(badutf8, "test").is_ok() {
        cmd.arg("-H").arg("test").arg(badutf8);
        assert_eq!(b"foo\xffbar:test\n".to_vec(), cmd.output().stdout);
    }
});

// See: https://github.com/BurntSushi/ripgrep/issues/228
rgtest!(r228, |dir: Dir, mut cmd: TestCommand| {
    dir.create_dir("foo");

    cmd.arg("--ignore-file").arg("foo").arg("test").assert_err();
});

// See: https://github.com/BurntSushi/ripgrep/issues/229
rgtest!(r229, |dir: Dir, mut cmd: TestCommand| {
    dir.create("foo", "economie");

    cmd.arg("-S").arg("[E]conomie").assert_err();
});

// See: https://github.com/BurntSushi/ripgrep/issues/251
rgtest!(r251, |dir: Dir, mut cmd: TestCommand| {
    dir.create("foo", "привет\nПривет\nПрИвЕт");

    let expected = "foo:привет\nfoo:Привет\nfoo:ПрИвЕт\n";
    eqnice!(expected, cmd.arg("-i").arg("привет").stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/256
#[cfg(not(windows))]
rgtest!(r256, |dir: Dir, mut cmd: TestCommand| {
    dir.create_dir("bar");
    dir.create("bar/baz", "test");
    dir.link_dir("bar", "foo");

    eqnice!("foo/baz:test\n", cmd.arg("test").arg("foo").stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/256
#[cfg(not(windows))]
rgtest!(r256_j1, |dir: Dir, mut cmd: TestCommand| {
    dir.create_dir("bar");
    dir.create("bar/baz", "test");
    dir.link_dir("bar", "foo");

    eqnice!("foo/baz:test\n", cmd.arg("-j1").arg("test").arg("foo").stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/270
rgtest!(r270, |dir: Dir, mut cmd: TestCommand| {
    dir.create("foo", "-test");

    cmd.arg("-e").arg("-test");
    eqnice!("foo:-test\n", cmd.stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/279
rgtest!(r279, |dir: Dir, mut cmd: TestCommand| {
    dir.create("foo", "test");

    eqnice!("", cmd.arg("-q").arg("test").stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/391
rgtest!(r391, |dir: Dir, mut cmd: TestCommand| {
    dir.create_dir(".git");
    dir.create("lock", "");
    dir.create("bar.py", "");
    dir.create(".git/packed-refs", "");
    dir.create(".git/description", "");

    cmd.args(&[
        "--no-ignore",
        "--hidden",
        "--follow",
        "--files",
        "--glob",
        "!{.git,node_modules,plugged}/**",
        "--glob",
        "*.{js,json,php,md,styl,scss,sass,pug,html,config,py,cpp,c,go,hs}",
    ]);
    eqnice!("bar.py\n", cmd.stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/405
rgtest!(r405, |dir: Dir, mut cmd: TestCommand| {
    dir.create_dir("foo/bar");
    dir.create_dir("bar/foo");
    dir.create("foo/bar/file1.txt", "test");
    dir.create("bar/foo/file2.txt", "test");

    cmd.arg("-g").arg("!/foo/**").arg("test");
    eqnice!("bar/foo/file2.txt:test\n", cmd.stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/428
#[cfg(not(windows))]
rgtest!(r428_color_context_path, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", "foo\nbar");
    cmd.args(&[
        "-A1",
        "-H",
        "--no-heading",
        "-N",
        "--colors=match:none",
        "--color=always",
        "--hyperlink-format=",
        "foo",
    ]);

    let expected = format!(
        "{colored_path}:foo\n{colored_path}-bar\n",
        colored_path =
            "\x1b\x5b\x30\x6d\x1b\x5b\x33\x35\x6dsherlock\x1b\x5b\x30\x6d"
    );
    eqnice!(expected, cmd.stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/428
rgtest!(r428_unrecognized_style, |dir: Dir, mut cmd: TestCommand| {
    dir.create("file.txt", "Sherlock");

    cmd.arg("--colors=match:style:").arg("Sherlock");
    cmd.assert_err();

    let output = cmd.raw_output();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let expected = "\
rg: error parsing flag --colors: \
unrecognized style attribute ''. Choose from: nobold, bold, nointense, \
intense, nounderline, underline, noitalic, italic.
";
    eqnice!(expected, stderr);
});

// See: https://github.com/BurntSushi/ripgrep/issues/451
rgtest!(r451_only_matching_as_in_issue, |dir: Dir, mut cmd: TestCommand| {
    dir.create("digits.txt", "1 2 3\n");
    cmd.arg("--only-matching").arg(r"[0-9]+").arg("digits.txt");

    let expected = "\
1
2
3
";
    eqnice!(expected, cmd.stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/451
rgtest!(r451_only_matching, |dir: Dir, mut cmd: TestCommand| {
    dir.create("digits.txt", "1 2 3\n123\n");
    cmd.args(&["--only-matching", "--column", r"[0-9]", "digits.txt"]);

    let expected = "\
1:1:1
1:3:2
1:5:3
2:1:1
2:2:2
2:3:3
";
    eqnice!(expected, cmd.stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/483
rgtest!(r483_matching_no_stdout, |dir: Dir, mut cmd: TestCommand| {
    dir.create("file.py", "");
    cmd.arg("--quiet").arg("--files").arg("--glob").arg("*.py");
    eqnice!("", cmd.stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/483
rgtest!(r483_non_matching_exit_code, |dir: Dir, mut cmd: TestCommand| {
    dir.create("file.rs", "");
    cmd.arg("--quiet").arg("--files").arg("--glob").arg("*.py");
    cmd.assert_err();
});

// See: https://github.com/BurntSushi/ripgrep/issues/493
rgtest!(r493, |dir: Dir, mut cmd: TestCommand| {
    dir.create("input.txt", "peshwaship 're seminomata");

    cmd.arg("-o").arg(r"\b 're \b").arg("input.txt");
    assert_eq!(" 're \n", cmd.stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/506
rgtest!(r506_word_not_parenthesized, |dir: Dir, mut cmd: TestCommand| {
    dir.create("wb.txt", "min minimum amin\nmax maximum amax");
    cmd.arg("-w").arg("-o").arg("min|max").arg("wb.txt");
    eqnice!("min\nmax\n", cmd.stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/553
rgtest!(r553_switch, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);

    let expected = "\
sherlock:For the Doctor Watsons of this world, as opposed to the Sherlock
sherlock:be, to a very large extent, the result of luck. Sherlock Holmes
";
    cmd.arg("-i").arg("sherlock");
    eqnice!(expected, cmd.stdout());

    // Repeat the `i` flag to make sure everything still works.
    eqnice!(expected, cmd.arg("-i").stdout());
});

rgtest!(r553_flag, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);

    let expected = "\
For the Doctor Watsons of this world, as opposed to the Sherlock
Holmeses, success in the province of detective work must always
--
but Doctor Watson has to have it taken out for him and dusted,
and exhibited clearly, with a label attached.
";
    cmd.arg("-C").arg("1").arg(r"world|attached").arg("sherlock");
    eqnice!(expected, cmd.stdout());

    let expected = "\
For the Doctor Watsons of this world, as opposed to the Sherlock
and exhibited clearly, with a label attached.
";
    eqnice!(expected, cmd.arg("-C").arg("0").stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/568
rgtest!(r568_leading_hyphen_option_args, |dir: Dir, mut cmd: TestCommand| {
    dir.create("file", "foo bar -baz\n");
    cmd.arg("-e-baz").arg("-e").arg("-baz").arg("file");
    eqnice!("foo bar -baz\n", cmd.stdout());

    let mut cmd = dir.command();
    cmd.arg("-rni").arg("bar").arg("file");
    eqnice!("foo ni -baz\n", cmd.stdout());

    let mut cmd = dir.command();
    cmd.arg("-r").arg("-n").arg("-i").arg("bar").arg("file");
    eqnice!("foo -n -baz\n", cmd.stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/599
//
// This test used to check that we emitted color escape sequences even for
// empty matches, but with the addition of the JSON output format, clients no
// longer need to rely on escape sequences to parse matches. Therefore, we no
// longer emit useless escape sequences.
rgtest!(r599, |dir: Dir, mut cmd: TestCommand| {
    dir.create("input.txt", "\n\ntest\n");
    cmd.args(&[
        "--color",
        "ansi",
        "--colors",
        "path:none",
        "--colors",
        "line:none",
        "--colors",
        "match:fg:red",
        "--colors",
        "match:style:nobold",
        "--line-number",
        r"^$",
        "input.txt",
    ]);

    let expected = "\
[0m1[0m:
[0m2[0m:
";
    eqnice_repr!(expected, cmd.stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/693
rgtest!(r693_context_in_contextless_mode, |dir: Dir, mut cmd: TestCommand| {
    dir.create("foo", "xyz\n");
    dir.create("bar", "xyz\n");

    cmd.arg("-C1").arg("-c").arg("--sort-files").arg("xyz");
    eqnice!("bar:1\nfoo:1\n", cmd.stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/807
rgtest!(r807, |dir: Dir, mut cmd: TestCommand| {
    dir.create_dir(".git");
    dir.create(".gitignore", ".a/b");
    dir.create_dir(".a/b");
    dir.create_dir(".a/c");
    dir.create(".a/b/file", "test");
    dir.create(".a/c/file", "test");

    eqnice!(".a/c/file:test\n", cmd.arg("--hidden").arg("test").stdout());
});

// See: https://github.com/BurntSushi/ripgrep/pull/2711
//
// Note that this isn't a regression test. In particular, this didn't fail
// with ripgrep 14.1.1. I couldn't figure out how to turn what the OP gave me
// into a failing test.
rgtest!(r2711, |dir: Dir, _cmd: TestCommand| {
    dir.create_dir("a/b");
    dir.create("a/.ignore", ".foo");
    dir.create("a/b/.foo", "");

    {
        let mut cmd = dir.command();
        eqnice!("a/.ignore\n", cmd.arg("--hidden").arg("--files").stdout());
    }
    {
        let mut cmd = dir.command();
        eqnice!(
            "./a/.ignore\n",
            cmd.arg("--hidden").arg("--files").arg("./").stdout()
        );
    }

    {
        let mut cmd = dir.command();
        eqnice!(
            "a/.ignore\n",
            cmd.arg("--hidden").arg("--files").arg("a").stdout()
        );
    }
    {
        let mut cmd = dir.command();
        cmd.arg("--hidden").arg("--files").arg("a/b").assert_err();
    }
    {
        let mut cmd = dir.command();
        eqnice!(
            "./a/.ignore\n",
            cmd.arg("--hidden").arg("--files").arg("./a").stdout()
        );
    }

    {
        let mut cmd = dir.command();
        cmd.current_dir(dir.path().join("a"));
        eqnice!(".ignore\n", cmd.arg("--hidden").arg("--files").stdout());
    }
    {
        let mut cmd = dir.command();
        cmd.current_dir(dir.path().join("a").join("b"));
        cmd.arg("--hidden").arg("--files").assert_err();
    }
    {
        let mut cmd = dir.command();
        cmd.current_dir(dir.path().join("./a"));
        eqnice!(".ignore\n", cmd.arg("--hidden").arg("--files").stdout());
    }
});

// See: https://github.com/BurntSushi/ripgrep/issues/829
rgtest!(r829_original, |dir: Dir, _cmd: TestCommand| {
    dir.create_dir("a/b");
    dir.create(".ignore", "/a/b");
    dir.create("a/b/test.txt", "Sample text");

    {
        let mut cmd = dir.command();
        cmd.args(&["Sample"]).assert_err();
    }
    {
        let mut cmd = dir.command();
        cmd.args(&["Sample", "a"]).assert_err();
    }
    {
        let mut cmd = dir.command();
        cmd.current_dir(dir.path().join("a"));
        cmd.args(&["Sample"]).assert_err();
    }
});

// See: https://github.com/BurntSushi/ripgrep/issues/2731
rgtest!(r829_2731, |dir: Dir, _cmd: TestCommand| {
    dir.create_dir("some_dir/build");
    dir.create("some_dir/build/foo", "string");
    dir.create(".ignore", "build/\n!/some_dir/build/");

    {
        let mut cmd = dir.command();
        eqnice!("some_dir/build/foo\n", cmd.arg("-l").arg("string").stdout());
    }
    {
        let mut cmd = dir.command();
        eqnice!(
            "some_dir/build/foo\n",
            cmd.arg("-l").arg("string").arg("some_dir").stdout()
        );
    }
    {
        let mut cmd = dir.command();
        eqnice!(
            "./some_dir/build/foo\n",
            cmd.arg("-l").arg("string").arg("./some_dir").stdout()
        );
    }
    {
        let mut cmd = dir.command();
        eqnice!(
            "some_dir/build/foo\n",
            cmd.arg("-l").arg("string").arg("some_dir/build").stdout()
        );
    }
    {
        let mut cmd = dir.command();
        eqnice!(
            "./some_dir/build/foo\n",
            cmd.arg("-l").arg("string").arg("./some_dir/build").stdout()
        );
    }
});

// See: https://github.com/BurntSushi/ripgrep/issues/2747
rgtest!(r829_2747, |dir: Dir, _cmd: TestCommand| {
    dir.create_dir("a/c/b");
    dir.create_dir("a/src/f/b");
    dir.create("a/c/b/foo", "");
    dir.create("a/src/f/b/foo", "");
    dir.create(".ignore", "/a/*/b");

    {
        let mut cmd = dir.command();
        eqnice!("a/src/f/b/foo\n", cmd.arg("--files").stdout());
    }
    {
        let mut cmd = dir.command();
        eqnice!("a/src/f/b/foo\n", cmd.arg("--files").arg("a/src").stdout());
    }
    {
        let mut cmd = dir.command();
        cmd.current_dir(dir.path().join("a/src"));
        eqnice!("f/b/foo\n", cmd.arg("--files").stdout());
    }
});

// See: https://github.com/BurntSushi/ripgrep/issues/2778
rgtest!(r829_2778, |dir: Dir, _cmd: TestCommand| {
    dir.create_dir("parent/subdir");
    dir.create(".ignore", "/parent/*.txt");
    dir.create("parent/ignore-me.txt", "");
    dir.create("parent/subdir/dont-ignore-me.txt", "");

    {
        let mut cmd = dir.command();
        eqnice!(
            "parent/subdir/dont-ignore-me.txt\n",
            cmd.arg("--files").stdout()
        );
    }
    {
        let mut cmd = dir.command();
        cmd.current_dir(dir.path().join("parent"));
        eqnice!("subdir/dont-ignore-me.txt\n", cmd.arg("--files").stdout());
    }
});

// See: https://github.com/BurntSushi/ripgrep/issues/2836
rgtest!(r829_2836, |dir: Dir, _cmd: TestCommand| {
    dir.create_dir("testdir/sub/sub2");
    dir.create(".ignore", "/testdir/sub/sub2/\n");
    dir.create("testdir/sub/sub2/foo", "");

    {
        let mut cmd = dir.command();
        cmd.arg("--files").assert_err();
    }
    {
        let mut cmd = dir.command();
        cmd.current_dir(dir.path().join("testdir"));
        cmd.arg("--files").assert_err();
    }
});

// See: https://github.com/BurntSushi/ripgrep/pull/2933
rgtest!(r829_2933, |dir: Dir, mut cmd: TestCommand| {
    dir.create_dir("testdir/sub/sub2");
    dir.create(".ignore", "/testdir/sub/sub2/");
    dir.create("testdir/sub/sub2/testfile", "needle");

    let args = &["--files-with-matches", "needle"];
    cmd.current_dir(dir.path().join("testdir"));
    cmd.args(args).assert_err();
});

// See: https://github.com/BurntSushi/ripgrep/issues/900
rgtest!(r900, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    dir.create("pat", "");

    cmd.arg("-fpat").arg("sherlock").assert_err();
});

// See: https://github.com/BurntSushi/ripgrep/issues/1064
rgtest!(r1064, |dir: Dir, mut cmd: TestCommand| {
    dir.create("input", "abc");
    eqnice!("input:abc\n", cmd.arg("a(.*c)").stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/1174
rgtest!(r1098, |dir: Dir, mut cmd: TestCommand| {
    dir.create_dir(".git");
    dir.create(".gitignore", "a**b");
    dir.create("afoob", "test");
    cmd.arg("test").assert_err();
});

// See: https://github.com/BurntSushi/ripgrep/issues/1130
rgtest!(r1130, |dir: Dir, mut cmd: TestCommand| {
    dir.create("foo", "test");
    eqnice!(
        "foo\n",
        cmd.arg("--files-with-matches").arg("test").arg("foo").stdout()
    );

    let mut cmd = dir.command();
    eqnice!(
        "foo\n",
        cmd.arg("--files-without-match").arg("nada").arg("foo").stdout()
    );
});

// See: https://github.com/BurntSushi/ripgrep/issues/1159
rgtest!(r1159_invalid_flag, |_: Dir, mut cmd: TestCommand| {
    cmd.arg("--wat").assert_exit_code(2);
});

// See: https://github.com/BurntSushi/ripgrep/issues/1159
rgtest!(r1159_exit_status, |dir: Dir, _: TestCommand| {
    dir.create("foo", "test");

    // search with a match gets 0 exit status.
    let mut cmd = dir.command();
    cmd.arg("test").assert_exit_code(0);

    // search with --quiet and a match gets 0 exit status.
    let mut cmd = dir.command();
    cmd.arg("-q").arg("test").assert_exit_code(0);

    // search with a match and an error gets 2 exit status.
    let mut cmd = dir.command();
    cmd.arg("test").arg("no-file").assert_exit_code(2);

    // search with a match in --quiet mode and an error gets 0 exit status.
    let mut cmd = dir.command();
    cmd.arg("-q").arg("test").arg("foo").arg("no-file").assert_exit_code(0);

    // search with no match gets 1 exit status.
    let mut cmd = dir.command();
    cmd.arg("nada").assert_exit_code(1);

    // search with --quiet and no match gets 1 exit status.
    let mut cmd = dir.command();
    cmd.arg("-q").arg("nada").assert_exit_code(1);

    // search with no match and an error gets 2 exit status.
    let mut cmd = dir.command();
    cmd.arg("nada").arg("no-file").assert_exit_code(2);

    // search with no match in --quiet mode and an error gets 2 exit status.
    let mut cmd = dir.command();
    cmd.arg("-q").arg("nada").arg("foo").arg("no-file").assert_exit_code(2);
});

// See: https://github.com/BurntSushi/ripgrep/issues/1163
rgtest!(r1163, |dir: Dir, mut cmd: TestCommand| {
    dir.create("bom.txt", "\u{FEFF}test123\ntest123");
    eqnice!(
        "bom.txt:test123\nbom.txt:test123\n",
        cmd.arg("^test123").stdout()
    );
});

// See: https://github.com/BurntSushi/ripgrep/issues/1164
rgtest!(r1164, |dir: Dir, mut cmd: TestCommand| {
    dir.create_dir(".git");
    dir.create(".gitignore", "myfile");
    dir.create("MYFILE", "test");

    cmd.arg("--ignore-file-case-insensitive").arg("test").assert_err();
    eqnice!(
        "MYFILE:test\n",
        cmd.arg("--no-ignore-file-case-insensitive").stdout()
    );
});

// See: https://github.com/BurntSushi/ripgrep/issues/1173
rgtest!(r1173, |dir: Dir, mut cmd: TestCommand| {
    dir.create_dir(".git");
    dir.create(".gitignore", "**");
    dir.create("foo", "test");
    cmd.arg("test").assert_err();
});

// See: https://github.com/BurntSushi/ripgrep/issues/1174
rgtest!(r1174, |dir: Dir, mut cmd: TestCommand| {
    dir.create_dir(".git");
    dir.create(".gitignore", "**/**/*");
    dir.create_dir("a");
    dir.create("a/foo", "test");
    cmd.arg("test").assert_err();
});

// See: https://github.com/BurntSushi/ripgrep/issues/1176
rgtest!(r1176_literal_file, |dir: Dir, mut cmd: TestCommand| {
    dir.create("patterns", "foo(bar\n");
    dir.create("test", "foo(bar");

    eqnice!(
        "foo(bar\n",
        cmd.arg("-F").arg("-f").arg("patterns").arg("test").stdout()
    );
});

// See: https://github.com/BurntSushi/ripgrep/issues/1176
rgtest!(r1176_line_regex, |dir: Dir, mut cmd: TestCommand| {
    dir.create("patterns", "foo\n");
    dir.create("test", "foobar\nfoo\nbarfoo\n");

    eqnice!(
        "foo\n",
        cmd.arg("-x").arg("-f").arg("patterns").arg("test").stdout()
    );
});

// See: https://github.com/BurntSushi/ripgrep/issues/1203
rgtest!(r1203_reverse_suffix_literal, |dir: Dir, _: TestCommand| {
    dir.create("test", "153.230000\n");

    let mut cmd = dir.command();
    eqnice!("153.230000\n", cmd.arg(r"\d\d\d00").arg("test").stdout());

    let mut cmd = dir.command();
    eqnice!("153.230000\n", cmd.arg(r"\d\d\d000").arg("test").stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/1223
rgtest!(
    r1223_no_dir_check_for_default_path,
    |dir: Dir, mut cmd: TestCommand| {
        dir.create_dir("-");
        dir.create("a.json", "{}");
        dir.create("a.txt", "some text");

        eqnice!(
            "a.json\na.txt\n",
            sort_lines(&cmd.arg("a").pipe(b"a.json\na.txt"))
        );
    }
);

// See: https://github.com/BurntSushi/ripgrep/issues/1259
rgtest!(r1259_drop_last_byte_nonl, |dir: Dir, mut cmd: TestCommand| {
    dir.create("patterns-nonl", "[foo]");
    dir.create("patterns-nl", "[foo]\n");
    dir.create("test", "fz");

    eqnice!("fz\n", cmd.arg("-f").arg("patterns-nonl").arg("test").stdout());
    cmd = dir.command();
    eqnice!("fz\n", cmd.arg("-f").arg("patterns-nl").arg("test").stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/1311
rgtest!(r1311_multi_line_term_replace, |dir: Dir, mut cmd: TestCommand| {
    dir.create("input", "hello\nworld\n");
    eqnice!(
        "1:hello?world?\n",
        cmd.args(&["-U", "-r?", "-n", "\n", "input"]).stdout()
    );
});

// See: https://github.com/BurntSushi/ripgrep/issues/1319
rgtest!(r1319, |dir: Dir, mut cmd: TestCommand| {
    dir.create("input", "CCAGCTACTCGGGAGGCTGAGGCTGGAGGATCGCTTGAGTCCAGGAGTTC");
    eqnice!(
        "input:CCAGCTACTCGGGAGGCTGAGGCTGGAGGATCGCTTGAGTCCAGGAGTTC\n",
        cmd.arg("TTGAGTCCAGGAG[ATCG]{2}C").stdout()
    );
});

// See: https://github.com/BurntSushi/ripgrep/issues/1332
rgtest!(r1334_invert_empty_patterns, |dir: Dir, _cmd: TestCommand| {
    dir.create("zero-patterns", "");
    dir.create("one-pattern", "\n");
    dir.create("haystack", "one\ntwo\nthree\n");

    // zero patterns matches nothing
    {
        let mut cmd = dir.command();
        cmd.arg("-f").arg("zero-patterns").arg("haystack").assert_err();
    }
    // one pattern that matches empty string matches everything
    {
        let mut cmd = dir.command();
        eqnice!(
            "one\ntwo\nthree\n",
            cmd.arg("-f").arg("one-pattern").arg("haystack").stdout()
        );
    }

    // inverting zero patterns matches everything
    // (This is the regression. ripgrep used to match nothing because of an
    // incorrect optimization.)
    {
        let mut cmd = dir.command();
        eqnice!(
            "one\ntwo\nthree\n",
            cmd.arg("-vf").arg("zero-patterns").arg("haystack").stdout()
        );
    }
    // inverting one pattern that matches empty string matches nothing
    {
        let mut cmd = dir.command();
        cmd.arg("-vf").arg("one-pattern").arg("haystack").assert_err();
    }
});

// See: https://github.com/BurntSushi/ripgrep/issues/1334
rgtest!(r1334_crazy_literals, |dir: Dir, mut cmd: TestCommand| {
    dir.create("patterns", &"1.208.0.0/12\n".repeat(40));
    dir.create("corpus", "1.208.0.0/12\n");
    eqnice!(
        "1.208.0.0/12\n",
        cmd.arg("-Ff").arg("patterns").arg("corpus").stdout()
    );
});

// See: https://github.com/BurntSushi/ripgrep/issues/1380
rgtest!(r1380, |dir: Dir, mut cmd: TestCommand| {
    dir.create(
        "foo",
        "\
a
b
c
d
e
d
e
d
e
d
e
",
    );

    eqnice!("d\ne\nd\n", cmd.args(&["-A2", "-m1", "d", "foo"]).stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/1389
rgtest!(r1389_bad_symlinks_no_biscuit, |dir: Dir, mut cmd: TestCommand| {
    dir.create_dir("mydir");
    dir.create("mydir/file.txt", "test");
    dir.link_dir("mydir", "mylink");

    let stdout = cmd
        .args(&["test", "--no-ignore", "--sort", "path", "mylink"])
        .stdout();
    eqnice!("mylink/file.txt:test\n", stdout);
});

// See: https://github.com/BurntSushi/ripgrep/issues/1401
rgtest!(r1401_look_ahead_only_matching_1, |dir: Dir, mut cmd: TestCommand| {
    // Only PCRE2 supports look-around.
    if !dir.is_pcre2() {
        return;
    }
    dir.create("ip.txt", "foo 42\nxoyz\ncat\tdog\n");
    cmd.args(&["-No", r".*o(?!.*\s)", "ip.txt"]);
    eqnice!("xo\ncat\tdo\n", cmd.stdout());

    let mut cmd = dir.command();
    cmd.args(&["-No", r".*o(?!.*[ \t])", "ip.txt"]);
    eqnice!("xo\ncat\tdo\n", cmd.stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/1401
rgtest!(r1401_look_ahead_only_matching_2, |dir: Dir, mut cmd: TestCommand| {
    // Only PCRE2 supports look-around.
    if !dir.is_pcre2() {
        return;
    }
    dir.create("ip.txt", "foo 42\nxoyz\ncat\tdog\nfoo");
    cmd.args(&["-No", r".*o(?!.*\s)", "ip.txt"]);
    eqnice!("xo\ncat\tdo\nfoo\n", cmd.stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/1412
rgtest!(r1412_look_behind_no_replacement, |dir: Dir, mut cmd: TestCommand| {
    // Only PCRE2 supports look-around.
    if !dir.is_pcre2() {
        return;
    }

    dir.create("test", "foo\nbar\n");
    cmd.args(&["-nU", "-rquux", r"(?<=foo\n)bar", "test"]);
    eqnice!("2:quux\n", cmd.stdout());
});

// See: https://github.com/BurntSushi/ripgrep/pull/1446
rgtest!(
    r1446_respect_excludes_in_worktree,
    |dir: Dir, mut cmd: TestCommand| {
        dir.create_dir("repo/.git/info");
        dir.create("repo/.git/info/exclude", "ignored");
        dir.create_dir("repo/.git/worktrees/repotree");
        dir.create("repo/.git/worktrees/repotree/commondir", "../..");

        dir.create_dir("repotree");
        dir.create("repotree/.git", "gitdir: repo/.git/worktrees/repotree");
        dir.create("repotree/ignored", "");
        dir.create("repotree/not-ignored", "");

        cmd.arg("--sort").arg("path").arg("--files").arg("repotree");
        eqnice!("repotree/not-ignored\n", cmd.stdout());
    }
);

// See: https://github.com/BurntSushi/ripgrep/issues/1537
rgtest!(r1537, |dir: Dir, mut cmd: TestCommand| {
    dir.create("foo", "abc;de,fg");

    let expected = "foo:abc;de,fg\n";
    eqnice!(expected, cmd.arg(";(.*,){1}").stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/1559
rgtest!(r1559, |dir: Dir, mut cmd: TestCommand| {
    dir.create(
        "foo",
        "\
type A struct {
	TaskID int `json:\"taskID\"`
}

type B struct {
	ObjectID string `json:\"objectID\"`
	TaskID   int    `json:\"taskID\"`
}
",
    );

    let expected = "\
foo:	TaskID int `json:\"taskID\"`
foo:	TaskID   int    `json:\"taskID\"`
";
    eqnice!(expected, cmd.arg("TaskID +int").stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/1573
//
// Tests that if look-ahead is used, then --count-matches is correct.
rgtest!(r1573, |dir: Dir, mut cmd: TestCommand| {
    // Only PCRE2 supports look-ahead.
    if !dir.is_pcre2() {
        return;
    }

    dir.create_bytes("foo", b"\xFF\xFE\x00\x62");
    dir.create(
        "foo",
        "\
def A;
def B;
use A;
use B;
",
    );

    // Check that normal --count is correct.
    cmd.args(&[
        "--pcre2",
        "--multiline",
        "--count",
        r"(?s)def (\w+);(?=.*use \w+)",
        "foo",
    ]);
    eqnice!("2\n", cmd.stdout());

    // Now check --count-matches.
    let mut cmd = dir.command();
    cmd.args(&[
        "--pcre2",
        "--multiline",
        "--count-matches",
        r"(?s)def (\w+);(?=.*use \w+)",
        "foo",
    ]);
    eqnice!("2\n", cmd.stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/1638
//
// Tests if UTF-8 BOM is sniffed, then the column index is correct.
rgtest!(r1638, |dir: Dir, mut cmd: TestCommand| {
    dir.create_bytes("foo", b"\xef\xbb\xbfx");

    eqnice!("foo:1:1:x\n", cmd.arg("--column").arg("x").stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/1739
rgtest!(r1739_replacement_lineterm_match, |dir: Dir, mut cmd: TestCommand| {
    dir.create("test", "a\n");
    cmd.args(&[r"-r${0}f", r".*", "test"]);
    eqnice!("af\n", cmd.stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/1757
rgtest!(f1757, |dir: Dir, _: TestCommand| {
    dir.create_dir("rust/target");
    dir.create(".ignore", "rust/target");
    dir.create("rust/source.rs", "needle");
    dir.create("rust/target/rustdoc-output.html", "needle");

    let args = &["--files-with-matches", "needle", "rust"];
    eqnice!("rust/source.rs\n", dir.command().args(args).stdout());
    let args = &["--files-with-matches", "needle", "./rust"];
    eqnice!("./rust/source.rs\n", dir.command().args(args).stdout());

    dir.create_dir("rust1/target/onemore");
    dir.create(".ignore", "rust1/target/onemore");
    dir.create("rust1/source.rs", "needle");
    dir.create("rust1/target/onemore/rustdoc-output.html", "needle");
    let args = &["--files-with-matches", "needle", "rust1"];
    eqnice!("rust1/source.rs\n", dir.command().args(args).stdout());
    let args = &["--files-with-matches", "needle", "./rust1"];
    eqnice!("./rust1/source.rs\n", dir.command().args(args).stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/1765
rgtest!(r1765, |dir: Dir, mut cmd: TestCommand| {
    dir.create("test", "\n");
    // We need to add --color=always here to force the failure, since the bad
    // code path is only triggered when colors are enabled.
    cmd.args(&[r"x?", "--crlf", "--color", "always"]);

    assert!(!cmd.stdout().is_empty());
});

// See: https://github.com/BurntSushi/ripgrep/issues/1838
rgtest!(r1838_nul_error_with_binary_detection, |dir: Dir, _: TestCommand| {
    // We don't support this error reporting with PCRE2 since we can't parse
    // the pattern (easily) to give a good error message.
    if dir.is_pcre2() {
        return;
    }
    dir.create("test", "foo\n");

    dir.command().args(&[r"foo\x00?"]).assert_err();
    eqnice!("test:foo\n", dir.command().args(&["-a", r"foo\x00?"]).stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/1866
rgtest!(r1866, |dir: Dir, mut cmd: TestCommand| {
    dir.create("test", "foobar\nfoobar\nfoo quux");
    cmd.args(&[
        "--multiline",
        "--vimgrep",
        r"foobar\nfoobar\nfoo|quux",
        "test",
    ]);

    // vimgrep only wants the first line of each match, even when a match
    // spans multiple lines.
    //
    // See: https://github.com/BurntSushi/ripgrep/issues/1866
    let expected = "\
test:1:1:foobar
test:3:5:foo quux
";
    eqnice!(expected, cmd.stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/1868
rgtest!(r1868_context_passthru_override, |dir: Dir, _: TestCommand| {
    dir.create("test", "foo\nbar\nbaz\nquux\n");

    let args = &["-C1", "bar", "test"];
    eqnice!("foo\nbar\nbaz\n", dir.command().args(args).stdout());
    let args = &["--passthru", "bar", "test"];
    eqnice!("foo\nbar\nbaz\nquux\n", dir.command().args(args).stdout());

    let args = &["--passthru", "-C1", "bar", "test"];
    eqnice!("foo\nbar\nbaz\n", dir.command().args(args).stdout());
    let args = &["-C1", "--passthru", "bar", "test"];
    eqnice!("foo\nbar\nbaz\nquux\n", dir.command().args(args).stdout());

    let args = &["--passthru", "-B1", "bar", "test"];
    eqnice!("foo\nbar\n", dir.command().args(args).stdout());
    let args = &["-B1", "--passthru", "bar", "test"];
    eqnice!("foo\nbar\nbaz\nquux\n", dir.command().args(args).stdout());

    let args = &["--passthru", "-A1", "bar", "test"];
    eqnice!("bar\nbaz\n", dir.command().args(args).stdout());
    let args = &["-A1", "--passthru", "bar", "test"];
    eqnice!("foo\nbar\nbaz\nquux\n", dir.command().args(args).stdout());
});

rgtest!(r1878, |dir: Dir, _: TestCommand| {
    dir.create("test", "a\nbaz\nabc\n");

    // Since ripgrep enables (?m) by default, '^' will match at the beginning
    // of a line, even when -U/--multiline is used.
    let args = &["-U", "--no-mmap", r"^baz", "test"];
    eqnice!("baz\n", dir.command().args(args).stdout());
    let args = &["-U", "--mmap", r"^baz", "test"];
    eqnice!("baz\n", dir.command().args(args).stdout());

    // But when (?-m) is disabled, or when \A is used, then there should be no
    // matches that aren't anchored to the beginning of the file.
    let args = &["-U", "--no-mmap", r"(?-m)^baz", "test"];
    dir.command().args(args).assert_err();
    let args = &["-U", "--mmap", r"(?-m)^baz", "test"];
    dir.command().args(args).assert_err();

    let args = &["-U", "--no-mmap", r"\Abaz", "test"];
    dir.command().args(args).assert_err();
    let args = &["-U", "--mmap", r"\Abaz", "test"];
    dir.command().args(args).assert_err();
});

// See: https://github.com/BurntSushi/ripgrep/issues/1891
rgtest!(r1891, |dir: Dir, mut cmd: TestCommand| {
    dir.create("test", "\n##\n");
    // N.B. We use -o here to force the issue to occur, which seems to only
    // happen when each match needs to be detected.
    eqnice!("1:\n2:\n2:\n2:\n", cmd.args(&["-won", "", "test"]).stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/2095
rgtest!(r2095, |dir: Dir, mut cmd: TestCommand| {
    dir.create(
        "test",
        "#!/usr/bin/env bash

zero=one

a=one

if true; then
	a=(
		a
		b
		c
	)
	true
fi

a=two

b=one
});
",
    );
    cmd.args(&[
        "--line-number",
        "--multiline",
        "--only-matching",
        "--replace",
        "${value}",
        r"^(?P<indent>\s*)a=(?P<value>(?ms:[(].*?[)])|.*?)$",
        "test",
    ]);
    let expected = "4:one
8:(
9:		a
10:		b
11:		c
12:	)
15:two
";
    eqnice!(expected, cmd.stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/2198
rgtest!(r2198, |dir: Dir, mut cmd: TestCommand| {
    dir.create(".ignore", "a");
    dir.create(".rgignore", "b");
    dir.create("a", "");
    dir.create("b", "");
    dir.create("c", "");

    cmd.arg("--files").arg("--sort").arg("path");
    eqnice!("c\n", cmd.stdout());
    eqnice!("a\nb\nc\n", cmd.arg("--no-ignore-dot").stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/2208
rgtest!(r2208, |dir: Dir, mut cmd: TestCommand| {
    dir.create("test", "# Compile requirements.txt files from all found or specified requirements.in files (compile).
# Use -h to include hashes, -u dep1,dep2... to upgrade specific dependencies, and -U to upgrade all.
pipc () {  # [-h] [-U|-u <pkgspec>[,<pkgspec>...]] [<reqs-in>...] [-- <pip-compile-arg>...]
    emulate -L zsh
    unset REPLY
    if [[ $1 == --help ]] { zpy $0; return }
    [[ $ZPY_PROCS ]] || return

    local gen_hashes upgrade upgrade_csv
    while [[ $1 == -[hUu] ]] {
        if [[ $1 == -h ]] { gen_hashes=--generate-hashes; shift   }
        if [[ $1 == -U ]] { upgrade=1;                    shift   }
        if [[ $1 == -u ]] { upgrade=1; upgrade_csv=$2;    shift 2 }
    }
}
");
    cmd.args(&[
        "-N",
        "-U",
        "-r", "$usage",
        r#"^(?P<predoc>\n?(# .*\n)*)(alias (?P<aname>pipc)="[^"]+"|(?P<fname>pipc) \(\) \{)(  #(?P<usage> .+))?"#,
        "test",
    ]);
    let expected = " [-h] [-U|-u <pkgspec>[,<pkgspec>...]] [<reqs-in>...] [-- <pip-compile-arg>...]\n";
    eqnice!(expected, cmd.stdout());
});

// See: https://github.com/BurntSushi/ripgrep/issues/2236
rgtest!(r2236, |dir: Dir, mut cmd: TestCommand| {
    dir.create(".ignore", r"foo\/");
    dir.create_dir("foo");
    dir.create("foo/bar", "test\n");
    cmd.args(&["test"]).assert_err();
});

// See: https://github.com/BurntSushi/ripgrep/issues/2480
rgtest!(r2480, |dir: Dir, mut cmd: TestCommand| {
    dir.create("file", "FooBar\n");

    // no regression in empty pattern behavior
    cmd.args(&["-e", "", "file"]);
    eqnice!("FooBar\n", cmd.stdout());

    // no regression in single pattern behavior
    let mut cmd = dir.command();
    cmd.args(&["-e", ")(", "file"]);
    eqnice!("FooBar\n", cmd.stdout());

    // no regression in multiple patterns behavior
    let mut cmd = dir.command();
    cmd.args(&["--only-matching", "-e", "Foo", "-e", "Bar", "file"]);
    eqnice!("Foo\nBar\n", cmd.stdout());

    // no regression in capture groups behavior
    let mut cmd = dir.command();
    cmd.args(&["-e", "Fo(oB)a(r)", "--replace", "${0}_${1}_${2}${3}", "file"]);
    eqnice!("FooBar_oB_r\n", cmd.stdout()); // note: ${3} expected to be empty

    // flag does not leak into next pattern on match
    let mut cmd = dir.command();
    cmd.args(&["--only-matching", "-e", "(?i)foo", "-e", "bar", "file"]);
    eqnice!("Foo\n", cmd.stdout());

    // flag does not leak into next pattern on mismatch
    let mut cmd = dir.command();
    cmd.args(&["--only-matching", "-e", "(?i)notfoo", "-e", "bar", "file"]);
    cmd.assert_err();
});

// See: https://github.com/BurntSushi/ripgrep/issues/2574
rgtest!(r2574, |dir: Dir, mut cmd: TestCommand| {
    dir.create("haystack", "some.domain.com\nsome.domain.com/x\n");
    let got = cmd
        .args(&[
            "--no-filename",
            "--no-unicode",
            "-w",
            "-o",
            r"(\w+\.)*domain\.(\w+)",
        ])
        .stdout();
    eqnice!("some.domain.com\nsome.domain.com\n", got);
});

// See: https://github.com/BurntSushi/ripgrep/issues/2658
rgtest!(r2658_null_data_line_regexp, |dir: Dir, mut cmd: TestCommand| {
    dir.create("haystack", "foo\0bar\0quux\0");
    let got = cmd.args(&["--null-data", "--line-regexp", r"bar"]).stdout();
    eqnice!("haystack:bar\0", got);
});

// See: https://github.com/BurntSushi/ripgrep/pull/2944
rgtest!(r2944_incorrect_bytes_searched, |dir: Dir, mut cmd: TestCommand| {
    dir.create("haystack", "foo1\nfoo2\nfoo3\nfoo4\nfoo5\n");
    let got = cmd.args(&["--stats", "-m2", "foo", "."]).stdout();
    println!("{got}");
    assert!(got.contains("10 bytes searched\n"));
});

// See: https://github.com/BurntSushi/ripgrep/issues/2990
#[cfg(unix)]
rgtest!(r2990_trip_over_trailing_dot, |dir: Dir, _cmd: TestCommand| {
    dir.create_dir("asdf");
    dir.create_dir("asdf.");
    dir.create("asdf/foo", "");
    dir.create("asdf./foo", "");

    let got = dir.command().args(&["--files", "-g", "!asdf/"]).stdout();
    eqnice!("asdf./foo\n", got);

    // This used to ignore the glob given and included `asdf./foo` in output.
    let got = dir.command().args(&["--files", "-g", "!asdf./"]).stdout();
    eqnice!("asdf/foo\n", got);
});

// See: https://github.com/BurntSushi/ripgrep/issues/3108
rgtest!(r3108_files_without_match_quiet_exit, |dir: Dir, _: TestCommand| {
    dir.create("yes-match", "abc");
    dir.create("non-match", "xyz");

    dir.command().args(&["-q", "abc", "non-match"]).assert_exit_code(1);
    dir.command().args(&["-q", "abc", "yes-match"]).assert_exit_code(0);
    dir.command()
        .args(&["--files-with-matches", "-q", "abc", "non-match"])
        .assert_exit_code(1);
    dir.command()
        .args(&["--files-with-matches", "-q", "abc", "yes-match"])
        .assert_exit_code(0);

    dir.command()
        .args(&["--files-without-match", "abc", "non-match"])
        .assert_exit_code(0);
    dir.command()
        .args(&["--files-without-match", "abc", "yes-match"])
        .assert_exit_code(1);

    let got = dir
        .command()
        .args(&["--files-without-match", "abc", "non-match"])
        .stdout();
    eqnice!("non-match\n", got);

    dir.command()
        .args(&["--files-without-match", "-q", "abc", "non-match"])
        .assert_exit_code(0);
    dir.command()
        .args(&["--files-without-match", "-q", "abc", "yes-match"])
        .assert_exit_code(1);

    let got = dir
        .command()
        .args(&["--files-without-match", "-q", "abc", "non-match"])
        .stdout();
    eqnice!("", got);
});

// See: https://github.com/BurntSushi/ripgrep/issues/3127
rgtest!(
    r3127_gitignore_allow_unclosed_class,
    |dir: Dir, mut cmd: TestCommand| {
        dir.create_dir(".git");
        dir.create(".gitignore", "[abc");
        dir.create("[abc", "");
        dir.create("test", "");

        let got = cmd.args(&["--files"]).stdout();
        eqnice!("test\n", got);
    }
);

// See: https://github.com/BurntSushi/ripgrep/issues/3127
rgtest!(
    r3127_glob_flag_not_allow_unclosed_class,
    |dir: Dir, mut cmd: TestCommand| {
        dir.create("[abc", "");
        dir.create("test", "");

        cmd.args(&["--files", "-g", "[abc"]).assert_err();
    }
);

// See: https://github.com/BurntSushi/ripgrep/issues/3139
rgtest!(
    r3139_multiline_lookahead_files_with_matches,
    |dir: Dir, _cmd: TestCommand| {
        // Only PCRE2 supports look-around.
        if !dir.is_pcre2() {
            return;
        }
        dir.create(
            "test",
            "\
Start \n   \n\n   \
XXXXXXXXXXXXXXXXXXXXXXXXXX\n   \
YYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY\n   \
\n      thing2 \n\n",
        );

        let got = dir
            .command()
            .args(&[
                "--multiline",
                "--pcre2",
                r"(?s)Start(?=.*thing2)",
                "test",
            ])
            .stdout();
        eqnice!("Start \n", got);

        let got = dir
            .command()
            .args(&[
                "--multiline",
                "--pcre2",
                "--files-with-matches",
                r"(?s)Start(?=.*thing2)",
                "test",
            ])
            .stdout();
        eqnice!("test\n", got);
    }
);
