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

//! Main interface for callers into cuckoo-miner. Provides functionality
//! to load a mining plugin, send it a Cuckoo Cycle POW problem, and
//! return any resulting solutions.

use std::sync::{Arc, RwLock};
use std::{thread, time};
use std::{fmt, cmp};
use std::path::PathBuf;

use byteorder::{ByteOrder, BigEndian};
use blake2::blake2b::Blake2b;

use cuckoo_sys::ffi::{PluginLibrary, SolverParams, SolverSolutions, SolverStats};
use config::types::PluginConfig;
use super::delegator:: {JobSharedData, JobControlData, Delegator};
use error::error::CuckooMinerError;

static SO_SUFFIX: &str = ".cuckooplugin";

// Hardcoded assumption for now that the solution size will be 42 will be
// maintained, to avoid having to allocate memory within the called C functions

const CUCKOO_SOLUTION_SIZE: usize = 42;

/// A simple struct to hold a cuckoo miner solution. Currently,
/// it's assumed that a solution will be 42 bytes. The `solution_nonces`
/// member is statically allocated here, and will be filled in
/// by a plugin upon finding a solution.

#[derive(Copy)]
pub struct CuckooMinerSolution {
	/// Cuckoo size
	pub cuckoo_size: u32,

	/// An array allocated in rust that will be filled
	/// by the called plugin upon successfully finding
	/// a solution
	pub solution_nonces: [u32; CUCKOO_SOLUTION_SIZE],

	/// The nonce that was used to generate the
	/// hash for which a solution was found
	pub nonce: [u8; 8],
}

impl Default for CuckooMinerSolution {
	fn default() -> CuckooMinerSolution {
		CuckooMinerSolution {
			cuckoo_size: 30,
			solution_nonces: [0; CUCKOO_SOLUTION_SIZE],
			nonce: [0; 8],
		}
	}
}

impl Clone for CuckooMinerSolution {
	fn clone(&self) -> CuckooMinerSolution {
		*self
	}
}

impl CuckooMinerSolution {
	/// Creates a new cuckoo miner solution
	/// with nonces set to a u32 array of size
	/// 42 filled with zeroes.

	pub fn new() -> CuckooMinerSolution {
		CuckooMinerSolution::default()
	}

	/// Sets the solution, mostly for testing
	pub fn set_solution(&mut self, nonces: [u32; CUCKOO_SOLUTION_SIZE]) {
		self.solution_nonces = nonces;
	}

	/// return the nonce as a u64, for convenience
	pub fn get_nonce_as_u64(&self) -> u64 {
		BigEndian::read_u64(&self.nonce)
	}

	/// Converts the proof to a vector of u64s
	pub fn to_u64s(&self) -> Vec<u64> {
		let mut nonces = Vec::with_capacity(CUCKOO_SOLUTION_SIZE);
		for n in self.solution_nonces.iter() {
			nonces.push(*n as u64);
		}
		nonces
	}

	/// Returns the has of the solution, as performed in
	/// grin
	pub fn hash(&self) -> [u8; 32] {
		// Hash
		let mut blake2b = Blake2b::new(32);
		for n in 0..self.solution_nonces.len() {
			let mut bytes = [0; 4];
			BigEndian::write_u32(&mut bytes, self.solution_nonces[n]);
			blake2b.update(&bytes);
		}
		let mut ret = [0; 32];
		ret.copy_from_slice(blake2b.finalize().as_bytes());
		ret
	}
}

impl fmt::Display for CuckooMinerSolution {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let mut comma_separated = String::new();

		for num in &self.solution_nonces[0..self.solution_nonces.len()] {
			comma_separated.push_str(&format!("0x{:X}", &num));
			comma_separated.push_str(", ");
		}
		comma_separated.pop();
		comma_separated.pop();

		write!(f, "[{}]", comma_separated)
	}
}

impl fmt::Debug for CuckooMinerSolution {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{:?}", &self.solution_nonces[..])
	}
}

impl cmp::PartialEq for CuckooMinerSolution {
	fn eq(&self, other: &CuckooMinerSolution) -> bool {
		for i in 0..CUCKOO_SOLUTION_SIZE {
			if self.solution_nonces[i] != other.solution_nonces[i] {
				return false;
			}
		}
		return true;
	}
}

