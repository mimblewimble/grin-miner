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

//! Public Types used for cuckoo-miner module

use plugin::SolverParams;
use std::env;
use std::{fmt, io};
use {CuckooMinerError, PluginLibrary};

pub static SO_SUFFIX: &str = ".cuckooplugin";

/// CuckooMinerPlugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
	/// The filename of the plugin to load
	pub name: String,

	/// device params
	pub params: SolverParams,
}

impl PluginConfig {
	/// create new!
	pub fn new(name: &str) -> Result<PluginConfig, CuckooMinerError> {
		// Ensure it exists and get default parameters
		let mut p_path = env::current_exe().unwrap();
		p_path.pop();
		// cargo test exes are a directory further down
		if p_path.ends_with("deps") {
			p_path.pop();
		}
		p_path.push("plugins");
		p_path.push(format!("{}{}", name, SO_SUFFIX).as_str());
		let l = PluginLibrary::new(p_path.to_str().unwrap())?;
		let params = l.get_default_params();
		l.unload();
		Ok(PluginConfig {
			name: name.to_owned(),
			params: params,
		})
	}
}

/// Error type wrapping config errors.
#[derive(Debug)]
#[allow(dead_code)]
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
