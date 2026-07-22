use crate::util::{Dir, TestCommand};

rgtest!(read_no_index, |dir: Dir, _cmd: TestCommand| {
    dir.create("test", "foobar");
    dir.command().arg("-X").arg("foobar").assert_err();
});
