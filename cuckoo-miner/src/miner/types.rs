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

//! Miner types
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use CuckooMinerError;
use {PluginConfig, PluginLibrary, SolverSolutions, SolverStats};

static SO_SUFFIX: &str = ".cuckooplugin";

pub type JobSharedDataType = Arc<RwLock<JobSharedData>>;
pub type JobControlDataType = Arc<RwLock<JobControlData>>;

/// Holds a loaded lib + config + stats
/// 1 instance = 1 device on 1 controlling thread
pub struct SolverInstance {
	/// The loaded plugin
	pub lib: PluginLibrary,
	/// Associated config
	pub config: PluginConfig,
	/// Last stats output
	pub stats: SolverStats,
	/// Last solution output
	pub solutions: SolverSolutions,
}

impl SolverInstance {
	/// Create a new solver instance with the given config
	pub fn new(config: PluginConfig) -> Result<SolverInstance, CuckooMinerError> {
		let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
		d.push(format!("../target/debug/plugins/{}{}", config.name, SO_SUFFIX).as_str());
		let l = PluginLibrary::new(d.to_str().unwrap())?;
		Ok(SolverInstance {
			lib: l,
			config: config,
			stats: SolverStats::default(),
			solutions: SolverSolutions::default(),
		})
	}

	/// Release the lib
	pub fn unload(&mut self) {
		self.lib.unload();
	}
}

/// Data intended to be shared across threads
pub struct JobSharedData {
	/// ID of the current running job (not currently used)
	pub job_id: u32,

	/// The part of the header before the nonce, which this
	/// module will mutate in search of a solution
	pub pre_nonce: String,

	/// The part of the header after the nonce
	pub post_nonce: String,

	/// The target difficulty. Only solutions >= this
	/// target will be put into the output queue
	pub difficulty: u64,

	/// Output solutions
	pub solutions: Vec<SolverSolutions>,

	/// Current stats
	pub stats: Vec<SolverStats>,
}

impl Default for JobSharedData {
	fn default() -> JobSharedData {
		JobSharedData {
			job_id: 0,
			pre_nonce: String::from(""),
			post_nonce: String::from(""),
			difficulty: 0,
			solutions: Vec::new(),
			stats: vec![],
		}
	}
}

impl JobSharedData {
	pub fn new(
		num_solvers: usize,
	) -> JobSharedData {
		JobSharedData {
			job_id: 0,
			pre_nonce: String::from(""),
			post_nonce: String::from(""),
			difficulty: 1,
			solutions: Vec::new(),
			stats: vec![SolverStats::default(); num_solvers],
		}
	}
}

/// an internal structure to flag job control
pub struct JobControlData {
	/// Stop solvers, pull down miners, exit
	pub stop_flag: bool,

	/// Pause all processing
	pub paused: bool,

	/// monitor whether processing stop flag should be sent
	/// (for one time stop signal only on pause)
	pub pause_signal: bool,
}

impl Default for JobControlData {
	fn default() -> JobControlData {
		JobControlData {
			stop_flag: false,
			paused: true,
			pause_signal: false,
		}
	}
}
