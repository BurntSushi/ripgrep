use crate::hay::SHERLOCK;
use crate::util::{Dir, TestCommand};

// This tests that multiline matches that span multiple lines, but where
// multiple matches may begin and end on the same line work correctly.
rgtest!(overlap1, |dir: Dir, mut cmd: TestCommand| {
    dir.create("test", "xxx\nabc\ndefxxxabc\ndefxxx\nxxx");
    cmd.arg("-n").arg("-U").arg("abc\ndef").arg("test");
    eqnice!("2:abc\n3:defxxxabc\n4:defxxx\n", cmd.stdout());
});

// Like overlap1, but tests the case where one match ends at precisely the same
// location at which the next match begins.
rgtest!(overlap2, |dir: Dir, mut cmd: TestCommand| {
    dir.create("test", "xxx\nabc\ndefabc\ndefxxx\nxxx");
    cmd.arg("-n").arg("-U").arg("abc\ndef").arg("test");
    eqnice!("2:abc\n3:defabc\n4:defxxx\n", cmd.stdout());
});

// Tests that even in a multiline search, a '.' does not match a newline.
rgtest!(dot_no_newline, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.args(&["-n", "-U", "of this world.+detective work", "sherlock"]);
    cmd.assert_err();
});

// Tests that the --multiline-dotall flag causes '.' to match a newline.
rgtest!(dot_all, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.args(&[
        "-n",
        "-U",
        "--multiline-dotall",
        "of this world.+detective work",
        "sherlock",
    ]);

    let expected = "\
1:For the Doctor Watsons of this world, as opposed to the Sherlock
2:Holmeses, success in the province of detective work must always
";
    eqnice!(expected, cmd.stdout());
});

// Tests that --only-matching works in multiline mode.
rgtest!(only_matching, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.args(&[
        "-n",
        "-U",
        "--only-matching",
        r"Watson|Sherlock\p{Any}+?Holmes",
        "sherlock",
    ]);

    let expected = "\
1:Watson
1:Sherlock
2:Holmes
3:Sherlock Holmes
5:Watson
";
    eqnice!(expected, cmd.stdout());
});

// Tests that --vimgrep works in multiline mode.
//
// In particular, we test that only the first line of each match is printed,
// even when a match spans multiple lines.
//
// See: https://github.com/BurntSushi/ripgrep/issues/1866
rgtest!(vimgrep, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.args(&[
        "-n",
        "-U",
        "--vimgrep",
        r"Watson|Sherlock\p{Any}+?Holmes",
        "sherlock",
    ]);

    let expected = "\
sherlock:1:16:For the Doctor Watsons of this world, as opposed to the Sherlock
sherlock:1:57:For the Doctor Watsons of this world, as opposed to the Sherlock
sherlock:3:49:be, to a very large extent, the result of luck. Sherlock Holmes
sherlock:5:12:but Doctor Watson has to have it taken out for him and dusted,
";
    eqnice!(expected, cmd.stdout());
});

// Tests that multiline search works when reading from stdin. This is an
// important test because multiline search must read the entire contents of
// what it is searching into memory before executing the search.
rgtest!(stdin, |_: Dir, mut cmd: TestCommand| {
    cmd.args(&["-n", "-U", r"of this world\p{Any}+?detective work"]);
    let expected = "\
1:For the Doctor Watsons of this world, as opposed to the Sherlock
2:Holmeses, success in the province of detective work must always
";
    eqnice!(expected, cmd.pipe(SHERLOCK.as_bytes()));
});

// Test that multiline search and contextual matches work.
rgtest!(context, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.args(&[
        "-n",
        "-U",
        "-C1",
        r"detective work\p{Any}+?result of luck",
        "sherlock",
    ]);

    let expected = "\
1-For the Doctor Watsons of this world, as opposed to the Sherlock
2:Holmeses, success in the province of detective work must always
3:be, to a very large extent, the result of luck. Sherlock Holmes
4-can extract a clew from a wisp of straw or a flake of cigar ash;
";
    eqnice!(expected, cmd.stdout());
});
