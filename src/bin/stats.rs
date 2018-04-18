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

//! Miner stats collection types, to be used by tests, logging or GUI/TUI
//! to collect information about mining status

/// Struct to return relevant information about the mining process
/// back to interested callers (such as the TUI)
 
use util;

#[derive(Clone)]
pub struct MiningStats {
	/// Server we're connected to
	pub server_url: String,
	/// combined graphs per second
	pub combined_gps: f64,
	/// what block height we're mining at
	pub block_height: u64,
	/// current network difficulty we're working on
	pub network_difficulty: u64,
	/// cuckoo size used for mining
	pub cuckoo_size: u16,
	/// Individual device status from Cuckoo-Miner
	pub device_stats: Option<Vec<Vec<util::cuckoo_miner::CuckooMinerDeviceStats>>>,
}

impl Default for MiningStats {
	fn default() -> MiningStats{
		MiningStats{
			server_url: "".to_string(),
			combined_gps: 0.0,
			block_height: 0,
			network_difficulty: 0,
			cuckoo_size: 0,
			device_stats: None,
		}
	}
}
