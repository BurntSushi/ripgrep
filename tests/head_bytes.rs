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

rgtest!(head_bytes_stdin, |_dir: Dir, mut cmd: TestCommand| {
    let expected = "\
Sherlock
";
    eqnice!(
        expected,
        cmd.arg("--max-bytes=16")
            .arg("Sherlock")
            .arg("-")
            .pipe(b"Sherlock\nextra data that should be ignored\n")
    );
});

rgtest!(head_bytes_binary_quick, |dir: Dir, mut cmd: TestCommand| {
    // Create a file that looks binary (lots of NULs) but has a header match.
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"HEADER Sherlock\n");
    bytes.extend_from_slice(&[0u8; 4096]);
    dir.create_bytes("bin", &bytes);

    // With a small byte cap, we still see that it is a binary match and exit
    // quickly without scanning the entire file.
    cmd.arg("--max-bytes=64")
        .arg("Sherlock")
        .arg("bin")
        .assert_exit_code(0);
});