/// Structure containing the configuration values to pass into an
/// instance of a miner
#[derive(Debug, Clone)]
pub struct CuckooMinerConfig {
	/// The full path to the plugin to load and use to find a solution
	/// to a POW problem. Defaults to empty string, so must be filled
	/// before use.
	pub plugin_full_path: String,

	/// A parameter list, which differs depending on which
	/// plugin is being called
	pub parameter_list: Vec<(String, u32, u32)>,
}

impl Default for CuckooMinerConfig {
	fn default() -> CuckooMinerConfig {
		CuckooMinerConfig {
			plugin_full_path: String::from(""),
			parameter_list: Vec::new(),
		}
	}
}

impl CuckooMinerConfig {
	/// Returns a new instance of CuckooMinerConfig

	pub fn new() -> CuckooMinerConfig {
		CuckooMinerConfig::default()
	}
}

/// Holds deserialised performance metrics returned from the
/// plugin
///
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CuckooMinerDeviceStats {
	/// The plugin file name (optional so the plugins don't have to deal with it on de/ser)
	pub plugin_name: Option<String>,

	/// The internal device id
	pub device_id: String,

	/// Cuckoo size currently being used by the device
	pub cuckoo_size: String,

	/// The device name
	pub device_name: String,

	/// Whether the device is marked for use
	pub in_use: u32,
 
	/// Whether the device has thrown an error (and has stopped)
	pub has_errored: u32,

	/// The time at which the device last began to search a hash (epoch in
	/// mills)
	pub last_start_time: u64,

	/// The time at which the device last completed a solution search (epoch in
	/// mills)
	pub last_end_time: u64,

	/// The amount of time the last solution search took (epoch in mills)
	pub last_solution_time: u64,

	/// The total number of searched performed since init
	pub iterations_completed: u32,
}

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

/// Handle to the miner's running job, used to read solutions
/// or to control the job. Internal members are not exposed
/// and all interactions should be via public functions
/// This will basically hold an arc reference clone of
/// the Delegator's internal shared data

pub struct CuckooMinerJobHandle {
	/// Data shared across threads
	pub shared_data: Arc<RwLock<JobSharedData>>,

	/// Job control flag
	pub control_data: Arc<RwLock<JobControlData>>,
}

impl CuckooMinerJobHandle {

	/// #Description
	///
	/// Returns a solution if one is currently waiting.
	///
	/// #Returns
	///
	/// If a solution was found and is waiting in the plugin's input queue,
	/// returns
	/// * Ok([CuckooMinerSolution](struct.CuckooMinerSolution.html)) if a
	/// solution is waiting in the queue. 
	/// * None if no solution is waiting

	pub fn get_solution(&self) -> Option<CuckooMinerSolution> {
		// just to prevent endless needless locking of this
		// when using fast test miners, in real cuckoo30 terms
		// this shouldn't be an issue
		// TODO: Make this less blocky
		thread::sleep(time::Duration::from_millis(10));
		// let time_pre_lock=Instant::now();
		let mut s = self.shared_data.write().unwrap();
		// let time_elapsed=Instant::now()-time_pre_lock;
		// println!("Get_solution Time spent waiting for lock: {}",
		// time_elapsed.as_secs()*1000 +(time_elapsed.subsec_nanos()/1_000_000)as u64);
		if s.solutions.len() > 0 {
			let sol = s.solutions.pop().unwrap();
			return Some(sol);
		}
		None
	}

	/// #Description
	///
	/// Returns an vector of [CuckooMinerDeviceStats](struct.CuckooMinerDeviceStats.html)
	/// which will contain information about every device currently mining within the plugin.
	/// In CPU based plugins, this will generally only contain the CPU device, but in plugins
	/// that access multiple devices (such as cuda) the vector will contain information for
	/// each currently engaged device. 
	///
	/// #Returns
	///
	/// * Ok([CuckooMinerDeviceStats](struct.CuckooMinerDeviceStats.html)) if successful
	/// * A [CuckooMinerError](enum.CuckooMinerError.html) with specific detail if an
	/// error occurred

	pub fn get_stats(&self) -> Result<Vec<SolverStats>, CuckooMinerError> {
		let s = self.shared_data.read().unwrap();
		Ok(s.stats.clone())
	}

