// Copyright 2018 The Grin Developers
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

//! Stratum client implementation, for standalone mining against a running
//! grin node
extern crate grin_miner_util as util;
extern crate grin_miner_config as config;

extern crate bufstream;
extern crate time;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
#[macro_use]
extern crate slog;
#[macro_use]
extern crate lazy_static;

use std::thread;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::net::TcpStream;
use bufstream::BufStream;
use config::GlobalConfig;
use util::cuckoo_miner as cuckoo;

use cuckoo::{CuckooPluginManager,
	CuckooPluginCapabilities,
	//CuckooMinerSolution,
	CuckooMinerConfig,
	CuckooMiner};

pub mod plugin;
mod mining;
pub mod client;
mod types;

use util::{init_logger, LOGGER};

// include build information
pub mod built_info {
	include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

pub fn info_strings() -> (String, String, String) {
	(
		format!(
			"This is Grin-Miner version {}{}, built for {} by {}.",
			built_info::PKG_VERSION,
			built_info::GIT_VERSION.map_or_else(|| "".to_owned(), |v| format!(" (git {})", v)),
			built_info::TARGET,
			built_info::RUSTC_VERSION
		).to_string(),
		format!(
			"Built with profile \"{}\", features \"{}\" on {}.",
			built_info::PROFILE,
			built_info::FEATURES_STR,
			built_info::BUILT_TIME_UTC
		).to_string(),
		format!("Dependencies:\n {}", built_info::DEPENDENCIES_STR).to_string(),
	)
}

fn log_build_info() {
	let (basic_info, detailed_info, deps) = info_strings();
	info!(LOGGER, "{}", basic_info);
	debug!(LOGGER, "{}", detailed_info);
	trace!(LOGGER, "{}", deps);
}

fn main() {
	// Init configuration
	let mut global_config = GlobalConfig::new(None).unwrap_or_else(|e| {
		panic!("Error parsing config file: {}", e);
	});
	println!("Starting Grin-Miner from config file at: {}", 
		global_config.config_file_path.unwrap().to_str().unwrap());
	// Init logging
	let log_conf = global_config
		.members
		.as_mut()
		.unwrap()
		.logging
		.clone()
		.unwrap();
	init_logger(Some(log_conf));
	let mining_config = global_config.members.as_mut().unwrap().mining.clone();

	log_build_info();

	// Init mining plugin configuration
	let mut plugin_miner = plugin::PluginMiner::new();
	plugin_miner.init(mining_config.clone());
	let mut mc = mining::Controller::new(plugin_miner).unwrap_or_else(|e| {
		panic!("Error loading mining controller: {}", e);
	});

	let cc = client::Controller::new(&mining_config.stratum_server_addr, mc.tx.clone()).unwrap_or_else(|e| {
		panic!("Error loading stratum client controller: {:?}", e);
	});

	mc.set_client_tx(cc.tx.clone());

	let _ = thread::Builder::new()
		.name("mining_controller".to_string())
		.spawn(move || {
			mc.run();
		});

	let _ = thread::Builder::new()
		.name("client_controller".to_string())
		.spawn(move || {
			cc.run();
		});

	loop{
	}
}
