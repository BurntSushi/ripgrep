/*!
Enumerate every fixed-string encoding of a literal byte query under a base64
alphabet.

Base64 packs 8-bit bytes into 6-bit groups, so the same literal can appear in
encoded output at three different bit offsets (0, 2, or 4 leading bits owned
by a neighboring byte). For each offset we enumerate the unknown leading and
trailing bits, encode the resulting padded buffer, and trim the chars that
cover those unknown bits. The disjunction of all such encodings is what we
want to search for; we return the literals themselves so callers can feed
them into ripgrep's fixed-strings path and build an `Hir::alternation` of
`Hir::literal` directly.
*/

/// The standard base64 alphabet (RFC 4648 Section 4).
const STANDARD: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// The URL- and filename-safe base64 alphabet (RFC 4648 Section 5).
const URL_SAFE: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// Which base64 alphabet to use when generating a search regex.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Alphabet {
    /// RFC 4648 Section 4 — uses `+/`.
    Standard,
    /// RFC 4648 Section 5 — uses `-_` instead of `+/`.
    UrlSafe,
}

impl Alphabet {
    fn table(self) -> &'static [u8; 64] {
        match self {
            Alphabet::Standard => STANDARD,
            Alphabet::UrlSafe => URL_SAFE,
        }
    }
}

/// Expand each pattern in `patterns` into every literal substring its base64
/// encoding can take, across all 3-byte alignments and all possible
/// surrounding bits.
///
/// The result is intended to be passed as fixed-string patterns to a regex
/// matcher with `fixed_strings(true)`, which will build an `Hir::alternation`
/// of `Hir::literal` directly and skip regex parsing entirely.
pub(crate) fn expand_patterns(
    alphabet: Alphabet,
    patterns: Vec<String>,
) -> Vec<String> {
    patterns
        .into_iter()
        .flat_map(|q| pattern_literals(alphabet, q.as_bytes()))
        .collect()
}

/// Returns every literal substring that the encoded form of `query` can take
/// in a base64 stream, across all 3-byte alignments and all possible
/// surrounding bits. Each `(offset, lead, trail)` tuple produces a distinct
/// bit pattern, so the result is naturally free of duplicates.
///
/// Returns an empty vector if `query` is empty.
fn pattern_literals(alphabet: Alphabet, query: &[u8]) -> Vec<String> {
    if query.is_empty() {
        return Vec::new();
    }

    let table = alphabet.table();
    // Worst case is 1*4 + 4*1 + 16*16 = 264 alternatives (sum of
    // lead_count * trail_count across offsets 0/1/2), attained when the
    // query length is 2 mod 3.
    let mut alternatives: Vec<String> = Vec::with_capacity(264);
    let mut encoded = String::new();
    // `+ 3` covers `offset` leading pad bytes (up to 2) plus one trailing
    // byte appended in the inner loop.
    let mut padded: Vec<u8> = Vec::with_capacity(query.len() + 3);

    // Each base64 character encodes 6 bits, so the query can start at one of
    // three bit alignments relative to the encoder's 3-byte groups: 0, 2 or
    // 4 leading bits owned by a preceding byte.
    for offset in 0..3usize {
        let lead_bits = 2 * offset;
        let lead_count = 1u32 << lead_bits;

        padded.clear();
        padded.resize(offset, 0);
        padded.extend_from_slice(query);

        // Number of bits at the tail of the encoding that are owned by a
        // following byte we haven't placed yet. A value of 6 means the query
        // ends exactly on a 6-bit boundary, so no trailing byte is needed.
        let total_bits = padded.len() * 8;
        let trail_bits = 6 - (total_bits % 6);

        for lead in 0..lead_count {
            if offset > 0 {
                // The lead value occupies the least-significant `lead_bits`
                // bits of the byte immediately before the query.
                padded[offset - 1] = lead as u8;
            }
            if trail_bits < 6 {
                let trail_count = 1u32 << trail_bits;
                for trail in 0..trail_count {
                    // The trail value occupies the most-significant
                    // `trail_bits` bits of the byte immediately after the
                    // query.
                    let tb = (trail as u8) << (8 - trail_bits);
                    padded.push(tb);
                    encoded.clear();
                    encode(table, &padded, &mut encoded);
                    // Skip the leading chars covered by `lead` and drop the
                    // trailing char covered by `trail`; what remains is the
                    // run of characters that `query` actually contributes to
                    // the output stream at this alignment.
                    let slice = &encoded[offset..encoded.len() - 1];
                    alternatives.push(slice.to_string());
                    padded.pop();
                }
            } else {
                encoded.clear();
                encode(table, &padded, &mut encoded);
                // Skip the leading chars covered by `lead`. No trailing
                // char to drop: the query ends on a 6-bit boundary.
                alternatives.push(encoded[offset..].to_string());
            }
        }
    }

    alternatives
}

