/// Like assert_eq, but nicer output for long strings.
#[cfg(test)]
#[macro_export]
macro_rules! assert_eq_printed {
    ($expected:expr, $got:expr, $($tt:tt)*) => {
        let expected = &*$expected;
        let got = &*$got;
        let label = format!($($tt)*);
        if expected != got {
            panic!("
printed outputs differ! (label: {})

expected:
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
{}
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

got:
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
{}
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
", label, expected, got);
        }
    }
}
