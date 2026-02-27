This directory contains a Windows manifest for various Windows-specific
settings.

The main thing we enable here is [`longPathAware`], which permits paths of the
form `C:\` to be longer than 260 characters.

The approach taken here was modeled off of a [similar change for `rustc`][rustc pr].
In particular, this manifest gets linked into the final binary. Those linker
arguments are applied in `build.rs`.

This currently only applies to MSVC builds. If there's an easy way to make this
apply to GNU builds as well, then patches are welcome.

[`longPathAware`]: https://learn.microsoft.com/en-us/windows/win32/sbscs/application-manifests#longpathaware
[rustc pr]: https://github.com/rust-lang/rust/pull/96737
