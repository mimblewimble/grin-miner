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

//! Main interface for the cuckoo_miner plugin manager, which queries
//! all available plugins in a particular directory and returns their
//! descriptions, parameters, and capabilities.
//!
//! #Example
//! ```
//! extern crate cuckoo_miner as cuckoo;
//! use std::path::PathBuf;
//! let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
//! d.push("target/debug/plugins/");
//!
//! // load plugin manager
//! let mut plugin_manager = cuckoo::CuckooPluginManager::new().unwrap();
//! plugin_manager
//! 	.load_plugin_dir(String::from(d.to_str().unwrap()))
//! 	.expect("");
//!
//!	let caps = plugin_manager.get_available_plugins("").unwrap();
//! //Print all available plugins
//! for c in &caps {
//!    println!("Found plugin: [{}]", c);
//! }
/// ```

use std::fmt;
use std::env;
use std::path::Path;

use regex::Regex;
use glob::glob;

use serde_json;

use cuckoo_sys::ffi::PluginLibrary;
use error::error::CuckooMinerError;

// OS-specific library extensions

static DLL_SUFFIX: &str = "cuckooplugin";

// Helper function to get the absolute path from a relative path

fn abspath<P: AsRef<Path> + ?Sized>(relpath: &P) -> String {
	let result = env::current_dir().map(|p| p.join(relpath.as_ref()));
	let full_path = result.unwrap();
	String::from(full_path.to_str().unwrap())
}

/// A wrapper for details that a plugin can report via it's cuckoo_description
/// function. Basic at the moment, but will be extended.
#[derive(Debug, Clone)]
pub struct CuckooPluginCapabilities {
	/// The full path to the plugin
	pub full_path: String,

	/// The plugin's file name
	pub file_name: String,
}

impl Default for CuckooPluginCapabilities {
	fn default() -> CuckooPluginCapabilities {
		CuckooPluginCapabilities {
			full_path: String::from(""),
			file_name: String::from(""),
		}
	}
}

impl fmt::Display for CuckooPluginCapabilities {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(
			f,
			"Path:{}\nParameters:{}\n",
			self.full_path,
			serde_json::to_string(&self.parameters).unwrap()
		)
	}
}

pub struct CuckooPluginManager {
	// The current directory
	plugin_dir: String,

	// Holds the current set of plugins
	current_plugin_caps: Option<Vec<CuckooPluginCapabilities>>,
}

impl Default for CuckooPluginManager {
	fn default() -> CuckooPluginManager {
		CuckooPluginManager {
			plugin_dir: String::from("target/debug"),
			current_plugin_caps: None,
		}
	}
}

impl CuckooPluginManager {
	/// #Description
	///
	/// Returns a new CuckooPluginManager. The default value of the
	/// plugin directory is "target/debug" to correspond with cargo's
	/// default location.
	///
	/// #Arguments
	///
	/// None
	///
	/// #Returns
	///
	/// * `Ok` if successful 
	/// * a [CuckooMinerError](enum.CuckooMinerError.html)
	/// with specific detail if an error is encountered.
	///

	pub fn new() -> Result<CuckooPluginManager, CuckooMinerError> {
		Ok(CuckooPluginManager::default())
	}

	/// #Description
	///
	/// Loads all available plugins in the specified directory one by one,
	/// calls their cuckoo_description functions, and stores an internal vector
	/// of [CuckooPluginCapabilities](struct.CuckooPluginCapabilities.html)
	/// representing the plugins in the directory. This will parse any file
	/// with the extension `.cuckooplugin`.
	///
	/// #Arguments
	///
	/// * `plugin_dir` (IN) The path to the prefered plugin directory. This can
	/// be either relative to the current directory or a full path. This will
	/// be resolved to a full path before calling each plugin.
	///
	/// #Returns
	///
	/// * `Ok` if successful 
	/// * [CuckooMinerError](enum.CuckooMinerError.html)
	/// with specific detail if an error is encountered. 
	///

	pub fn load_plugin_dir(&mut self, plugin_dir: String) -> Result<(), CuckooMinerError> {
		self.plugin_dir = plugin_dir.clone();
		let caps = self.load_all_plugin_caps(&plugin_dir)?;
		self.current_plugin_caps = Some(caps);
		Ok(())
	}