	/// #Description
	///
	/// Stops the current job, and signals for the loaded plugin to stop
	/// processing and perform any cleanup it needs to do.
	///
	/// #Returns
	///
	/// Nothing

	pub fn stop_jobs(&self) {
		debug!("Stop jobs called");
		{ 
			let mut r = self.control_data.write().unwrap();
			r.stop_flag = true;
		}
		debug!("Stop jobs flag set");
	}

}

/// An instance of a miner, which loads a cuckoo-miner plugin
/// and calls its mine function according to the provided configuration

pub struct CuckooMiner {
	/// Delegator object, used when spawning a processing thread
	delegator: Option<Delegator>,

	/// Loaded plugin
	solvers: Vec<SolverInstance>,
}

impl CuckooMiner {
	/// #Description
	///
	/// Creates a new instance of a CuckooMiner with the given configuration.
	///
	/// #Arguments
	///
	/// * `configs` an vector of
	/// [CuckooMinerConfigs](struct.CuckooMinerConfig.html), one for each plugin
	/// that is to be loaded and run, and each of which contains
	/// the full path name of a valid mining plugin. Each config struct may
	/// also contain values in its `parameter_list` field, which will be automatically set
	/// in the specified plugin. 
	///
	/// #Returns
	///
	/// * `Ok()` if successful, and the specified plugin has been loaded internally.
	/// * Otherwise a [CuckooMinerError](enum.CuckooMinerError.html)
	/// with specific detail

	pub fn new(configs: Vec<PluginConfig>) -> Result<CuckooMiner, CuckooMinerError> {
		CuckooMiner::init(configs)
	}

	/// Internal function to perform the actual library loading
	fn init(configs: Vec<PluginConfig>) -> Result<CuckooMiner, CuckooMinerError> {
		let mut lib_vec=Vec::new();
		for c in configs {
			lib_vec.push(SolverInstance::new(c)?);
		}

		let ret_val=CuckooMiner {
			delegator : None,
			solvers : lib_vec,
		};

		Ok(ret_val)
	}

	/// #Description
	///
	/// An asynchronous -esque version of the plugin miner, which takes
	/// parts of the header and the target difficulty as input, and begins
	/// asyncronous processing to find a solution. The loaded plugin is
	/// responsible
	/// for how it wishes to manage processing or distribute the load. Once
	/// called
	/// this function will continue to find solutions over the target difficulty
	/// for the given inputs and place them into its output queue until
	/// instructed to stop.
	///
	/// Once this function is called, the miner is consumed, and all
	/// interaction with the miner,
	/// including reading solutions or stopping the job, then takes place via
	/// the returned
	/// [CuckooMinerJobHandle](struct.CuckooMinerJobHandle.html) struct.
	///
	///
	/// #Arguments
	///
	/// * `job_id` (IN) A job ID, for later reference (not currently used).
	///
	/// * `pre_nonce` (IN) The part of the header which comes before the nonce,
	///   as a hex string slice.
	///
	/// * 'post_nonce` (IN) The part of the header which comes after the nonce
	///   as a hex string slice. This will be hashed together with generated
	///   nonces and the pre_nonce field to create hash inputs for the loaded
	///   cuckoo miner plugin.
	///
	/// * `difficulty` (IN) The miner will only put solutions greater than or
	///   equal to this difficulty in its output queue.
	///
	/// #Returns
	///
	/// * Ok([CuckooMinerJobHandle](struct.CuckooMinerJobHandle.html)) if the
	/// job
	/// is successfully started.
	/// * A [CuckooMinerError](enum.CuckooMinerError.html)
	/// if there is no plugin loaded, or if there is an error calling the
	/// function.

	pub fn notify(
		mut self,
		job_id: u32, // Job id
		pre_nonce: &str, // Pre-nonce portion of header
		post_nonce: &str, // Post-nonce portion of header
		difficulty: u64, /* The target difficulty, only sols greater than this difficulty will
		                  * be returned. */
	) -> Result<CuckooMinerJobHandle, CuckooMinerError> {

		//Note this gives up the plugin to the job thread
		self.delegator = Some(Delegator::new(job_id, pre_nonce, post_nonce, difficulty, self.solvers));
		Ok(self.delegator.unwrap().start_job_loop().unwrap())
	}
}
