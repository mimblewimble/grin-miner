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
/// 

use std::sync::{mpsc, Arc, RwLock};
use time;
use std::{self, thread};
use util::LOGGER;
use config;
use stats;

use cuckoo::{
	CuckooMinerSolution,
	CuckooMinerJobHandle,
	CuckooMinerError};

use {plugin, types};

pub struct Controller {
	config: config::MinerConfig,
	plugin_miner: Option<plugin::PluginMiner>,
	job_handle: Option<CuckooMinerJobHandle>,
	rx: mpsc::Receiver<types::MinerMessage>,
	pub tx: mpsc::Sender<types::MinerMessage>,
	client_tx: Option<mpsc::Sender<types::ClientMessage>>,
	current_height: u64,
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
			config: config,
			plugin_miner: None,
			job_handle: None,
			rx: rx,
			tx: tx,
			client_tx: None,
			current_height: 0,
			current_target_diff: 0,
			stats: stats,
		})
	}

	pub fn set_client_tx(&mut self, client_tx: mpsc::Sender<types::ClientMessage>) {
		self.client_tx = Some(client_tx);
	}

	/// Run the mining controller
	pub fn run(&mut self){
		// how often to output stats
		let stat_output_interval = 2;
		let mut next_stat_output = time::get_time().sec + stat_output_interval;

		loop {
			while let Some(message) = self.rx.try_iter().next() {
				debug!(LOGGER, "Miner received message: {:?}", message);
				let result = match message {
					types::MinerMessage::ReceivedJob(height, diff, pre_pow) => {
						self.stop_job();
						self.current_height = height;
						self.current_target_diff = diff;
						self.start_job(30, &pre_pow)
					},
					types::MinerMessage::StopJob => {
						debug!(LOGGER, "Stopping jobs");
						self.stop_job();
						Ok(())
					}types::MinerMessage::Shutdown => {
						debug!(LOGGER, "Stopping jobs and Shutting down mining controller");
						self.stop_job();
						return;
					}
				};
				if let Err(e) = result {
					error!(LOGGER, "Mining Controller Error {:?}", e);
				}
			}

			if time::get_time().sec > next_stat_output {
				self.output_job_stats();
				next_stat_output = time::get_time().sec + stat_output_interval;
			}

			let sol = self.check_solutions();
			if sol.is_some(){
				let sol = sol.unwrap();
				let _ = self.client_tx.as_mut().unwrap().send(types::ClientMessage::FoundSolution (
					self.current_height,
					sol.get_nonce_as_u64(),
					sol.solution_nonces.to_vec(),
				));
			}
			thread::sleep(std::time::Duration::from_millis(100));
		}
	}

	/// Inner part of the mining loop for cuckoo-miner async mode
	fn start_job(
		&mut self,
		cuckoo_size: usize,
		pre_pow: &str,
	) -> Result<(), CuckooMinerError> {
		debug!(
			LOGGER,
			"Mining Cuckoo{} for height: {}",
			cuckoo_size,
			self.current_height,
		);

		// Init the miner
		let mut plugin_miner = plugin::PluginMiner::new();
		plugin_miner.init(self.config.clone());
		self.plugin_miner = Some(plugin_miner);

		let mut hash_header = true;
		if self.config.hash_header == Some(false) {
			hash_header = false;
		}

		// Start the miner working
		let miner = self.plugin_miner.as_mut().unwrap().get_consumable();
		self.job_handle = Some(miner.notify(1, &pre_pow, "", self.current_target_diff, hash_header)?);
		Ok(())
	}

	fn check_solutions(&mut self) -> Option<CuckooMinerSolution> {
		if self.job_handle.is_none() {
			return None;
		}
		let job_handle = self.job_handle.as_mut().unwrap();
		if let Some(s) = job_handle.get_solution() {
			debug!(
				LOGGER,
				"Found cuckoo solution! nonce {}",
				s.get_nonce_as_u64(),
			);
			return Some(s);
		}
		None
	}

	fn output_job_stats(&mut self) {
		if self.job_handle.is_none() {
			return;
		}
		let mut sps_total = 0.0;
		let plugin_miner = self.plugin_miner.as_mut().unwrap();
		let job_handle = self.job_handle.as_mut().unwrap();
		for i in 0..plugin_miner.loaded_plugin_count() {
			let stats = job_handle.get_stats(i);
			if let Ok(stat_vec) = stats {
				for s in stat_vec {
					if s.in_use == 0 {
						continue;
					}
					let last_solution_time_secs =
						s.last_solution_time as f64 / 1000000000.0;
					let last_hashes_per_sec = 1.0 / last_solution_time_secs;
					let status = match s.has_errored {
						0 => "OK",
						_ => "ERRORED",
					};
					debug!(
						LOGGER,
								"Mining: Plugin {} - Device {} ({}) Status: {} : Last Graph time: {}s; \
						 Graphs per second: {:.*} - Total Attempts: {}",
								i,
						s.device_id,
						s.device_name,
						status,
						last_solution_time_secs,
						3,
						last_hashes_per_sec,
						s.iterations_completed
					);
					if last_hashes_per_sec.is_finite() {
						sps_total += last_hashes_per_sec;
					}
				}
			}
		}
		info!(
			LOGGER,
			"Mining: Cuckoo{} at {} gps (graphs per second)", 30, sps_total
		);
		if sps_total.is_finite() {
			let mut stats = self.stats.write().unwrap();
			stats.mining_stats.combined_gps = sps_total;
			stats.mining_stats.target_difficulty = self.current_target_diff;
			stats.mining_stats.block_height = self.current_height;
			stats.mining_stats.cuckoo_size = 30;
			let mut device_vec = vec![];
			for i in 0..plugin_miner.loaded_plugin_count() {
				device_vec.push(job_handle.get_stats(i).unwrap());
			}
			stats.mining_stats.device_stats = Some(device_vec);
		}
	}

	fn stop_job(&mut self){
		if self.job_handle.is_none() {
			return;
		}
		self.job_handle.as_mut().unwrap().stop_jobs();
	}
}
