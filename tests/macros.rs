#[macro_export]
macro_rules! rgtest {
    ($name:ident, $fun:expr) => {
        #[test]
        fn $name() {
            let (dir, cmd) = crate::util::setup(stringify!($name));
            $fun(dir, cmd);

            if cfg!(feature = "pcre2") {
                let (dir, cmd) = crate::util::setup_pcre2(stringify!($name));
                $fun(dir, cmd);
            }
        }
    };
}

#[macro_export]
macro_rules! eqnice {
    ($expected:expr, $got:expr) => {
        let expected = &*$expected;
        let got = &*$got;
        if expected != got {
            panic!("
printed outputs differ!

expected:
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
{}
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

got:
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
{}
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
", expected, got);
        }
    }
}

#[macro_export]
macro_rules! eqnice_repr {
    ($expected:expr, $got:expr) => {
        let expected = &*$expected;
        let got = &*$got;
        if expected != got {
            panic!("
printed outputs differ!

expected:
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
{:?}
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

got:
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
{:?}
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
", expected, got);
        }
    }
}
