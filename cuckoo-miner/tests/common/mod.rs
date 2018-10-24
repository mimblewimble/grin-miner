
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
//

//! Common values and functions that can be used in all mining tests

extern crate cuckoo_miner as cuckoo;
extern crate time;

use std;
use self::cuckoo::{CuckooMiner, PluginConfig};

// Grin Pre and Post headers, into which a nonce is to be insterted for mutation
pub const SAMPLE_GRIN_PRE_HEADER_1:&str = "00000000000000118e0fe6bcfaa76c6795592339f27b6d330d8f9c4ac8e86171a66357d1\
    d0fce808000000005971f14f0000000000000000000000000000000000000000000000000000000000000000\
    3e1fcdd453ce51ffbb16dd200aeb9ef7375aec196e97094868428a7325e4a19b00";

pub const SAMPLE_GRIN_POST_HEADER_1:&str = "010a020364";

/*extern crate time;
extern crate rand;

use std::path::PathBuf;
use std::fmt::Write;
use std;

use common::rand::Rng;

use self::cuckoo::{PluginLibrary,
	PluginConfig,
	SolverCtx,
	SolverParams,
	SolverStats,
	SolverSolutions};

// Encode the provided bytes into a hex string
pub fn to_hex(bytes: Vec<u8>) -> String {
	let mut s = String::new();
	for byte in bytes {
		write!(&mut s, "{:02x}", byte).expect("Unable to write");
	}
	s
}

//Helper to convert from hex string
//avoids a lot of awkward byte array initialisation below
pub fn _from_hex_string(in_str: &str) -> Vec<u8> {
	let mut bytes = Vec::new();
	for i in 0..(in_str.len() / 2) {
		let res = u8::from_str_radix(&in_str[2 * i..2 * i + 2], 16);
		match res {
			Ok(v) => bytes.push(v),
			Err(e) => println!("Problem with hex: {}", e),
		}
	}
	bytes
}

pub const _DLL_SUFFIX: &str = ".cuckooplugin";

pub const _TEST_PLUGIN_LIBS_CORE : [&str;3] = [
	"lean_cpu_16",
	"lean_cpu_30",
	"mean_cpu_30",
];

pub const _TEST_PLUGIN_LIBS_OPTIONAL : [&str;1] = [
	"lean_cuda_30",
];


// Grin Pre and Post headers, into which a nonce is to be insterted for mutation
pub const SAMPLE_GRIN_PRE_HEADER_1:&str = "00000000000000118e0fe6bcfaa76c6795592339f27b6d330d8f9c4ac8e86171a66357d1\
    d0fce808000000005971f14f0000000000000000000000000000000000000000000000000000000000000000\
    3e1fcdd453ce51ffbb16dd200aeb9ef7375aec196e97094868428a7325e4a19b00";

pub const SAMPLE_GRIN_POST_HEADER_1:&str = "010a020364";

//hashes known to return a solution at cuckoo 30 and 16
pub const KNOWN_30_HASH_1:&str = "11c5059b4d4053131323fdfab6a6509d73ef22\
9aedc4073d5995c6edced5a3e6";

pub const KNOWN_16_HASH_1:&str = "c008b9ff7292fdacef0efbdff73d1db66674ff\
3b6dea6cca670c85b6a110f0b2";

pub fn get_random_hash() -> [u8;32] {
	let mut ret_val:[u8;32] = [0;32];
	for i in 0..32 {
		ret_val[i]=rand::OsRng::new().unwrap().gen();
	};
	ret_val
}

// Helper to load plugin
pub fn load_plugin(filter: &str) -> PluginLibrary {
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

// Helper function, mines a plugin once
pub fn mine_once(full_path:&str, params:Option<Vec<(String, u32, u32)>>) {

	let mut config_vec=Vec::new();
	let mut config = CuckooMinerConfig::new();
	config.plugin_full_path = String::from(full_path);

	if let Some(p) = params {
		config.parameter_list = p;
	}

	config_vec.push(config);
	for c in config_vec.clone().into_iter(){
		println!("Plugin (Mine once): {}", c.plugin_full_path);
	}

	let miner = CuckooMiner::new(config_vec.clone()).expect("");
	let header:[u8; 32] = get_random_hash();
	let mut cuckoo_size = 0;
	let mut solution = CuckooMinerSolution::new();
	let result = miner.mine(&header, &mut cuckoo_size, &mut solution, 0).unwrap();
}

// Helper function, tests a particular miner implementation against a known set
pub fn mine_sync_for_duration(full_path:&str, duration_in_seconds: i64, params:Option<Vec<(String, u32, u32)>>) {
	let mut config_vec=Vec::new();
	let mut config = CuckooMinerConfig::new();
	config.plugin_full_path = String::from(full_path);

	if let Some(p) = params {
		config.parameter_list = p;
	}

	config_vec.push(config);

	let stat_check_interval = 3;
	let deadline = time::get_time().sec + duration_in_seconds;
	let mut next_stat_check = time::get_time().sec + stat_check_interval;

	let mut i:u64=0;
	println!("Test mining for {} seconds, looking for difficulty > 0", duration_in_seconds);
	for c in config_vec.clone().into_iter(){
		println!("Plugin (Sync Mode): {}", c.plugin_full_path);
	}
	let miner = CuckooMiner::new(config_vec.clone()).expect("");
	while time::get_time().sec < deadline {
		let mut iterations=0;
		let mut solution = CuckooMinerSolution::new();
		loop {
			let header:[u8; 32] = get_random_hash();
			let mut cuckoo_size = 0;
			//Mine on plugin loaded at index 0
			let result = miner.mine(&header, &mut cuckoo_size, &mut solution, 0).unwrap();
			iterations+=1;
			if result == true {
				println!("Solution found after {} iterations: {}", i, solution);
				println!("For hash: {:?}", to_hex(header.to_vec()));
				i=0;
				break;
			}
			if time::get_time().sec > deadline {
				println!("Exiting after {} iterations", iterations);
				break;
			}
			if time::get_time().sec >= next_stat_check {
				let stats_vec=miner.get_stats(0).unwrap();
				for s in stats_vec.into_iter() {
					if s.in_use == 0 {continue;}
					let last_solution_time_secs = s.last_solution_time as f64 / 1000000000.0;
					let last_hashes_per_sec = 1.0 / last_solution_time_secs;
					let status = match s.has_errored {
						0 => "OK",
						_ => "ERRORED", 
					};
					println!("Plugin 0 - Device {} ({}) Status: {}, - Last Graph time: {}; Graphs per second: {:.*} \
					- Total Attempts {}", 
					s.device_id, s.device_name, status, last_solution_time_secs, 3, last_hashes_per_sec,
					s.iterations_completed);
				}
				next_stat_check = time::get_time().sec + stat_check_interval;
			}
			i+=1;
		}
	}
}*/

