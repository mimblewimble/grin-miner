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

//! Miner stats collection types, to be used by tests, logging or GUI/TUI
//! to collect information about mining status

/// Struct to return relevant information about the mining process
/// back to interested callers (such as the TUI)
use plugin;

#[derive(Clone)]
pub struct SolutionStats {
	/// total solutions found
	pub num_solutions_found: u32,
	/// total shares accepted
	pub num_shares_accepted: u32,
	/// total solutions rejected
	pub num_rejected: u32,
	/// total solutions staled
	pub num_staled: u32,
	/// total blocks found
	pub num_blocks_found: u32,
}

impl Default for SolutionStats {
	fn default() -> SolutionStats {
		SolutionStats {
			num_solutions_found: 0,
			num_shares_accepted: 0,
			num_rejected: 0,
			num_staled: 0,
			num_blocks_found: 0,
		}
	}
}

#[derive(Clone)]
pub struct MiningStats {
	/// combined graphs per second
	combined_gps: Vec<f64>,
	/// what block height we're mining at
	pub block_height: u64,
	/// current target for share difficulty we're working on
	pub target_difficulty: u64,
	/// solution statistics
	pub solution_stats: SolutionStats,
	/// Individual device status from Cuckoo-Miner
	pub device_stats: Vec<plugin::SolverStats>,
}

impl Default for MiningStats {
	fn default() -> MiningStats {
		MiningStats {
			combined_gps: vec![],
			block_height: 0,
			target_difficulty: 0,
			solution_stats: SolutionStats::default(),
			device_stats: vec![],
		}
	}
}

impl MiningStats {
	pub fn add_combined_gps(&mut self, val: f64) {
		self.combined_gps.insert(0, val);
		self.combined_gps.truncate(50);
	}

	pub fn combined_gps(&self) -> f64 {
		if self.combined_gps.is_empty() {
			0.0
		} else {
			let sum: f64 = self.combined_gps.iter().sum();
			sum / (self.combined_gps.len() as f64)
		}
	}
}

#[derive(Clone)]
pub struct ClientStats {
	/// Server we're connected to
	pub server_url: String,
	/// whether we're connected
	pub connected: bool,
	/// Connection status
	pub connection_status: String,
	/// Last message sent to server
	pub last_message_sent: String,
	/// Last response/command received from server
	pub last_message_received: String,
}

impl Default for ClientStats {
	fn default() -> ClientStats {
		ClientStats {
			server_url: "".to_string(),
			connected: false,
			connection_status: "Connection Status: Starting".to_string(),
			last_message_sent: "Last Message Sent: None".to_string(),
			last_message_received: "Last Message Received: None".to_string(),
		}
	}
}

#[derive(Clone)]
pub struct Stats {
	/// Client/networking stats
	pub client_stats: ClientStats,
	/// Mining stats
	pub mining_stats: MiningStats,
}

impl Default for Stats {
	fn default() -> Stats {
		Stats {
			client_stats: ClientStats::default(),
			mining_stats: MiningStats::default(),
		}
	}
}
