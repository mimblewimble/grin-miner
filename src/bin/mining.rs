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

/// Plugin controller, listens for messages sent from the stratum 
/// server, controls plugins and responds appropriately

use std::sync::{mpsc, Arc, RwLock};
use time;
use std::{self, thread};
use util::LOGGER;
use {config, types, stats};

use cuckoo::{
	CuckooMiner,
	CuckooMinerError,
	SolverStats
};

pub struct Controller {
	_config: config::MinerConfig,
	rx: mpsc::Receiver<types::MinerMessage>,
	pub tx: mpsc::Sender<types::MinerMessage>,
	client_tx: Option<mpsc::Sender<types::ClientMessage>>,
	current_height: u64,
	current_job_id: u64,
	current_target_diff: u64,
	stats: Arc<RwLock<stats::Stats>>,
}

impl Controller {
	pub fn new(config: config::MinerConfig, stats: Arc<RwLock<stats::Stats>>) -> Result<Controller, String> {
		{
			let mut stats_w = stats.write().unwrap();
			stats_w.client_stats.server_url = config.stratum_server_addr.clone();
		}
		let (tx, rx) = mpsc::channel::<types::MinerMessage>();
		Ok(Controller {
			_config: config,
			rx: rx,
			tx: tx,
			client_tx: None,
			current_height: 0,
			current_job_id: 0,
			current_target_diff: 0,
			stats: stats,
		})
	}

	pub fn set_client_tx(&mut self, client_tx: mpsc::Sender<types::ClientMessage>) {
		self.client_tx = Some(client_tx);
	}

	/// Run the mining controller, solvers in miner should already be going
	pub fn run(&mut self, mut miner: CuckooMiner) -> Result<(), CuckooMinerError>{
		// how often to output stats
		let stat_output_interval = 2;
		let mut next_stat_output = time::get_time().sec + stat_output_interval;

		loop {
			while let Some(message) = self.rx.try_iter().next() {
				debug!(LOGGER, "Miner received message: {:?}", message);
				let result = match message {
					types::MinerMessage::ReceivedJob(height, job_id, diff, pre_pow) => {
						self.current_height = height;
						self.current_job_id = job_id;
						self.current_target_diff = diff;
						miner.notify(self.current_job_id as u32, &pre_pow, "", 0)
					},
					types::MinerMessage::StopJob => {
						debug!(LOGGER, "Stopping jobs");
						miner.pause_solvers();
						Ok(())
					}types::MinerMessage::Shutdown => {
						debug!(LOGGER, "Stopping jobs and Shutting down mining controller");
						miner.stop_solvers();
						miner.wait_for_solver_shutdown();
						return Ok(());
					}
				};
				if let Err(e) = result {
					error!(LOGGER, "Mining Controller Error {:?}", e);
				}
			}

			if time::get_time().sec > next_stat_output {
				self.output_job_stats(miner.get_stats().unwrap());
				next_stat_output = time::get_time().sec + stat_output_interval;
			}

			let solutions = miner.get_solutions();
			if let Some(ss) = solutions {
				let edge_bits = ss.edge_bits;
				for i in 0..ss.num_sols {
					let _ = self.client_tx.as_mut().unwrap().send(types::ClientMessage::FoundSolution (
						self.current_height,
						self.current_job_id,
						edge_bits,
						ss.sols[i as usize].nonce,
						ss.sols[i as usize].proof.to_vec(),
					));
				}
			}
			thread::sleep(std::time::Duration::from_millis(100));
		}
	}

	fn output_job_stats(&mut self, stats: Vec<SolverStats>) {
		let mut sps_total = 0.0;
		let mut i = 0;
		for s in stats.clone() {
			let last_solution_time_secs =
				s.last_solution_time as f64 / 1000000000.0;
			let last_hashes_per_sec = 1.0 / last_solution_time_secs;
			let status = match s.has_errored {
				false => "OK",
				_ => "ERRORED",
			};
			if !s.has_errored {
				debug!(
					LOGGER,
							"Mining: Plugin {} - Device {} ({}) at Cuck(at)oo{} - Status: {} : Last Graph time: {}s; \
					 Graphs per second: {:.*} - Total Attempts: {}",
							i,
					s.device_id,
					s.get_device_name(),
					s.edge_bits,
					status,
					last_solution_time_secs,
					3,
					last_hashes_per_sec,
					s.iterations
				);
				if last_hashes_per_sec.is_finite() {
					sps_total += last_hashes_per_sec;
				}
			} else {
				debug!(
					LOGGER,
							"Mining: Plugin {} - Device {} ({}) Has ERRORED! Reason: {}",
							i,
							s.device_id,
							s.get_device_name(),
							s.get_error_reason(),
					);
			}
			i += 1;
		}
		info!(
			LOGGER,
			"Mining: Cuck(at)oo at {} gps (graphs per second)", sps_total
		);

		if sps_total.is_finite() {
			let mut s_stats = self.stats.write().unwrap();
			s_stats.mining_stats.combined_gps = sps_total;
			s_stats.mining_stats.target_difficulty = self.current_target_diff;
			s_stats.mining_stats.block_height = self.current_height;
			s_stats.mining_stats.device_stats = stats;
		}
	}

}
