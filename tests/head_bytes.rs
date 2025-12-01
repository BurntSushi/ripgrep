use crate::hay::SHERLOCK;
use crate::util::{Dir, TestCommand};

rgtest!(head_bytes_full, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);

    let expected = "\
For the Doctor Watsons of this world, as opposed to the Sherlock
";
    eqnice!(
        expected,
        cmd.arg("Sherlock")
            .arg("sherlock")
            .arg("--max-bytes=64")
            .stdout()
    );
});

rgtest!(head_bytes_limit, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);

    cmd.arg("-F")
        .arg("Sherlock")
        .arg("sherlock")
        .arg("--max-bytes=63")
        .assert_exit_code(1);
});

