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

//! Public types for config modules

use std::path::PathBuf;
use std::{io, fmt};
use std::collections::HashMap;

use util;

/// CuckooMinerPlugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuckooMinerPluginConfig {
	/// The type of plugin to load (i.e. filters on filename)
	pub type_filter: String,

	/// Cuckoo size (edge bits) for the plugin
	pub edge_bits: u8,

	/// device params
	pub device_parameters: Option<HashMap<String, HashMap<String, u32>>>,
}

impl Default for CuckooMinerPluginConfig {
	fn default() -> CuckooMinerPluginConfig {
		CuckooMinerPluginConfig {
			type_filter: String::new(),
			edge_bits: 30,
			device_parameters: None,
		}
	}
}

/// Error type wrapping config errors.
#[derive(Debug)]
pub enum ConfigError {
	/// Error with parsing of config file
	ParseError(String, String),

	/// Error with fileIO while reading config file
	FileIOError(String, String),

	/// No file found
	FileNotFoundError(String),

	/// Error serializing config values
	SerializationError(String),
}

impl fmt::Display for ConfigError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			ConfigError::ParseError(ref file_name, ref message) => write!(
				f,
				"Error parsing configuration file at {} - {}",
				file_name, message
			),
			ConfigError::FileIOError(ref file_name, ref message) => {
				write!(f, "{} {}", message, file_name)
			}
			ConfigError::FileNotFoundError(ref file_name) => {
				write!(f, "Configuration file not found: {}", file_name)
			}
			ConfigError::SerializationError(ref message) => {
				write!(f, "Error serializing configuration: {}", message)
			}
		}
	}
}

impl From<io::Error> for ConfigError {
	fn from(error: io::Error) -> ConfigError {
		ConfigError::FileIOError(
			String::from(""),
			String::from(format!("Error loading config file: {}", error)),
		)
	}
}

/// basic mining configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerConfig {
	/// Whether to run the tui
	pub run_tui: bool,

	/// mining loop by adding a sleep to the thread
	pub stratum_server_addr: String,

	/// login for the stratum server
	pub stratum_server_login: Option<String>,

	/// password for the stratum server
	pub stratum_server_password: Option<String>,

	/// plugin dir
	pub miner_plugin_dir: Option<String>,

	/// whether to hash the whole header sent to the plugin
	/// (for testnet2 and previous compatibility)
	pub hash_header: Option<bool>,

	/// Cuckoo miner plugin configuration, one for each plugin
	pub miner_plugin_config: Vec<CuckooMinerPluginConfig>,
}

impl Default for MinerConfig {
	fn default() -> MinerConfig {
		MinerConfig {
			run_tui: false,
			miner_plugin_dir: None,
			miner_plugin_config: vec![],
			stratum_server_addr: String::from("http://127.0.0.1:13416"),
			stratum_server_login: None,
			stratum_server_password: None,
			hash_header: None,
		}
	}
}

/// separately for now, then put them together as a single
/// ServerConfig object afterwards. This is to flatten
/// out the configuration file into logical sections,
/// as they tend to be quite nested in the code
/// Most structs optional, as they may or may not
/// be needed depending on what's being run
#[derive(Debug, Serialize, Deserialize)]
pub struct GlobalConfig {
	/// Keep track of the file we've read
	pub config_file_path: Option<PathBuf>,
	/// keep track of whether we're using
	/// a config file or just the defaults
	/// for each member
	pub using_config_file: bool,
	/// Global member config
	pub members: Option<ConfigMembers>,
}

/// Keeping an 'inner' structure here, as the top
/// level GlobalConfigContainer options might want to keep
/// internal state that we don't necessarily
/// want serialised or deserialised
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigMembers {
	/// Server config
	/// Mining config
	pub mining: MinerConfig,
	/// Logging config
	pub logging: Option<util::types::LoggingConfig>,
}
