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
extern crate cursive;

pub mod plugin;
pub mod mining;
pub mod client;
pub mod types;
pub mod stats;
pub mod tui;

use std::thread;
use std::sync::{mpsc, Arc, RwLock};
use config::GlobalConfig;
use util::cuckoo_miner as cuckoo;

use tui::ui;

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


fn start_tui(
	mc: Arc<RwLock<stats::MiningStats>>, 
	client_tx: mpsc::Sender<types::ClientMessage>,
	miner_tx: mpsc::Sender<types::MinerMessage>) {
	// Run the UI controller.. here for now for simplicity to access
	// everything it might need
	println!("Starting Grin Miner in UI mode...");
	let _ = thread::Builder::new()
		.name("ui".to_string())
		.spawn(move || {
			let mut controller = ui::Controller::new().unwrap_or_else(|e| {
				panic!("Error loading UI controller: {}", e);
			});
			controller.run(mc.clone());
			// Shut down everything else on tui exit
			let _ = client_tx.send(types::ClientMessage::Shutdown);
			let _ = miner_tx.send(types::MinerMessage::Shutdown);
			println!("Stopping mining plugins and exiting...");
		});
}

fn main() {
	// Init configuration
	let mut global_config = GlobalConfig::new(None).unwrap_or_else(|e| {
		panic!("Error parsing config file: {}", e);
	});
	println!("Starting Grin-Miner from config file at: {}", 
		global_config.config_file_path.unwrap().to_str().unwrap());
	// Init logging
	let mut log_conf = global_config
		.members
		.as_mut()
		.unwrap()
		.logging
		.clone()
		.unwrap();

	let mining_config = global_config.members.as_mut().unwrap().mining.clone();

	if mining_config.run_tui {
		log_conf.log_to_stdout = false;
		log_conf.tui_running = Some(true);
	}

	init_logger(Some(log_conf));

	log_build_info();

	let stats = Arc::new(RwLock::new(stats::MiningStats::default()));

	let mut mc = mining::Controller::new(mining_config.clone(), stats.clone()).unwrap_or_else(|e| {
		panic!("Error loading mining controller: {}", e);
	});

	let cc = client::Controller::new(&mining_config.stratum_server_addr, mc.tx.clone()).unwrap_or_else(|e| {
		panic!("Error loading stratum client controller: {:?}", e);
	});

	if mining_config.run_tui {
		start_tui(stats.clone(), cc.tx.clone(), mc.tx.clone());
	}

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
