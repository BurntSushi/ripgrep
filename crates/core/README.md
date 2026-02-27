ripgrep core
------------
This is the core ripgrep crate. In particular, `main.rs` is where the `main`
function lives.

Most of ripgrep core consists of two things:

* The definition of the CLI interface, including docs for every flag.
* Glue code that brings the `grep-matcher`, `grep-regex`, `grep-searcher` and
  `grep-printer` crates together to actually execute the search.

Currently, there are no plans to make ripgrep core available as an independent
library. However, much of the heavy lifting of ripgrep is done via its
constituent crates, which can be reused independent of ripgrep. Unfortunately,
there is no guide or tutorial to teach folks how to do this yet.
