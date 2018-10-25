# Grin Miner

A standalone mining implementation intended for mining Grin against a running Grin node.

## Supported Platforms

At present, only mining plugins for linux-x86_64 and MacOS exist. This will likely change over time as the community creates more solvers for different platforms.

## Requirements

- rust 1.30+ (use [rustup]((https://www.rustup.rs/))- i.e. `curl https://sh.rustup.rs -sSf | sh; source $HOME/.cargo/env`)
- cmake 3.2+ (for [Cuckoo mining plugins]((https://github.com/mimblewimble/cuckoo-miner)))
- ncurses and libs (ncurses, ncursesw5)
- zlib libs (zlib1g-dev or zlib-devel)
- linux-headers (reported needed on Alpine linux)

And a [running Grin node](https://github.com/mimblewimble/grin/blob/master/doc/build.md) to mine into!

## Build steps

```sh
git clone https://github.com/mimblewimble/grin-miner.git
cd grin-miner
git submodule update --init
cargo build
```

### Building the Cuckoo-Miner plugins

Grin-miner automatically builds x86_64 CPU plugins. Cuda plugins are also provided, but are
not enabled by default. To enable them, modify `Cargo.toml` as follows:

```
change:
cuckoo_miner = { path = "./cuckoo-miner" }
to:
cuckoo_miner = { path = "./cuckoo-miner", features = ["build-cuda-plugins"]}
```

The Cuda toolkit 9+ must be installed on your system (check with `nvcc --version`)

It is also possible to build slightly more optimized versions of the CPU plugins if your processor
supports avx2 instructions:

```
cuckoo_miner = { path = "./cuckoo-miner", features = ["build-mean-avx2"]}
```

### Build errors

See [Troubleshooting](https://github.com/mimblewimble/docs/wiki/Troubleshooting)

## What was built?

A successful build gets you:

 - `target/debug/grin-miner` - the main grin-miner binary
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
This file contains contains inline documentation on all configuration
options, and should be the first point of reference.

You should always ensure that this file exists in the directory from which you're
running grin-miner.

# Using grin-miner

There is a [Grin forum post](https://www.grin-forum.org/t/how-to-mine-cuckoo-30-in-grin-help-us-test-and-collect-stats/152) with further detail on how to configure grin-miner and mine grin's testnet.
