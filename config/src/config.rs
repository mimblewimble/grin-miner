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

//! Configuration file management

use std::env;
use std::io::Read;
use std::path::PathBuf;
use std::fs::File;

use toml;
use types::MinerConfig;
use util::{LoggingConfig, LOGGER};
use types::{ConfigError, ConfigMembers, GlobalConfig, GrinMinerPluginConfig};
use cuckoo::{PluginConfig, CuckooMinerError};

extern crate dirs;

/// The default file name to use when trying to derive
/// the config file location

const CONFIG_FILE_NAME: &'static str = "grin-miner.toml";
const GRIN_HOME: &'static str = ".grin";

/// resolve a read parameter to a solver param, (or not if it isn't found)
fn resolve_param(config: &mut PluginConfig, name: &str, value: u32) {
	match name {
		"nthreads" => config.params.nthreads = value,
		"ntrims" => config.params.ntrims = value,
		"cpuload" => config.params.cpuload = match value {
			1 => true,
			_ => false,
		},
		"device" => config.params.device = value,
		"blocks" => config.params.blocks = value,
		"tbp" => config.params.tpb = value,
		"expand" => config.params.expand = value,
		"genablocks" => config.params.genablocks = value,
		"genatpb" => config.params.genatpb = value,
		"genbtpb" => config.params.genbtpb = value,
		"trimtpb" => config.params.trimtpb = value,
		"tailtpb" => config.params.tailtpb = value,
		"recoverblocks" => config.params.recoverblocks = value,
		"recovertpb" => config.params.recovertpb = value,
		n => {
				warn!(LOGGER, "Configuration param: {} unknown. Ignored.", n);
			},
	};
}

/// Transforms a set of grin-miner plugin configs to cuckoo-miner plugins configs
pub fn read_configs(conf_in: Vec<GrinMinerPluginConfig>) -> Result<Vec<PluginConfig>, CuckooMinerError> {
	let mut return_vec = vec![];
	for conf in conf_in {
		let res = PluginConfig::new(&conf.plugin_name);
		match res {
			Err(e) => {
				error!(LOGGER, "Error reading plugin config: {:?}", e);
				return Err(e);
			},
			Ok(mut c) => {
				if conf.parameters.is_some() {
					let params = conf.parameters.unwrap();
					for k in params.keys() {
						resolve_param(&mut c, k, *params.get(k).unwrap());
					}
				}
				return_vec.push(c)
			}
		}
	}
	Ok(return_vec)
}

/// Returns the defaults, as strewn throughout the code
impl Default for ConfigMembers {
	fn default() -> ConfigMembers {
		ConfigMembers {
			mining: MinerConfig::default(),
			logging: Some(LoggingConfig::default()),
		}
	}
}

impl Default for GlobalConfig {
	fn default() -> GlobalConfig {
		GlobalConfig {
			config_file_path: None,
			using_config_file: false,
			members: Some(ConfigMembers::default()),
		}
	}
}

impl GlobalConfig {
	/// Need to decide on rules where to read the config file from,
	/// but will take a stab at logic for now

	fn derive_config_location(&mut self) -> Result<(), ConfigError> {
		// First, check working directory
		let mut config_path = env::current_dir().unwrap();
		config_path.push(CONFIG_FILE_NAME);
		if config_path.exists() {
			self.config_file_path = Some(config_path);
			return Ok(());
		}
		// Next, look in directory of executable
		let mut config_path = env::current_exe().unwrap();
		config_path.pop();
		config_path.push(CONFIG_FILE_NAME);
		if config_path.exists() {
			self.config_file_path = Some(config_path);
			return Ok(());
		}
		// Then look in {user_home}/.grin
		let config_path = dirs::home_dir();
		if let Some(mut p) = config_path {
			p.push(GRIN_HOME);
			p.push(CONFIG_FILE_NAME);
			if p.exists() {
				self.config_file_path = Some(p);
				return Ok(());
			}
		}

		// Give up
		Err(ConfigError::FileNotFoundError(String::from("")))
	}

	/// Takes the path to a config file, or if NONE, tries
	/// to determine a config file based on rules in
	/// derive_config_location

	pub fn new(file_path: Option<&str>) -> Result<GlobalConfig, ConfigError> {
		let mut return_value = GlobalConfig::default();
		if let Some(fp) = file_path {
			return_value.config_file_path = Some(PathBuf::from(&fp));
		} else {
			let _result = return_value.derive_config_location();
		}

		// No attempt at a config file, just return defaults
		if let None = return_value.config_file_path {
			return Ok(return_value);
		}

		// Config file path is given but not valid
		if !return_value.config_file_path.as_mut().unwrap().exists() {
			return Err(ConfigError::FileNotFoundError(String::from(
				return_value
					.config_file_path
					.as_mut()
					.unwrap()
					.to_str()
					.unwrap()
					.clone(),
			)));
		}

		// Try to parse the config file if it exists
		// explode if it does exist but something's wrong
		// with it
		return_value.read_config()
	}

	/// Read config
	pub fn read_config(mut self) -> Result<GlobalConfig, ConfigError> {
		let mut file = File::open(self.config_file_path.as_mut().unwrap())?;
		let mut contents = String::new();
		file.read_to_string(&mut contents)?;
		let decoded: Result<ConfigMembers, toml::de::Error> = toml::from_str(&contents);
		match decoded {
			Ok(gc) => {
				// Put the struct back together, because the config
				// file was flattened a bit
				self.using_config_file = true;
				self.members = Some(gc);
				return Ok(self);
			}
			Err(e) => {
				return Err(ConfigError::ParseError(
					String::from(
						self.config_file_path
							.as_mut()
							.unwrap()
							.to_str()
							.unwrap()
							.clone(),
					),
					String::from(format!("{}", e)),
				));
			}
		}
	}

	/// Serialize config
	pub fn ser_config(&mut self) -> Result<String, ConfigError> {
		let encoded: Result<String, toml::ser::Error> =
			toml::to_string(self.members.as_mut().unwrap());
		match encoded {
			Ok(enc) => return Ok(enc),
			Err(e) => {
				return Err(ConfigError::SerializationError(String::from(format!(
					"{}",
					e
				))));
			}
		}
	}
}
