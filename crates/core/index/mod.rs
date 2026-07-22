pub(crate) use self::imp::*;

#[cfg(not(feature = "unstable-index"))]
#[path = "disabled.rs"]
mod imp;
#[cfg(feature = "unstable-index")]
#[path = "enabled.rs"]
mod imp;