/// Encode `input` into `out` using raw (unpadded) base64 with the given
/// 64-byte alphabet table.
fn encode(table: &[u8; 64], input: &[u8], out: &mut String) {
    let mut i = 0;
    while i + 3 <= input.len() {
        let n = (u32::from(input[i]) << 16)
            | (u32::from(input[i + 1]) << 8)
            | u32::from(input[i + 2]);
        out.push(table[((n >> 18) & 0x3f) as usize] as char);
        out.push(table[((n >> 12) & 0x3f) as usize] as char);
        out.push(table[((n >> 6) & 0x3f) as usize] as char);
        out.push(table[(n & 0x3f) as usize] as char);
        i += 3;
    }
    match input.len() - i {
        0 => {}
        1 => {
            let n = u32::from(input[i]) << 16;
            out.push(table[((n >> 18) & 0x3f) as usize] as char);
            out.push(table[((n >> 12) & 0x3f) as usize] as char);
        }
        2 => {
            let n =
                (u32::from(input[i]) << 16) | (u32::from(input[i + 1]) << 8);
            out.push(table[((n >> 18) & 0x3f) as usize] as char);
            out.push(table[((n >> 12) & 0x3f) as usize] as char);
            out.push(table[((n >> 6) & 0x3f) as usize] as char);
        }
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_string(table: &[u8; 64], input: &[u8]) -> String {
        let mut s = String::new();
        encode(table, input, &mut s);
        s
    }

    #[test]
    fn encode_known_answers_standard() {
        // RFC 4648 test vectors (raw, no padding).
        assert_eq!(encode_string(STANDARD, b""), "");
        assert_eq!(encode_string(STANDARD, b"f"), "Zg");
        assert_eq!(encode_string(STANDARD, b"fo"), "Zm8");
        assert_eq!(encode_string(STANDARD, b"foo"), "Zm9v");
        assert_eq!(encode_string(STANDARD, b"foob"), "Zm9vYg");
        assert_eq!(encode_string(STANDARD, b"fooba"), "Zm9vYmE");
        assert_eq!(encode_string(STANDARD, b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn encode_url_safe_swaps_chars() {
        // Bytes that produce `+` and `/` under the standard alphabet must
        // produce `-` and `_` under the URL-safe alphabet.
        // 0xfb, 0xff packs to indices 0x3e (`+` / `-`) and 0x3f (`/` / `_`).
        assert_eq!(encode_string(STANDARD, &[0xfb, 0xff, 0xff]), "+///");
        assert_eq!(encode_string(URL_SAFE, &[0xfb, 0xff, 0xff]), "-___");
    }

    #[test]
    fn empty_query_returns_no_literals() {
        assert!(pattern_literals(Alphabet::Standard, b"").is_empty());
        assert!(pattern_literals(Alphabet::UrlSafe, b"").is_empty());
    }

    #[test]
    fn aligned_encoding_is_a_literal() {
        // For a 3-byte query (aligned), the unpadded encoding must be one of
        // the returned literals.
        let lits = pattern_literals(Alphabet::Standard, b"foo");
        assert!(lits.iter().any(|s| s == "Zm9v"), "missing Zm9v in {lits:?}");
    }

    #[test]
    fn literals_are_unique() {
        // The (offset, lead, trail) enumeration is supposed to produce
        // distinct bit patterns. If a future change breaks that invariant,
        // we'd silently bloat the matcher with duplicate literals.
        use std::collections::HashSet;
        for alphabet in [Alphabet::Standard, Alphabet::UrlSafe] {
            for q in [&b"a"[..], b"foo", b"hello", b"secret-token-1234"] {
                let lits = pattern_literals(alphabet, q);
                let set: HashSet<&String> = lits.iter().collect();
                assert_eq!(
                    set.len(),
                    lits.len(),
                    "duplicate literals for alphabet={alphabet:?}, \
                     query={q:?}: produced {} but only {} unique",
                    lits.len(),
                    set.len(),
                );
            }
        }
    }

    #[test]
    fn url_safe_literals_have_no_standard_only_chars() {
        // URL-safe literals must never contain `+` or `/`.
        let lits =
            pattern_literals(Alphabet::UrlSafe, b"any old secret 12345");
        for lit in &lits {
            assert!(!lit.contains('+'), "url-safe literal has '+': {lit}");
            assert!(!lit.contains('/'), "url-safe literal has '/': {lit}");
        }
    }

    /// Every encoded instance of the query, at any 3-byte alignment with any
    /// surrounding bytes, must be matched when the returned literals are fed
    /// to a regex matcher with `fixed_strings(true)`.
    #[test]
    fn matches_every_alignment() {
        use grep::matcher::Matcher;
        use grep::regex::RegexMatcherBuilder;

        let queries: &[&[u8]] =
            &[b"foo", b"hello", b"hi", b"a", b"secret-token-1234"];
        let prefixes: &[&[u8]] =
            &[b"", b"\x00", b"\xff", b"\x00\x00", b"\xff\xff"];
        let suffixes: &[&[u8]] =
            &[b"", b"\x00", b"\xff", b"\x00\x00", b"\xff\xff"];

        for alphabet in [Alphabet::Standard, Alphabet::UrlSafe] {
            let table = alphabet.table();
            for q in queries {
                let lits = pattern_literals(alphabet, q);
                let matcher = RegexMatcherBuilder::new()
                    .fixed_strings(true)
                    .build_many(&lits)
                    .unwrap();
                for p in prefixes {
                    for s in suffixes {
                        let mut bytes = Vec::new();
                        bytes.extend_from_slice(p);
                        bytes.extend_from_slice(q);
                        bytes.extend_from_slice(s);
                        let mut encoded = String::new();
                        encode(table, &bytes, &mut encoded);
                        assert!(
                            matcher.is_match(encoded.as_bytes()).unwrap(),
                            "matcher did not match {encoded:?} \
                             (alphabet={alphabet:?}, query={q:?}, \
                             prefix={p:?}, suffix={s:?})",
                        );
                    }
                }
            }
        }
    }
}
