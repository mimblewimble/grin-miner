# Grin Miner - Build, Configuration, and Running

## Supported Platforms

At present, only mining plugins for linux-x86_64 and MacOS exist. This will likley change over time as the community creates more solvers for different platforms.

## Requirements

- rust 1.24+ (use [rustup]((https://www.rustup.rs/))- i.e. `curl https://sh.rustup.rs -sSf | sh; source $HOME/.cargo/env`)
- cmake 3.2+ (for [Cuckoo mining plugins]((https://github.com/mimblewimble/cuckoo-miner)))
- ncurses and libs (ncurses, ncursesw5)
- zlib libs (zlib1g-dev or zlib-devel)
- linux-headers (reported needed on Alpine linux)

And a [running Grin node](https://github.com/mimblewimble/grin/blob/master/doc/build.md) to mine into!

## Build steps

```sh
git clone https://github.com/mimblewimble/grin-miner.git
cd grin-miner
cargo build
```

### Building the Cuckoo-Miner plugins

Grin-miner automatically builds x86_64 CPU plugins. Cuda plugins are also provided, but are
not enabled by default. To enable them uncomment the line:

```
features=["build-cuda-plugins"]
```
In util/Cargo.toml. The Cuda toolkit must be installed on your system.

### Build errors

See [Troubleshooting](https://github.com/mimblewimble/docs/wiki/Troubleshooting)

## What was built?

A successful build gets you:

 - `target/debug/grin-miner` - the main grin binary
 - `target/debug/plugins/*` - mining plugins

Make sure you always run grin-miner within a directory that contains a
`grin-miner.toml` configuration file.

While testing, put the grin-miner binary on your path like this:

```
export PATH=/path/to/grin-miner/dir/target/debug:$PATH
```

You can then run `grin-miner` directly.

# Configuration

Grin-miner can be further configured via the `grin-miner.toml` file. 
You should always ensure that this file is available to grin-miner.
The supplied `grin-miner.toml` contains inline documentation on all configuration
options, and should be the first point of reference for all options.

The `grin-miner.toml` file can placed in one of several locations, using the first one it finds:

1. The current working directory
2. In the directory that holds the grin executable
3. {USER_HOME}/.grin

# Using grin

There is a [Grin forum post](https://www.grin-forum.org/t/how-to-mine-cuckoo-30-in-grin-help-us-test-and-collect-stats/152) with further detail on how to configure and mine within grin.

