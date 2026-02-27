# Fuzz Testing

## Introduction

Fuzz testing produces pseudo-random / arbitrary data that is used to find
stability issues within a code base. While Rust provides a strong type system,
this does not guarantee that an object will convert properly from one struct
to another. It is the responsibility of the developer to ensure that a struct
is converted properly. Fuzz testing will generate input within the domain of
each property. This arbitrary data can then be used to convert from ObjectA
to ObjectB and then back. This type of testing will help catch bugs that the
type system is not able to see.

## Installation

This crate relies on the `cargo-fuzz` component. To install this component,
run the following from the `fuzz` directory:

```bash
cargo install cargo-fuzz
```

## Listing Targets

Once installed, fuzz targets can be listed by running the following command:

```bash
cargo fuzz list
```

This command will print out a list of all targets that can be tested.

## Running Fuzz Tests

To run a fuzz test, the target must be specified:

```bash
cargo fuzz run <target>
```

Note that the above will run the fuzz test indefinitely. Use the
`-max_total_time=<num seconds>` flag to specify how many seconds the test
should run for:

```bash
cargo fuzz run <target> -- -max_total_time=5
```

The above command will run the fuzz test for five seconds. If the test
completes without error it will show how many tests were run successfully.
The test will abort and return a non-zero error code if it is able to produce
an error. The arbitrary input will be displayed in the event of a failure.