	/// #Description
	///
	/// Returns an list of
	/// [CuckooPluginCapabilities](struct.CuckooPluginCapabilities.html)
	/// representing the currently installed plugins in the currently 
	/// loaded directory. Can optionally take a filter,
	/// which will limit the returned plugins to those with the occurrence of a
	/// particular string in their name.
	///
	/// #Arguments
	///
	/// * `filter` If an empty string, return all of the plugins found in the
	/// directory. Otherwise, only return plugins containing a occurrence of this 
	/// string in their filename.
	///
	/// #Returns
	///
	/// *If successful, a Result containing a vector of
	/// [CuckooPluginCapabilities](struct.CuckooPluginCapabilities.html) ,
	/// one for each plugin successfully read from the plugin directory,
	/// filtered as requested.
	/// *If there is an error loading plugins from the given directory,
	/// a [CuckooMinerError](enum.CuckooMinerError.html)
	/// will be returned outlining more specific details.
	///
	/// #Example
	///
	/// ```
	/// extern crate cuckoo_miner as cuckoo;
  /// use std::path::PathBuf;	
	/// let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	/// d.push("target/debug/plugins/");
	///
	/// // load plugin manager
	/// let mut plugin_manager = cuckoo::CuckooPluginManager::new().unwrap();
	/// plugin_manager
	/// 	.load_plugin_dir(String::from(d.to_str().unwrap()))
	/// 	.expect("");
	///
	///	let caps = plugin_manager.get_available_plugins("").unwrap();
	///
	/// //Print all available plugins
	/// for c in &caps {
	///     println!("Found plugin: [{}]", c);
	/// }
	/// ```
	///

	pub fn get_available_plugins(
		&mut self,
		filter: &str,
	) -> Result<Vec<CuckooPluginCapabilities>, CuckooMinerError> {
		if filter.len() == 0 {
			return Ok(self.current_plugin_caps.as_mut().unwrap().clone());
		} else {
			let result = self.current_plugin_caps
				.as_mut()
				.unwrap()
				.clone()
				.into_iter()
				.filter(|ref i| {
					let re = Regex::new(&format!(r"{}", filter)).unwrap();
					let caps = re.captures(&i.full_path);
					match caps {
						Some(_) => return true,
						None => return false,
					}
				})
				.collect::<Vec<_>>();
			if result.len() == 0 {
				return Err(CuckooMinerError::NoPluginsFoundError(
					format!("For given filter: {}", filter),
				));
			}
			return Ok(result);
		}
	}

	/// Fills out and Returns a CuckooPluginCapabilities structure parsed from a
	/// call to cuckoo_description in the currently loaded plugin

	fn load_plugin_caps(
		&mut self,
		full_path: String,
	) -> Result<CuckooPluginCapabilities, CuckooMinerError> {
		debug!("Querying plugin at {}", full_path);
		let library = PluginLibrary::new(&full_path).unwrap();
		let mut caps = CuckooPluginCapabilities::default();

		caps.full_path = full_path.clone();
		caps.file_name = String::from("");

		let mut param_list_bytes: [u8; 4096] = [0; 4096];
		let mut param_list_len = param_list_bytes.len() as u32;
		// get a list of parameters
		library.call_cuckoo_parameter_list(&mut param_list_bytes, &mut param_list_len);
		let mut param_list_vec: Vec<u8> = Vec::new();
		// result contains null zero
		for i in 0..param_list_len {
			param_list_vec.push(param_list_bytes[i as usize].clone());
		}
		let param_list_json = String::from_utf8(param_list_vec)?;
		caps.parameters = serde_json::from_str(&param_list_json).unwrap();

		library.unload();

		return Ok(caps);
	}

	/// Loads and fills out the internal plugin capabilites vector from the
	/// given directory.

	fn load_all_plugin_caps(
		&mut self,
		plugin_dir: &str,
	) -> Result<Vec<CuckooPluginCapabilities>, CuckooMinerError> {
		let lib_full_path = abspath(Path::new(&plugin_dir));
		let glob_search_path = format!("{}/*.{}", lib_full_path, DLL_SUFFIX);

		let mut result_vec: Vec<CuckooPluginCapabilities> = Vec::new();

		for entry in glob(&glob_search_path).expect("Failed to read glob pattern") {
			match entry {
				Ok(path) => {
					let caps = self.load_plugin_caps(String::from(path.to_str().unwrap()))?;
					result_vec.push(caps);
				}
				Err(e) => error!("{:?}", e),
			}
		}

		if result_vec.len() == 0 {
			return Err(CuckooMinerError::NoPluginsFoundError(format!(
				"No plugins found in plugin directory {}",
				lib_full_path.clone()
			)));
		}

		Ok(result_vec)
	}
}
