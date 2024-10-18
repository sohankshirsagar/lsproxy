# Contributing Guide

We welcome all kinds of contributions. _You don't need to be an expert
in API or rust development to help out._

## Checklist

Contributions are made through
[pull requests](https://help.github.com/articles/using-pull-requests/).
Before sending a pull request, make sure to do the following:

- [Build and format](#build-format) your code
- [Write tests](#tests)
- [Run tests](#tests) and check that they pass

_Please reach out to the lsproxy team before starting work on a large
contribution._ Get in touch at
[GitHub issues](https://github.com/agentic-labs/lsproxy/issues)
or [on Discord](https://discord.gg/WafeS3jN).

## Prerequisites

To build lsproxy from source, all you need is docker. But if you'd like to do some intermediate steps locally (e.g., `cargo fmt`) feel free.

- [Docker installation instructions](https://docs.docker.com/engine/install/)
- [Rust installation instructions](https://www.rust-lang.org/tools/install)

## Building from source

Building using docker is pretty simple. We have a build script for this purpose.

```
./scripts/build.sh
```

> :warning: You *can* build outside of the docker container with `cargo build`. But this may cause compatibility issues if your system is significantly different from the docker container.

## Running locally

Running is also pretty simple.

```
./scripts/run.sh $WORKSPACE_PATH
```

The run script also builds as well, so you can just use this single script if you're iterating in development.

> :warning: Like above you *can* run outside the docker container, but there are runtime dependencies and specific paths lsproxy is expecting. We do not officially support running lsproxy outside of docker.
