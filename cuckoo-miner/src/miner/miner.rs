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
use util::LOGGER;

use config::types::PluginConfig;
use miner::types::{
	JobControlData, JobControlDataType, JobSharedData, JobSharedDataType,
	SolverInstance,
};
use miner::util;
use {CuckooMinerError, PluginLibrary, SolverStats, SolverSolutions};

/// An instance of a miner, which loads a cuckoo-miner plugin
/// and calls its mine function according to the provided configuration

pub struct CuckooMiner {
	/// Configurations
	configs: Vec<PluginConfig>,

	/// Data shared across threads
	pub shared_data: Arc<RwLock<JobSharedData>>,

	/// Job control flag
	pub control_data: Arc<RwLock<JobControlData>>,
}

impl CuckooMiner {
	/// Creates a new instance of a CuckooMiner with the given configuration.
	/// One PluginConfig per device

	pub fn new(configs: Vec<PluginConfig>) -> CuckooMiner {
		let len = configs.len();
		CuckooMiner {
			configs: configs,
			shared_data: Arc::new(RwLock::new(JobSharedData::new(len))),
			control_data: Arc::new(RwLock::new(JobControlData::default())),
		}
	}

	/// Solver's instance of a thread
	fn solver_thread(
		mut solver: SolverInstance,
		instance: usize,
		shared_data: JobSharedDataType,
		control_data: JobControlDataType,
	) {
		// "Detach" a stop function from the solver, to let us keep a control thread going
		let stop_fn = solver.lib.get_stop_solver_instance();
		let ctrl_data = control_data.clone();
		let sleep_dur = time::Duration::from_millis(100);
		// monitor whether to send a stop signal to the solver, which should
		// end the current solve attempt below
		let stop_handle = thread::spawn(move || {
			loop {
				{
					let mut s = ctrl_data.write().unwrap();
					if s.stop_flag || s.pause_signal {
						PluginLibrary::stop_solver_from_instance(stop_fn.clone());
						s.pause_signal = false;
						break;
					}
				}
				//avoid busy wait
				thread::sleep(sleep_dur);
			}
		});

		let mut iter_count = 0;
		let ctx = solver.lib.create_solver_ctx(&mut solver.config.params);
		loop {
			{
				let c = control_data.read().unwrap();
				if c.paused {
					thread::sleep(sleep_dur);
					continue;
				}
				if c.stop_flag {
					break;
				}
			}
			{
				let mut s = shared_data.write().unwrap();
				s.stats[instance].set_plugin_name(&solver.config.name);
			}
			let header_pre = shared_data.read().unwrap().pre_nonce.clone();
			let header_post = shared_data.read().unwrap().pre_nonce.clone();
			let header = util::get_next_header_data(&header_pre, &header_post);
			let nonce = header.0;
			solver.lib.run_solver(
				ctx,
				header.1,
				0,
				1,
				&mut solver.solutions,
				&mut solver.stats,
			);
			iter_count += 1;
			{
				let mut s = shared_data.write().unwrap();
				s.stats[instance] = solver.stats.clone();
				s.stats[instance].iterations = iter_count;
				if solver.solutions.num_sols > 0 {
					for mut ss in solver.solutions.sols.iter_mut() {
						ss.nonce = nonce;
					}
					s.solutions.push(solver.solutions.clone());
				}
			}
			solver.solutions = SolverSolutions::default();
		}

		let _ = stop_handle.join();
		solver.lib.destroy_solver_ctx(ctx);
		solver.lib.unload();
	}

	/// Starts solvers, ready for jobs via job control
	pub fn start_solvers(
		&mut self,
	) -> Result<(), CuckooMinerError> {
		let mut solvers = Vec::new();
		for c in self.configs.clone() {
			solvers.push(SolverInstance::new(c)?);
		}
		let mut i = 0;
		for s in solvers {
			let sd = self.shared_data.clone();
			let cd = self.control_data.clone();
			thread::spawn(move || {
				let _ = CuckooMiner::solver_thread(s, i, sd, cd);
			});
			i += 1;
		}
		Ok(())
	}

	/// An asynchronous -esque version of the plugin miner, which takes
	/// parts of the header and the target difficulty as input, and begins
	/// asyncronous processing to find a solution. The loaded plugin is
	/// responsible
	/// for how it wishes to manage processing or distribute the load. Once
	/// called
	/// this function will continue to find solutions over the target difficulty
	/// for the given inputs and place them into its output queue until
	/// instructed to stop.

	pub fn notify(
		&mut self,
		job_id: u32,      // Job id
		pre_nonce: &str,  // Pre-nonce portion of header
		post_nonce: &str, // Post-nonce portion of header
		difficulty: u64,  /* The target difficulty, only sols greater than this difficulty will
		                   * be returned. */
	) -> Result<(), CuckooMinerError> {
		// stop/pause any existing jobs
		self.set_paused(true);
		// Notify of new header data
		let mut sd = self.shared_data.write().unwrap();
		sd.job_id = job_id;
		sd.pre_nonce = pre_nonce.to_owned();
		sd.post_nonce = post_nonce.to_owned();
		sd.difficulty = difficulty;
		// resume jobs
		self.set_paused(false);
		Ok(())
	}

	/// Returns solutions if currently waiting.

	pub fn get_solutions(&self) -> Option<SolverSolutions> {
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

	/// get stats for all running solvers
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

	pub fn stop_solvers(&self) {
		{
			let mut r = self.control_data.write().unwrap();
			r.stop_flag = true;
		}
		debug!(LOGGER, "Stop jobs flag set");
	}

	/// Tells current solvers to stop and wait
	pub fn set_paused(&self, value: bool) {
		{
			let mut r = self.control_data.write().unwrap();
			r.paused = value;
			if r.paused {
				r.pause_signal = true;
			}
		}
		debug!(LOGGER, "Pause jobs flag set to {}", value);
	}
}
