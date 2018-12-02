// Copyright 2017 The Grin Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

/// Tests exercising the loading and unloading of plugins, as well as the
/// existence and correct functionality of each plugin function
mod common;

extern crate rand;
extern crate cuckoo_miner as cuckoo;

use cuckoo::{PluginConfig};
use common::mine_async_for_duration;

#[test]
fn mine_cuckatoo_mean_compat_cpu_19() {
	let mut config = PluginConfig::new("cuckatoo_mean_compat_cpu_19").unwrap();
	config.params.nthreads = 4;
	mine_async_for_duration(&vec![config], 20);
}

#[test]
fn mine_cuckatoo_mean_compat_cpu_29() {
	let mut config = PluginConfig::new("cuckatoo_mean_compat_cpu_29").unwrap();
	config.params.nthreads = 4;
	mine_async_for_duration(&vec![config], 20);
}

#[ignore]
#[test]
fn mine_cuckatoo_mean_compat_cpu_30() {
	let mut config = PluginConfig::new("cuckatoo_mean_compat_cpu_30").unwrap();
	config.params.nthreads = 4;
	mine_async_for_duration(&vec![config], 20);
}

#[cfg(feature="build-mean-avx2")]
#[test]
fn mine_cuckatoo_mean_avx2_cpu_29() {
	let mut config = PluginConfig::new("cuckatoo_mean_avx2_cpu_29").unwrap();
	config.params.nthreads = 4;
	mine_async_for_duration(&vec![config], 20);
}

#[ignore]
#[cfg(feature="build-mean-avx2")]
#[test]
fn mine_cuckatoo_mean_avx2_cpu_30() {
	let mut config = PluginConfig::new("cuckatoo_mean_avx2_cpu_30").unwrap();
	config.params.nthreads = 4;
	mine_async_for_duration(&vec![config], 20);
}

#[test]
fn mine_cuckatoo_lean_cpu_19() {
	let mut config = PluginConfig::new("cuckatoo_lean_cpu_19").unwrap();
	config.params.nthreads = 4;
	mine_async_for_duration(&vec![config], 20);
}

#[test]
fn mine_cuckatoo_lean_cpu_29() {
	let mut config = PluginConfig::new("cuckatoo_lean_cpu_29").unwrap();
	config.params.nthreads = 4;
	mine_async_for_duration(&vec![config], 20);
}

#[cfg(feature="build-cuda-plugins")]
#[test]
fn mine_cuckatoo_mean_cuda_29() {
	let mut config = PluginConfig::new("cuckatoo_mean_cuda_29").unwrap();
	config.params.expand = 1;
	mine_async_for_duration(&vec![config], 20);
}

#[cfg(feature="build-cuda-plugins")]
#[test]
fn mine_cuckatoo_lean_cuda_29() {
	let config = PluginConfig::new("cuckatoo_lean_cuda_29").unwrap();
	mine_async_for_duration(&vec![config], 20);
}
