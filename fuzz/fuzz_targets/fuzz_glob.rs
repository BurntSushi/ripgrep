#![no_main]

use std::str::FromStr;

use globset::Glob;

libfuzzer_sys::fuzz_target!(|glob_str: &str| {
    let Ok(glob) = Glob::new(glob_str) else {
        return;
    };

    let Ok(glob2) = Glob::from_str(glob_str) else {
        return;
    };

    // Verify that a `Glob` constructed with `new` is the same as a `Glob`` constructed
    // with `from_str`.
    assert_eq!(glob, glob2);

    // Verify that `Glob::glob` produces the same string as the original.
    assert_eq!(glob.glob(), glob_str);
});