// Helper function, tests a particular miner implementation against a known set
pub fn mine_async_for_duration(configs: &Vec<PluginConfig>, duration_in_seconds: i64) {
	let stat_check_interval = 3;
	let mut deadline = time::get_time().sec + duration_in_seconds;
	let mut next_stat_check = time::get_time().sec + stat_check_interval;
	let mut stats_updated = false;

	//for CI testing on slower servers
	//if we're trying to quit and there are no stats yet, keep going for a bit
	let mut extra_time=false;
	let extra_time_value=600;

	// these always get consumed after a notify
	let mut miner = CuckooMiner::new(configs.clone());
	let _ = miner.start_solvers();

	while time::get_time().sec < deadline {

		println!("Test mining for {} seconds, looking for difficulty > 0", duration_in_seconds);
		let mut i=0;
		for c in configs.clone().into_iter(){
			println!("Plugin {}: {}", i, c.name);
			i+=1;
		}

		miner.notify(1, SAMPLE_GRIN_PRE_HEADER_1, SAMPLE_GRIN_POST_HEADER_1, 0).unwrap();

		loop {
			if let Some(solutions) = miner.get_solution() {
				for i in 0..solutions.num_sols {
					println!("Sol found: {}", solutions.sols[i as usize]);
					continue;
				}
			}
			if time::get_time().sec >= next_stat_check {
				let mut sps_total=0.0;
				let stats_vec=miner.get_stats();
				for s in stats_vec.unwrap().into_iter() {
					let last_solution_time_secs = s.last_solution_time as f64 / 1000000000.0;
					let last_hashes_per_sec = 1.0 / last_solution_time_secs;
					let status = match s.has_errored {
						false => "OK",
						_ => "ERRORED",
					};
					println!("Plugin 0 - Device {} ({}) (Cuck(at)oo{}) - Status: {} - Last Graph time: {}; Graphs per second: {:.*} \
					- Total Attempts: {}",
					s.device_id, s.get_device_name(), s.edge_bits, status, last_solution_time_secs, 3, last_hashes_per_sec, s.iterations);
					if last_hashes_per_sec.is_finite() {
						sps_total+=last_hashes_per_sec;
					}
					if last_solution_time_secs > 0.0 {
						stats_updated = true;
					}
					i+=1;
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
					miner.stop_solvers();
					break;
				}
			}
			if stats_updated && extra_time {
				break;
			}
			//avoid busy wait 
			let sleep_dur = std::time::Duration::from_millis(100);
			std::thread::sleep(sleep_dur);
			if stats_updated && extra_time {
				break;
			}
		}
	}
}
