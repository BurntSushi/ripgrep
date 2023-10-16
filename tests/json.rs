use std::time;

use serde_derive::Deserialize;
use serde_json as json;

use crate::hay::{SHERLOCK, SHERLOCK_CRLF};
use crate::util::{Dir, TestCommand};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "data")]
#[serde(rename_all = "snake_case")]
enum Message {
    Begin(Begin),
    End(End),
    Match(Match),
    Context(Context),
    Summary(Summary),
}

impl Message {
    fn unwrap_begin(&self) -> Begin {
        match *self {
            Message::Begin(ref x) => x.clone(),
            ref x => panic!("expected Message::Begin but got {:?}", x),
        }
    }

    fn unwrap_end(&self) -> End {
        match *self {
            Message::End(ref x) => x.clone(),
            ref x => panic!("expected Message::End but got {:?}", x),
        }
    }

    fn unwrap_match(&self) -> Match {
        match *self {
            Message::Match(ref x) => x.clone(),
            ref x => panic!("expected Message::Match but got {:?}", x),
        }
    }

    fn unwrap_context(&self) -> Context {
        match *self {
            Message::Context(ref x) => x.clone(),
            ref x => panic!("expected Message::Context but got {:?}", x),
        }
    }

    fn unwrap_summary(&self) -> Summary {
        match *self {
            Message::Summary(ref x) => x.clone(),
            ref x => panic!("expected Message::Summary but got {:?}", x),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct Begin {
    path: Option<Data>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct End {
    path: Option<Data>,
    binary_offset: Option<u64>,
    stats: Stats,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct Summary {
    elapsed_total: Duration,
    stats: Stats,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct Match {
    path: Option<Data>,
    lines: Data,
    line_number: Option<u64>,
    absolute_offset: u64,
    submatches: Vec<SubMatch>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct Context {
    path: Option<Data>,
    lines: Data,
    line_number: Option<u64>,
    absolute_offset: u64,
    submatches: Vec<SubMatch>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct SubMatch {
    #[serde(rename = "match")]
    m: Data,
    start: usize,
    end: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
enum Data {
    Text { text: String },
    // This variant is used when the data isn't valid UTF-8. The bytes are
    // base64 encoded, so using a String here is OK.
    Bytes { bytes: String },
}

impl Data {
    fn text(s: &str) -> Data {
        Data::Text { text: s.to_string() }
    }
    fn bytes(s: &str) -> Data {
        Data::Bytes { bytes: s.to_string() }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct Stats {
    elapsed: Duration,
    searches: u64,
    searches_with_match: u64,
    bytes_searched: u64,
    bytes_printed: u64,
    matched_lines: u64,
    matches: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct Duration {
    #[serde(flatten)]
    duration: time::Duration,
    human: String,
}

/// Decode JSON Lines into a Vec<Message>. If there was an error decoding,
/// this function panics.
fn json_decode(jsonlines: &str) -> Vec<Message> {
    json::Deserializer::from_str(jsonlines)
        .into_iter()
        .collect::<Result<Vec<Message>, _>>()
        .unwrap()
}

rgtest!(basic, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.arg("--json").arg("-B1").arg("Sherlock Holmes").arg("sherlock");

    let msgs = json_decode(&cmd.stdout());

    assert_eq!(
        msgs[0].unwrap_begin(),
        Begin { path: Some(Data::text("sherlock")) }
    );
    assert_eq!(
        msgs[1].unwrap_context(),
        Context {
            path: Some(Data::text("sherlock")),
            lines: Data::text(
                "Holmeses, success in the province of \
                 detective work must always\n",
            ),
            line_number: Some(2),
            absolute_offset: 65,
            submatches: vec![],
        }
    );
    assert_eq!(
        msgs[2].unwrap_match(),
        Match {
            path: Some(Data::text("sherlock")),
            lines: Data::text(
                "be, to a very large extent, the result of luck. \
                 Sherlock Holmes\n",
            ),
            line_number: Some(3),
            absolute_offset: 129,
            submatches: vec![SubMatch {
                m: Data::text("Sherlock Holmes"),
                start: 48,
                end: 63,
            },],
        }
    );
    assert_eq!(msgs[3].unwrap_end().path, Some(Data::text("sherlock")));
    assert_eq!(msgs[3].unwrap_end().binary_offset, None);
    assert_eq!(msgs[4].unwrap_summary().stats.searches_with_match, 1);
    assert_eq!(msgs[4].unwrap_summary().stats.bytes_printed, 494);
});

rgtest!(quiet_stats, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK);
    cmd.arg("--json")
        .arg("--quiet")
        .arg("--stats")
        .arg("Sherlock Holmes")
        .arg("sherlock");

    let msgs = json_decode(&cmd.stdout());
    assert_eq!(msgs[0].unwrap_summary().stats.searches_with_match, 1);
    assert_eq!(msgs[0].unwrap_summary().stats.bytes_searched, 367);
});

#[cfg(unix)]
rgtest!(notutf8, |dir: Dir, mut cmd: TestCommand| {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    // This test does not work with PCRE2 because PCRE2 does not support the
    // `u` flag.
    if dir.is_pcre2() {
        return;
    }
    // macOS doesn't like this either... sigh.
    if cfg!(target_os = "macos") {
        return;
    }

    let name = &b"foo\xFFbar"[..];
    let contents = &b"quux\xFFbaz"[..];

    // APFS does not support creating files with invalid UTF-8 bytes, so just
    // skip the test if we can't create our file. Presumably we don't need this
    // check if we're already skipping it on macOS, but maybe other file
    // systems won't like this test either?
    if !dir.try_create_bytes(OsStr::from_bytes(name), contents).is_ok() {
        return;
    }
    cmd.arg("--json").arg(r"(?-u)\xFF");

    let msgs = json_decode(&cmd.stdout());

    assert_eq!(
        msgs[0].unwrap_begin(),
        Begin { path: Some(Data::bytes("Zm9v/2Jhcg==")) }
    );
    assert_eq!(
        msgs[1].unwrap_match(),
        Match {
            path: Some(Data::bytes("Zm9v/2Jhcg==")),
            lines: Data::bytes("cXV1eP9iYXo="),
            line_number: Some(1),
            absolute_offset: 0,
            submatches: vec![SubMatch {
                m: Data::bytes("/w=="),
                start: 4,
                end: 5,
            },],
        }
    );
});

rgtest!(notutf8_file, |dir: Dir, mut cmd: TestCommand| {
    use std::ffi::OsStr;

    // This test does not work with PCRE2 because PCRE2 does not support the
    // `u` flag.
    if dir.is_pcre2() {
        return;
    }

    let name = "foo";
    let contents = &b"quux\xFFbaz"[..];

    // APFS does not support creating files with invalid UTF-8 bytes, so just
    // skip the test if we can't create our file.
    if !dir.try_create_bytes(OsStr::new(name), contents).is_ok() {
        return;
    }
    cmd.arg("--json").arg(r"(?-u)\xFF");

    let msgs = json_decode(&cmd.stdout());

    assert_eq!(
        msgs[0].unwrap_begin(),
        Begin { path: Some(Data::text("foo")) }
    );
    assert_eq!(
        msgs[1].unwrap_match(),
        Match {
            path: Some(Data::text("foo")),
            lines: Data::bytes("cXV1eP9iYXo="),
            line_number: Some(1),
            absolute_offset: 0,
            submatches: vec![SubMatch {
                m: Data::bytes("/w=="),
                start: 4,
                end: 5,
            },],
        }
    );
});

// See: https://github.com/BurntSushi/ripgrep/issues/416
//
// This test in particular checks that our match does _not_ include the `\r`
// even though the '$' may be rewritten as '(?:\r??$)' and could thus include
// `\r` in the match.
rgtest!(crlf, |dir: Dir, mut cmd: TestCommand| {
    dir.create("sherlock", SHERLOCK_CRLF);
    cmd.arg("--json").arg("--crlf").arg(r"Sherlock$").arg("sherlock");

    let msgs = json_decode(&cmd.stdout());

    assert_eq!(
        msgs[1].unwrap_match().submatches[0].clone(),
        SubMatch { m: Data::text("Sherlock"), start: 56, end: 64 },
    );
});

// See: https://github.com/BurntSushi/ripgrep/issues/1095
//
// This test checks that we don't drop the \r\n in a matching line when --crlf
// mode is enabled.
rgtest!(r1095_missing_crlf, |dir: Dir, mut cmd: TestCommand| {
    dir.create("foo", "test\r\n");

    // Check without --crlf flag.
    let msgs = json_decode(&cmd.arg("--json").arg("test").stdout());
    assert_eq!(msgs.len(), 4);
    assert_eq!(msgs[1].unwrap_match().lines, Data::text("test\r\n"));

    // Now check with --crlf flag.
    let msgs = json_decode(&cmd.arg("--crlf").stdout());
    assert_eq!(msgs.len(), 4);
    assert_eq!(msgs[1].unwrap_match().lines, Data::text("test\r\n"));
});

// See: https://github.com/BurntSushi/ripgrep/issues/1095
//
// This test checks that we don't return empty submatches when matching a `\n`
// in CRLF mode.
rgtest!(r1095_crlf_empty_match, |dir: Dir, mut cmd: TestCommand| {
    dir.create("foo", "test\r\n\n");

    // Check without --crlf flag.
    let msgs = json_decode(&cmd.arg("-U").arg("--json").arg("\n").stdout());
    assert_eq!(msgs.len(), 4);

    let m = msgs[1].unwrap_match();
    assert_eq!(m.lines, Data::text("test\r\n\n"));
    assert_eq!(m.submatches[0].m, Data::text("\n"));
    assert_eq!(m.submatches[1].m, Data::text("\n"));

    // Now check with --crlf flag.
    let msgs = json_decode(&cmd.arg("--crlf").stdout());
    assert_eq!(msgs.len(), 4);

    let m = msgs[1].unwrap_match();
    assert_eq!(m.lines, Data::text("test\r\n\n"));
    assert_eq!(m.submatches[0].m, Data::text("\n"));
    assert_eq!(m.submatches[1].m, Data::text("\n"));
});

// See: https://github.com/BurntSushi/ripgrep/issues/1412
rgtest!(r1412_look_behind_match_missing, |dir: Dir, mut cmd: TestCommand| {
    // Only PCRE2 supports look-around.
    if !dir.is_pcre2() {
        return;
    }

    dir.create("test", "foo\nbar\n");

    let msgs = json_decode(
        &cmd.arg("-U").arg("--json").arg(r"(?<=foo\n)bar").stdout(),
    );
    assert_eq!(msgs.len(), 4);

    let m = msgs[1].unwrap_match();
    assert_eq!(m.lines, Data::text("bar\n"));
    assert_eq!(m.submatches.len(), 1);
});
