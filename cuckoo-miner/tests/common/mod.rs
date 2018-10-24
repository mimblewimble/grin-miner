
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
			if let Some(solutions) = miner.get_solutions() {
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
