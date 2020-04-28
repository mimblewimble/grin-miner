// Copyright 2020 The Grin Developers
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
extern crate cuckoo_miner as cuckoo;
extern crate grin_miner_config as config;
extern crate grin_miner_plugin as plugin;
extern crate grin_miner_util as util;

extern crate bufstream;
extern crate native_tls;
extern crate time;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
#[macro_use]
extern crate slog;

#[cfg(feature = "tui")]
extern crate cursive;

pub mod client;
pub mod mining;
pub mod stats;
pub mod types;

#[cfg(feature = "tui")]
pub mod tui;

use config::GlobalConfig;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;

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
		),
		format!(
			"Built with profile \"{}\", features \"{}\" on {}.",
			built_info::PROFILE,
			built_info::FEATURES_STR,
			built_info::BUILT_TIME_UTC
		),
		format!("Dependencies:\n {}", built_info::DEPENDENCIES_STR),
	)
}

fn log_build_info() {
	let (basic_info, detailed_info, deps) = info_strings();
	info!(LOGGER, "{}", basic_info);
	debug!(LOGGER, "{}", detailed_info);
	trace!(LOGGER, "{}", deps);
}

#[cfg(feature = "tui")]
mod with_tui {
	use stats;
	use std::sync::atomic::{AtomicBool, Ordering};
	use std::sync::{mpsc, Arc, RwLock};
	use std::thread;
	use tui::ui;
	use types;

	pub fn start_tui(
		s: Arc<RwLock<stats::Stats>>,
		client_tx: mpsc::Sender<types::ClientMessage>,
		miner_tx: mpsc::Sender<types::MinerMessage>,
		stop: Arc<AtomicBool>,
	) {
		// Run the UI controller.. here for now for simplicity to access
		// everything it might need
		println!("Starting Grin Miner in UI mode...");
		println!("Waiting for solvers to shutdown...");
		let _ = thread::Builder::new()
			.name("ui".to_string())
			.spawn(move || {
				let mut controller = ui::Controller::new().unwrap_or_else(|e| {
					panic!("Error loading UI controller: {}", e);
				});
				controller.run(s.clone());
				// Shut down everything else on tui exit
				let _ = client_tx.send(types::ClientMessage::Shutdown);
				let _ = miner_tx.send(types::MinerMessage::Shutdown);
				stop.store(true, Ordering::Relaxed);
			});
	}
}

fn main() {
	// Init configuration
	let mut global_config = GlobalConfig::new(None).unwrap_or_else(|e| {
		panic!("Error parsing config file: {}", e);
	});
	println!(
		"Starting Grin-Miner from config file at: {}",
		global_config.config_file_path.unwrap().to_str().unwrap()
	);
	// Init logging
	let mut log_conf = global_config
		.members
		.as_mut()
		.unwrap()
		.logging
		.clone()
		.unwrap();

	let mining_config = global_config.members.as_mut().unwrap().mining.clone();

	if cfg!(feature = "tui") && mining_config.run_tui {
		log_conf.log_to_stdout = false;
		log_conf.tui_running = Some(true);
	}

	init_logger(Some(log_conf));

	log_build_info();
	let stats = Arc::new(RwLock::new(stats::Stats::default()));

	let mut mc =
		mining::Controller::new(mining_config.clone(), stats.clone()).unwrap_or_else(|e| {
			panic!("Error loading mining controller: {}", e);
		});
	let cc = client::Controller::new(
		&mining_config.stratum_server_addr,
		mining_config.stratum_server_login.clone(),
		mining_config.stratum_server_password.clone(),
		mining_config.stratum_server_tls_enabled,
		mc.tx.clone(),
		stats.clone(),
	)
	.unwrap_or_else(|e| {
		panic!("Error loading stratum client controller: {:?}", e);
	});
	let tui_stopped = Arc::new(AtomicBool::new(false));
	let miner_stopped = Arc::new(AtomicBool::new(false));
	let client_stopped = Arc::new(AtomicBool::new(false));

	// Load plugin configuration and start solvers first,
	// so we can exit pre-tui if something is obviously wrong
	debug!(LOGGER, "Starting solvers");
	let result = config::read_configs(
		mining_config.miner_plugin_dir.clone(),
		mining_config.miner_plugin_config.clone(),
	);
	let mut miner = match result {
		Ok(cfgs) => cuckoo::CuckooMiner::new(cfgs),
		Err(e) => {
			println!("Error loading plugins. Please check logs for further info.");
			println!("Error details:");
			println!("{:?}", e);
			println!("Exiting");
			return;
		}
	};
	if let Err(e) = miner.start_solvers() {
		println!("Error starting plugins. Please check logs for further info.");
		println!("Error details:");
		println!("{:?}", e);
		println!("Exiting");
		return;
	}

	if mining_config.run_tui {
		#[cfg(feature = "tui")]
		with_tui::start_tui(stats, cc.tx.clone(), mc.tx.clone(), tui_stopped.clone());

		#[cfg(not(feature = "tui"))]
		warn!(LOGGER, "Grin-miner was built with TUI support disabled!");
	} else {
		tui_stopped.store(true, Ordering::Relaxed);
	}

	mc.set_client_tx(cc.tx.clone());

	let miner_stopped_internal = miner_stopped.clone();
	let _ = thread::Builder::new()
		.name("mining_controller".to_string())
		.spawn(move || {
			if let Err(e) = mc.run(miner) {
				error!(
					LOGGER,
					"Error loading plugins. Please check logs for further info: {:?}", e
				);
				return;
			}
			miner_stopped_internal.store(true, Ordering::Relaxed);
		});

	let client_stopped_internal = client_stopped.clone();
	let _ = thread::Builder::new()
		.name("client_controller".to_string())
		.spawn(move || {
			cc.run();
			client_stopped_internal.store(true, Ordering::Relaxed);
		});

	loop {
		if miner_stopped.load(Ordering::Relaxed)
			&& client_stopped.load(Ordering::Relaxed)
			&& tui_stopped.load(Ordering::Relaxed)
		{
			thread::sleep(std::time::Duration::from_millis(100));
			break;
		}
		thread::sleep(std::time::Duration::from_millis(100));
	}
}
