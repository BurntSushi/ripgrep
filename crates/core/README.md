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

这是核心ripgrep crate。特别是“main”。Rs '是' main '函数所在的位置。
大多数ripgrep核心包括两件事:

* CLI接口的定义，包括每个标志的文档。

* 将' grep-matcher '， ' grep-regex '， ' grep-searcher '和' grep-printer '的代码粘合在一起，以实际执行搜索。

目前，没有计划将ripgrep core作为一个独立的库提供。

然而，ripgrep的大部分繁重工作都是通过其组成的crate完成的，这些crate可以独立于ripgrep进行重用。

不幸的是，目前还没有指南或教程来教人们如何做到这一点。
