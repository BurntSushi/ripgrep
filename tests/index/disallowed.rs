use crate::util::{Dir, TestCommand};

rgtest!(search_mode, |dir: Dir, _cmd: TestCommand| {
    dir.create("test", "foobar");
    dir.command().arg("--x-crud").assert_exit_code(0);

    dir.command()
        .arg("-X")
        .arg("--files-without-match")
        .arg("no-match")
        .assert_err();
});

rgtest!(binary, |dir: Dir, _cmd: TestCommand| {
    dir.create("test", "foobar");
    dir.command().arg("--x-crud").assert_exit_code(0);

    dir.command().arg("-X").arg("--text").arg("foobar").assert_err();
});

rgtest!(encoding, |dir: Dir, _cmd: TestCommand| {
    dir.create("test", "foobar");
    dir.command().arg("--x-crud").assert_exit_code(0);

    dir.command().arg("-X").arg("-Eutf16").arg("foobar").assert_err();
    dir.command().arg("-X").arg("-Enone").arg("foobar").assert_err();

    eqnice!(
        "test:foobar\n",
        dir.command().arg("-X").arg("-Eauto").arg("foobar").stdout()
    );
});

rgtest!(engine, |dir: Dir, _cmd: TestCommand| {
    dir.create("test", "foobar");
    dir.command().arg("--x-crud").assert_exit_code(0);

    dir.command().arg("-X").arg("--engine=pcre2").arg("foobar").assert_err();

    eqnice!(
        "test:foobar\n",
        dir.command().arg("-X").arg("--engine=auto").arg("foobar").stdout()
    );
    eqnice!(
        "test:foobar\n",
        dir.command().arg("-X").arg("--engine=default").arg("foobar").stdout()
    );
});

rgtest!(follow, |dir: Dir, _cmd: TestCommand| {
    dir.create("test", "foobar");
    dir.command().arg("--x-crud").assert_exit_code(0);

    dir.command().arg("-X").arg("--follow").arg("foobar").assert_err();
});

rgtest!(glob, |dir: Dir, _cmd: TestCommand| {
    dir.create("test", "foobar");
    dir.command().arg("--x-crud").assert_exit_code(0);

    dir.command().arg("-X").arg("--glob=test").arg("foobar").assert_err();
});

rgtest!(hidden, |dir: Dir, _cmd: TestCommand| {
    dir.create(".test", "foobar");
    dir.command().arg("--x-crud").assert_exit_code(0);

    dir.command().arg("-X").arg("--hidden").arg("foobar").assert_err();
});

rgtest!(iglob, |dir: Dir, _cmd: TestCommand| {
    dir.create("test", "foobar");
    dir.command().arg("--x-crud").assert_exit_code(0);

    dir.command().arg("-X").arg("--iglob=test").arg("foobar").assert_err();
});

rgtest!(ignore_file, |dir: Dir, _cmd: TestCommand| {
    dir.create("test", "foobar");
    dir.create("ignore-file", "");
    dir.command().arg("--x-crud").assert_exit_code(0);

    dir.command()
        .arg("-X")
        .arg("--ignore-file=ignore-file")
        .arg("foobar")
        .assert_err();
});

rgtest!(no_ignore, |dir: Dir, _cmd: TestCommand| {
    dir.create("test", "foobar");
    dir.command().arg("--x-crud").assert_exit_code(0);

    dir.command().arg("-X").arg("--no-ignore").arg("foobar").assert_err();
    dir.command().arg("-X").arg("--no-ignore-dot").arg("foobar").assert_err();
    dir.command()
        .arg("-X")
        .arg("--no-ignore-exclude")
        .arg("foobar")
        .assert_err();
    dir.command()
        .arg("-X")
        .arg("--no-ignore-files")
        .arg("foobar")
        .assert_err();
    dir.command()
        .arg("-X")
        .arg("--no-ignore-global")
        .arg("foobar")
        .assert_err();
    dir.command()
        .arg("-X")
        .arg("--no-ignore-parent")
        .arg("foobar")
        .assert_err();
    dir.command().arg("-X").arg("--no-ignore-vcs").arg("foobar").assert_err();
});

rgtest!(no_require_git, |dir: Dir, _cmd: TestCommand| {
    dir.create("test", "foobar");
    dir.command().arg("--x-crud").assert_exit_code(0);

    dir.command().arg("-X").arg("--no-require-git").arg("foobar").assert_err();
});

rgtest!(one_file_system, |dir: Dir, _cmd: TestCommand| {
    dir.create("test", "foobar");
    dir.command().arg("--x-crud").assert_exit_code(0);

    dir.command()
        .arg("-X")
        .arg("--one-file-system")
        .arg("foobar")
        .assert_err();
});

#[cfg(unix)] // in order to use `cat`
rgtest!(pre, |dir: Dir, _cmd: TestCommand| {
    dir.create("test", "foobar");
    dir.command().arg("--x-crud").assert_exit_code(0);

    dir.command().arg("-X").arg("--pre").arg("cat").arg("foobar").assert_err();
});

rgtest!(search_zip, |dir: Dir, _cmd: TestCommand| {
    dir.create("test", "foobar");
    dir.command().arg("--x-crud").assert_exit_code(0);

    dir.command().arg("-X").arg("--search-zip").arg("foobar").assert_err();
});

rgtest!(unrestricted, |dir: Dir, _cmd: TestCommand| {
    dir.create("test", "foobar");
    dir.command().arg("--x-crud").assert_exit_code(0);

    dir.command().arg("-X").arg("-u").arg("foobar").assert_err();
    dir.command().arg("-X").arg("-uu").arg("foobar").assert_err();
    dir.command().arg("-X").arg("-uuu").arg("foobar").assert_err();
});
