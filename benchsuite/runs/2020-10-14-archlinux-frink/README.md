This directory contains updated benchmarks as of 2020-10-14. They were captured
via the benchsuite script at `benchsuite/benchsuite` from the root of this
repository. The command that was run:

    $ ./benchsuite \
          --dir /tmp/benchsuite \
          --raw runs/2020-10-14-archlinux-frink/raw.csv \
          --warmup-iter 1 \
          --bench-iter 5

The versions of each tool are as follows:

    $ rg --version
    ripgrep 12.1.1 (rev def993bad1)
    -SIMD -AVX (compiled)
    +SIMD +AVX (runtime)

    $ grep -V
    grep (GNU grep) 3.4

    $ ag -V
    ag version 2.2.0

    Features:
      +jit +lzma +zlib

    $ git --version
    git version 2.28.0

    $ ugrep --version
    ugrep 3.0.2 x86_64-pc-linux-gnu +avx2 +pcre2_jit +zlib +bzip2 +lzma +lz4
    License BSD-3-Clause: <https://opensource.org/licenses/BSD-3-Clause>
    Written by Robert van Engelen and others: <https://github.com/Genivia/ugrep>

The version of ripgrep used was compiled from source on commit def993bad1:

    $ cargo build --release --features 'pcre2'
