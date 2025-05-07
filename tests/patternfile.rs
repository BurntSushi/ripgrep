use crate::files::{NEWLINE, ONETWOTHREE};
use crate::hay::SHERLOCK;
use crate::util::{cmd_exists, sort_lines, Dir, TestCommand};

// This file contains specified to-fix tests.
// $ rg -f /dev/null <<< "wat"
// $ rg -v -f /dev/null <<< "wat"
// $ grep -f /dev/null <<< "wat"
// $ grep -v -f /dev/null <<< "wat"
// wat
// $ rg -f <(echo) <<< "wat"
// wat
// $ rg -v -f <(echo) <<< "wat"
//

rgtest!(empty_file, |dir: Dir, mut cmd: TestCommand| {
    dir.create("onetwothree", ONETWOTHREE);
    dir.create("empty", "");

    let expected = "\
one
two
three
";
    eqnice!(
        expected,
        cmd.args(&["-v", "-f", "onetwothree", "empty",]).stdout()
    );
});

rgtest!(new_line, |dir: Dir, mut cmd: TestCommand| {
    dir.create("onetwothree", ONETWOTHREE);
    dir.create("newline", NEWLINE);

    cmd.args(&["-v", "-f", "onetwothree", "newline"]);
    cmd.assert_err();
});
