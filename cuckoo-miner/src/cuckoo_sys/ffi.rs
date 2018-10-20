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

//! Low-Level manager for loading and unloading plugins. These functions
//! should generally not be called directly by most consumers, who should
//! be using the high level interfaces found in the config, manager, and
//! miner modules. These functions are meant for internal cuckoo-miner crates,
//! and will not be exposed to other projects including the cuckoo-miner crate.

use std::sync::Mutex;

use libc::*;
use libloading;

use error::error::CuckooMinerError;

pub const PROOFSIZE: usize = 42;

/// A solver context, opaque reference to C++ type underneath
#[derive(Clone, Debug)]
#[repr(C)]
pub struct SolverCtx(c_int);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[repr(C)]
pub struct SolverParams {
	pub num_threads: uint32_t,
	pub num_trims: uint32_t,
	pub cuda_device_id: uint32_t,
	// etc
}

#[derive(Clone, Debug)]
#[repr(C)]
pub struct SolverStats {
	pub device_id: uint32_t,
	pub edge_bits: uint32_t,
	pub device_name: *const c_uchar,
	pub device_name_len: *const uint32_t,
	pub last_start_time: uint64_t,
	pub last_end_time: uint64_t,
	pub last_solution_time: uint64_t,
}

#[repr(C)]
pub struct SolverSolutions {
	//TODO: Multiple solution struct layout
	pub nonce: uint64_t,
	pub proof: [uint64_t; PROOFSIZE],
}

// Type definitions corresponding to each function that the plugin/solver implements
type CuckooCreateSolverCtx = unsafe extern "C" fn() -> SolverCtx;
type CuckooDestroySolverCtx = unsafe extern "C" fn(*mut SolverCtx);
type CuckooRunSolver = unsafe extern "C" fn(
	*mut SolverCtx,       // Solver context
	*const SolverParams,  // Solver parameters
	*const c_uchar,       // header
	uint32_t,             // header length
	*const uint64_t,      // nonce
	*mut SolverSolutions, // reference to any found solutions
	*mut SolverStats,     // solver stats
) -> uint32_t;
type CuckooStopSolver = unsafe extern "C" fn();

/// Struct to hold instances of loaded plugins

pub struct PluginLibrary {
	///The full file path to the plugin loaded by this instance
	pub lib_full_path: String,

	loaded_library: Mutex<libloading::Library>,
	cuckoo_create_solver_ctx: Mutex<CuckooCreateSolverCtx>,
	cuckoo_destroy_solver_ctx: Mutex<CuckooDestroySolverCtx>,
	cuckoo_run_solver: Mutex<CuckooRunSolver>,
	cuckoo_stop_solver: Mutex<CuckooStopSolver>,
}

impl PluginLibrary {
	/// Loads the specified library

	pub fn new(lib_full_path: &str) -> Result<PluginLibrary, CuckooMinerError> {
		debug!("Loading miner plugin: {}", &lib_full_path);

		let result = libloading::Library::new(lib_full_path);

		if let Err(e) = result {
			return Err(CuckooMinerError::PluginNotFoundError(String::from(
				format!("{} - {:?}", lib_full_path, e),
			)));
		}

		let loaded_library = result.unwrap();
		PluginLibrary::load_symbols(loaded_library, lib_full_path)
	}

	fn load_symbols(
		loaded_library: libloading::Library,
		path: &str,
	) -> Result<PluginLibrary, CuckooMinerError> {
		unsafe {
			let ret_val = PluginLibrary {
				lib_full_path: String::from(path),

				cuckoo_create_solver_ctx: {
					let cuckoo_create_solver_ctx: libloading::Symbol<
						CuckooCreateSolverCtx,
					> = loaded_library.get(b"create_solver_ctx\0").unwrap();
					Mutex::new(*cuckoo_create_solver_ctx.into_raw())
				},

				cuckoo_destroy_solver_ctx: {
					let cuckoo_destroy_solver_ctx: libloading::Symbol<
						CuckooDestroySolverCtx,
					> = loaded_library.get(b"destroy_solver_ctx\0").unwrap();
					Mutex::new(*cuckoo_destroy_solver_ctx.into_raw())
				},

				cuckoo_run_solver: {
					let cuckoo_run_solver: libloading::Symbol<CuckooRunSolver> =
						loaded_library.get(b"run_solver\0").unwrap();
					Mutex::new(*cuckoo_run_solver.into_raw())
				},

				cuckoo_stop_solver: {
					let cuckoo_stop_solver: libloading::Symbol<
						CuckooStopSolver,
					> = loaded_library.get(b"stop_solver\0").unwrap();
					Mutex::new(*cuckoo_stop_solver.into_raw())
				},

				loaded_library: Mutex::new(loaded_library),
			};

			return Ok(ret_val);
		}
	}

	/// #Description
	///
	/// Unloads the currently loaded plugin and all symbols.
	///
	/// #Arguments
	///
	/// None
	///
	/// #Returns
	///
	/// Nothing
	///

	pub fn unload(&self) {
		let cuckoo_create_solver_ref = self.cuckoo_create_solver_ctx.lock().unwrap();
		drop(cuckoo_create_solver_ref);

		let cuckoo_destroy_solver_ref = self.cuckoo_destroy_solver_ctx.lock().unwrap();
		drop(cuckoo_destroy_solver_ref);

		let cuckoo_run_solver_ref = self.cuckoo_run_solver.lock().unwrap();
		drop(cuckoo_run_solver_ref);

		let cuckoo_stop_solver_ref = self.cuckoo_stop_solver.lock().unwrap();
		drop(cuckoo_stop_solver_ref);

		let loaded_library_ref = self.loaded_library.lock().unwrap();
		drop(loaded_library_ref);
	}

	/// Create a solver context
	pub fn call_cuckoo_create_solver_ctx(&self) -> SolverCtx {
		let call_ref = self.cuckoo_create_solver_ctx.lock().unwrap();
		unsafe { call_ref() }
	}

	/// Destroy solver context
	pub fn call_cuckoo_destroy_solver_ctx(&self, mut ctx: SolverCtx) {
		let call_ref = self.cuckoo_destroy_solver_ctx.lock().unwrap();
		unsafe { call_ref(&mut ctx) }
	}

	/// Run Solver
	pub fn call_cuckoo_run_solver(
		&self,
		mut ctx: SolverCtx,
		params: SolverParams,
		header: Vec<u8>,
		nonce: u64,
		solutions: &mut SolverSolutions,
		stats: &mut SolverStats,
	) -> u32 {
		let call_ref = self.cuckoo_run_solver.lock().unwrap();
		unsafe {
			call_ref(
				&mut ctx,
				&params,
				header.as_ptr(),
				header.len() as u32,
				&nonce,
				solutions,
				stats,
			)
		}
	}

	/// Stop solver
	pub fn call_cuckoo_stop_solver(&self) {
		let call_ref = self.cuckoo_stop_solver.lock().unwrap();
		unsafe { call_ref() }
	}
}
