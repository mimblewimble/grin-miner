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

use std::thread;
use std::io::BufRead;
use std::path::PathBuf;
use std::net::TcpStream;
use bufstream::BufStream;
use config::GlobalConfig;
use util::cuckoo_miner as cuckoo;

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

use cuckoo::{CuckooPluginManager,
	CuckooPluginCapabilities,
	//CuckooMinerSolution,
	CuckooMinerConfig,
	CuckooMiner};


#[derive(Serialize, Deserialize, Debug)]
struct JobTemplate {
	pre_pow: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct RpcRequest {
	id: String,
	jsonrpc: String,
	method: String,
	params: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct RpcResponse {
	id: String,
	jsonrpc: String,
	result: Option<String>,
	error: Option<RpcError>,
}

#[derive(Serialize, Deserialize, Debug)]
struct RpcError {
	code: i32,
	message: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct LoginParams {
	login: String,
	pass: String,
	agent: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct SubmitParams {
	height: u64,
	nonce: u64,
	pow: Vec<u32>,
} 
//
// Helper to load plugins
pub fn get_plugin_vec(filter: &str) -> Vec<CuckooPluginCapabilities>{
	let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	d.push("target/debug/plugins/");

	// get all plugins in directory
	let mut plugin_manager = CuckooPluginManager::new().unwrap();
	plugin_manager
		.load_plugin_dir(String::from(d.to_str().unwrap()))
		.expect("");

	// Get a list of installed plugins and capabilities
	plugin_manager.get_available_plugins(filter).unwrap()
}

// Helper function to actually mine, just  testing for now.
pub fn mine_async(full_paths: Vec<&str>, 
	duration_in_seconds: i64, 
	pre_header: &str,
	post_header: &str,
	params:Option<Vec<(String, u32, u32)>>) {
	let mut config_vec=Vec::new();
	for p in full_paths.into_iter() {
		let mut config = CuckooMinerConfig::new();
		config.plugin_full_path = String::from(p);
		if let Some(p) = params.clone() {
			config.parameter_list = p.clone();
		}
		config_vec.push(config);
	}

	let stat_check_interval = 3;
	let mut deadline = time::get_time().sec + duration_in_seconds;
	let mut next_stat_check = time::get_time().sec + stat_check_interval;
	let mut stats_updated=false;
	//for CI testing on slower servers
	//if we're trying to quit and there are no stats yet, keep going for a bit
	let mut extra_time=false;
	let extra_time_value=600;

	while time::get_time().sec < deadline {

		println!("Test mining for {} seconds, looking for difficulty > 0", duration_in_seconds);
		let mut i=0;
		for c in config_vec.clone().into_iter(){
			println!("Plugin {}: {}", i, c.plugin_full_path);
			i+=1;
		}

		// these always get consumed after a notify
		let miner = CuckooMiner::new(config_vec.clone()).expect("");
		let job_handle = miner.notify(1, pre_header, post_header, 0).unwrap();

		loop {
			if let Some(s) = job_handle.get_solution() {
				println!("Sol found: {}, {:?}", s.get_nonce_as_u64(), s);
				// up to you to read it and check difficulty
				continue;
			}
			if time::get_time().sec >= next_stat_check {
				let mut sps_total=0.0;
				for index in 0..config_vec.len() {
					let stats_vec=job_handle.get_stats(index);
					if let Err(e) = stats_vec {
						panic!("Error getting stats: {:?}", e);
					}
					for s in stats_vec.unwrap().into_iter() {
						if s.in_use == 0 {continue;}
						let status = match s.has_errored {
							0 => "OK",
							_ => "ERRORED", 
						};
						let last_solution_time_secs = s.last_solution_time as f64 / 1000000000.0;
						let last_hashes_per_sec = 1.0 / last_solution_time_secs;
						println!("Plugin 0 - Device {} ({}) Status: {} - Last Graph time: {}; Graphs per second: {:.*} \
						- Total Attempts {}", 
						s.device_id, s.device_name, status, last_solution_time_secs, 3, last_hashes_per_sec,
						s.iterations_completed);
						if last_hashes_per_sec.is_finite() {
							sps_total+=last_hashes_per_sec;
						}
						if last_solution_time_secs > 0.0 {
							stats_updated = true;
						}
						i+=1;
					}
				}
				println!("Total solutions per second: {}", sps_total);
				next_stat_check = time::get_time().sec + stat_check_interval;
			}
			if time::get_time().sec > deadline {
				if !stats_updated && !extra_time {
					extra_time=true;
					deadline+=extra_time_value;
					println!("More time needed");
				} else {
					println!("Stopping jobs and waiting for cleanup");
					job_handle.stop_jobs();
					break;
				}
			}
			if stats_updated && extra_time {
				break;
			}
			//avoid busy wait 
			let sleep_dur = std::time::Duration::from_millis(100);
			std::thread::sleep(sleep_dur);

		}
		if stats_updated && extra_time {
			break;
		}
	}
	assert!(stats_updated==true);
}

fn main() {
	let mut global_config = GlobalConfig::new(None).unwrap_or_else(|e| {
		panic!("Error parsing config file: {}", e);
	});
	println!("Starting Grin-Miner from config file at: {}", 
		global_config.config_file_path.unwrap().to_str().unwrap());
	// initialise the logger
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

	let conn = TcpStream::connect(mining_config.stratum_server_addr).unwrap();
	conn.set_nonblocking(true)
		.expect("Failed to set TcpStream to non-blocking");
	let mut stream = BufStream::new(conn);

	//TODO: Mining plugin params, config, logging, etc
	//just use mean miner for now
	let mut params=Vec::new();
	params.push((String::from("NUM_THREADS"),0,8));
	let caps = get_plugin_vec("mean_cpu_30");
	let mut plugin_path_vec:Vec<&str> = Vec::new();
	for c in &caps {
		plugin_path_vec.push(&c.full_path);
	}

	//Main loop to listen for stratum requests
	loop {
		thread::sleep(std::time::Duration::from_secs(1));
		let mut message = String::new();
		let result = stream.read_line(&mut message);
		if let Err(_e) = result {
			//TODO: handle these properly, right now this is getting os error 11
			//if there's no message available
			//println!("Error reading stream: {}", e);
		} 
		if message=="" {
			continue;
		}

		debug!(LOGGER, "Received message: {}", message);
		let request:RpcRequest = match serde_json::from_str(&message) {
			Ok(r) => r,
			Err(e) => {
				println!("invalid request received: {}", e);
				continue;
				//TODO: Handle properly
			}

		};

		println!("Request: {:?}", request);
		let header_params = request.params.unwrap();
		let header_params:JobTemplate = serde_json::from_str(&header_params).unwrap();
		mine_async(plugin_path_vec.clone(), 60, &header_params.pre_pow, "", Some(params.clone()));
		
	}
}
