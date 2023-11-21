These are Docker images used for cross compilation in CI builds (or locally)
via the [Cross](https://github.com/rust-embedded/cross) tool.

The Cross tool actually provides its own Docker images, and all Docker images
in this directory are derived from one of them. We provide our own in order to
customize the environment. For example, we need to install compression tools
like `xz` so that tests for the `-z/--search-zip` flag are run.

If you make a change to a Docker image, then you can re-build it. `cd` into the
directory containing the `Dockerfile` and run:

    $ cd x86_64-unknown-linux-musl
    $ ./build

At this point, subsequent uses of `cross` will now use your built image since
Docker prefers local images over remote images. In order to make these changes
stick, they need to be pushed to Docker Hub:

    $ docker push burntsushi/cross:x86_64-unknown-linux-musl

Of course, only I (BurntSushi) can push to that location. To make `cross` use
a different location, then edit `Cross.toml` in the root of this repo to use
a different image name for the desired target.
