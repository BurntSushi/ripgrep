This directory contains updated benchmarks as of 2022-12-16. They were captured
via the benchsuite script at `benchsuite/benchsuite` from the root of this
repository. The command that was run:

    $ ./benchsuite \
          --dir /dev/shm/benchsuite \
          --raw runs/2022-12-16-archlinux-duff/raw.csv \
          | tee runs/2022-12-16-archlinux-duff/summary

The versions of each tool are as follows:

    $ rg --version
    ripgrep 13.0.0 (rev 87c4a2b4b1)
    -SIMD -AVX (compiled)
    +SIMD +AVX (runtime)

    $ grep -V
    grep (GNU grep) 3.8

    $ ag -V
    ag version 2.2.0

    Features:
      +jit +lzma +zlib

    $ git --version
    git version 2.39.0

    $ ugrep --version
    ugrep 3.9.2 x86_64-pc-linux-gnu +avx2 +pcre2jit +zlib +bzip2 +lzma +lz4 +zstd
    License BSD-3-Clause: <https://opensource.org/licenses/BSD-3-Clause>
    Written by Robert van Engelen and others: <https://github.com/Genivia/ugrep>

The version of ripgrep used was compiled from source on commit 7f23cd63:

    $ cargo build --release --features 'pcre2'

This was run on a machine with an Intel i9-12900K with 128GB of memory.
