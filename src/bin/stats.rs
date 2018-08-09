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
	/// combined graphs per second
	pub combined_gps: f64,
	/// what block height we're mining at
	pub block_height: u64,
	/// current target for share difficulty we're working on
	pub target_difficulty: u64,
	/// cuckoo size used for mining
	pub cuckoo_size: u16,
	/// Individual device status from Cuckoo-Miner
	pub device_stats: Option<Vec<Vec<util::cuckoo_miner::CuckooMinerDeviceStats>>>,
}

impl Default for MiningStats {
	fn default() -> MiningStats{
		MiningStats{
			combined_gps: 0.0,
			block_height: 0,
			target_difficulty: 0,
			cuckoo_size: 0,
			device_stats: None,
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
