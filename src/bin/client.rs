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

//! Client network controller, controls requests and responses from the
//! stratum server

use std::net::TcpStream;
use std;
use std::thread;
use std::io::{BufRead, ErrorKind, Write};
use std::sync::{mpsc, Arc, RwLock};

use serde_json;
use bufstream::BufStream;
use time;

use types;
use util::LOGGER;
use stats;

#[derive(Debug)]
pub enum Error {
	ConnectionError(String),
}

pub struct Controller {
	_id: u32,
	server_url: String,
	stream: Option<BufStream<TcpStream>>,
	rx: mpsc::Receiver<types::ClientMessage>,
	pub tx: mpsc::Sender<types::ClientMessage>,
	miner_tx: mpsc::Sender<types::MinerMessage>,
	last_request_id: u32,
	stats: Arc<RwLock<stats::Stats>>,
}

impl Controller {
	pub fn new(
		server_url: &str,
		miner_tx: mpsc::Sender<types::MinerMessage>,
		stats: Arc<RwLock<stats::Stats>>,
	) -> Result<Controller, Error> {
		let (tx, rx) = mpsc::channel::<types::ClientMessage>();
		Ok(Controller {
			_id: 0,
			server_url: server_url.to_string(),
			stream: None,
			tx: tx,
			rx: rx,
			miner_tx: miner_tx,
			last_request_id: 0,
			stats: stats,
		})
	}

	pub fn try_connect(&mut self) -> Result<(), Error> {
		match TcpStream::connect(self.server_url.clone()) {
			Ok(conn) => {
				let _ = conn.set_nonblocking(true);
				self.stream = Some(BufStream::new(conn));
				Ok(())
			}
			Err(e) => Err(Error::ConnectionError(format!("{}", e))),
		}
	}

	fn read_message(&mut self) -> Result<Option<String>, Error> {
		if let None = self.stream {
			return Err(Error::ConnectionError("broken pipe".to_string()));
		}
		let mut line = String::new();
		match self.stream.as_mut().unwrap().read_line(&mut line) {
			Ok(_) => {
				// stream is not returning a proper error on disconnect
				if line == "" {
					return Err(Error::ConnectionError("broken pipe".to_string()));
				}
				return Ok(Some(line));
			}
			Err(ref e) if e.kind() == ErrorKind::BrokenPipe => {
				return Err(Error::ConnectionError("broken pipe".to_string()));
			}
			Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
				return Ok(None);
			}
			Err(e) => {
				error!(LOGGER, "Communication error with stratum server: {}", e);
				return Err(Error::ConnectionError("broken pipe".to_string()));
			}
		}
	}

	fn send_message(&mut self, message: &str) -> Result<(), Error> {
		if let None = self.stream {
			return Err(Error::ConnectionError(String::from("No server connection")));
		}
		debug!(LOGGER, "sending request: {}", message);
		let _ = self.stream
			.as_mut()
			.unwrap()
			.write(message.as_bytes())
			.unwrap();
		let _ = self.stream
			.as_mut()
			.unwrap()
			.write("\n".as_bytes())
			.unwrap();
		let _ = self.stream.as_mut().unwrap().flush().unwrap();
		Ok(())
	}

	fn send_message_get_job_template(&mut self) -> Result<(), Error> {
		let req = types::RpcRequest {
			id: self.last_request_id.to_string(),
			jsonrpc: "2.0".to_string(),
			method: "getjobtemplate".to_string(),
			params: None,
		};
		let req_str = serde_json::to_string(&req).unwrap();
		{
			let mut stats = self.stats.write().unwrap();
			stats.client_stats.last_message_sent = format!("Last Message Sent: Get New Job");
		}
		self.send_message(&req_str)
	}

	fn send_message_get_status(&mut self) -> Result<(), Error> {
		let req = types::RpcRequest {
			id: self.last_request_id.to_string(),
			jsonrpc: "2.0".to_string(),
			method: "status".to_string(),
			params: None,
		};
		let req_str = serde_json::to_string(&req).unwrap();
		self.send_message(&req_str)
	}

	fn send_message_submit(&mut self, height: u64, nonce: u64, pow: Vec<u32>) -> Result<(), Error> {
		let params_in = types::SubmitParams {
			height: height,
			nonce: nonce,
			pow: pow,
		};
		let params = serde_json::to_string(&params_in).unwrap();
		let req = types::RpcRequest {
			id: self.last_request_id.to_string(),
			jsonrpc: "2.0".to_string(),
			method: "submit".to_string(),
			params: Some(params),
		};
		let req_str = serde_json::to_string(&req).unwrap();
		{
			let mut stats = self.stats.write().unwrap();
			stats.client_stats.last_message_sent = format!(
				"Last Message Sent: Found block for height: {} - nonce: {}",
				params_in.height, params_in.nonce
			);
		}
		self.send_message(&req_str)
	}

	fn send_miner_job(&mut self, job: types::JobTemplate) -> Result<(), Error> {
		let miner_message =
			types::MinerMessage::ReceivedJob(job.height, job.difficulty, job.pre_pow);
		let mut stats = self.stats.write().unwrap();
		stats.client_stats.last_message_received = format!(
			"Last Message Received: Start Job for Height: {}, Difficulty: {}",
			job.height, job.difficulty
		);
		let _ = self.miner_tx.send(miner_message);
		Ok(())
	}

	fn send_miner_stop(&mut self) -> Result<(), Error> {
		let miner_message = types::MinerMessage::StopJob;
		self.miner_tx.send(miner_message).unwrap();
		Ok(())
	}

	pub fn handle_request(&mut self, req: types::RpcRequest) -> Result<(), Error> {
		debug!(LOGGER, "Received request type: {}", req.method);
		let _ = match req.method.as_str() {
			"job" => {
				let job: types::JobTemplate = serde_json::from_str(&req.params.unwrap()).unwrap();
				info!(LOGGER, "Got a new job: {:?}", job);
				self.send_miner_job(job)
			}
			_ => Ok(()),
		};
		Ok(())
	}

	pub fn handle_response(&mut self, res: types::RpcResponse) -> Result<(), Error> {
		debug!(LOGGER, "Received response with id: {}", res.id);
		match res.method.as_str() {
			// "status" response can be used to further populate stats object
			"status" => {
				if res.result.is_some() {
					let st: types::WorkerStatus =
						serde_json::from_str(&res.result.unwrap()).unwrap();
					info!(
						LOGGER,
						"Status for worker {} - Height: {}, Difficulty: {}, ({}/{}/{})",
						st.id,
						st.height,
						st.difficulty,
						st.accepted,
						st.rejected,
						st.stale
					);
					// XXX TODO:  Add thses status to the stats
					let mut stats = self.stats.write().unwrap();
					stats.client_stats.last_message_received = format!("Last Message Received: Accepted: {}, Rejected: {}, Stale: {}", st.accepted, st.rejected, st.stale);
				} else {
					let err = res.error.unwrap();
					let mut stats = self.stats.write().unwrap();
					stats.client_stats.last_message_received =
						format!("Last Message Received: Failed to get status: {:?}", err);
					error!(LOGGER, "Failed to get status: {:?}", err);
				}
				()
			}
			// "getjobtemplate" response gets sent to miners to work on
			"getjobtemplate" => {
				if res.result.is_some() {
					let job: types::JobTemplate =
						serde_json::from_str(&res.result.unwrap()).unwrap();
					{
						let mut stats = self.stats.write().unwrap();
						stats.client_stats.last_message_received = format!(
							"Last Message Received: Got job for block {} at difficulty {}",
							job.height, job.difficulty
						);
					}
					info!(
						LOGGER,
						"Got a job at height {} and difficulty {}", job.height, job.difficulty
					);
					let _ = self.send_miner_job(job);
				} else {
					let err = res.error.unwrap();
					let mut stats = self.stats.write().unwrap();
					stats.client_stats.last_message_received = format!(
						"Last Message Received: Failed to get job template: {:?}",
						err
					);
					error!(LOGGER, "Failed to get a job template: {:?}", err);
				}
				()
			}
			// "submit" response
			"submit" => {
				if res.result.is_some() {
					info!(LOGGER, "Solution Accepted!!");
					let mut stats = self.stats.write().unwrap();
					stats.client_stats.last_message_received =
						format!("Last Message Received: Solution Accepted!!");
				} else {
					let err = res.error.unwrap();
					let mut stats = self.stats.write().unwrap();
					stats.client_stats.last_message_received = format!(
						"Last Message Received: Failed to submit a solution: {:?}",
						err
					);
					error!(LOGGER, "Failed to submit a solution: {:?}", err);
				}
				()
			}
			// "keepalive" response
			"keepalive" => {
				if res.result.is_some() {
					// Nothing to do for keepalive "ok"
					// dont update last_message_received with good keepalive response
				} else {
					let err = res.error.unwrap();
					let mut stats = self.stats.write().unwrap();
					stats.client_stats.last_message_received = format!(
						"Last Message Received: Failed to request keepalive: {:?}",
						err
					);
					error!(LOGGER, "Failed to request keepalive: {:?}", err);
				}
			}
			// unknown method response
			_ => {
				let mut stats = self.stats.write().unwrap();
				stats.client_stats.last_message_received =
					format!("Last Message Received: Unknown Response: {:?}", res);
				warn!(LOGGER, "Unknown Response: {:?}", res);
				()
			}
		}

		return Ok(());
	}

	pub fn run(mut self) {
		let server_read_interval = 1;
		let server_retry_interval = 5;
		let mut next_server_read = time::get_time().sec + server_read_interval;
		let status_interval = 30;
		let mut next_status_request = time::get_time().sec + status_interval;
		let mut next_server_retry = time::get_time().sec;
		// Request the first job template
		thread::sleep(std::time::Duration::from_secs(1));
		let mut was_disconnected = true;

		loop {
			// Check our connection status, and try to correct if possible
			if let None = self.stream {
				if !was_disconnected {
					let _ = self.send_miner_stop();
				}
				was_disconnected = true;
				if time::get_time().sec > next_server_retry {
					if let Err(_) = self.try_connect() {
						let status = format!("Connection Status: Can't establish server connection to {}. Will retry every {} seconds",
							self.server_url,
							server_retry_interval);
						warn!(LOGGER, "{}", status);
						let mut stats = self.stats.write().unwrap();
						stats.client_stats.connection_status = status;
						stats.client_stats.connected = false;
					} else {
						let status = format!(
							"Connection Status: Connected to Grin server at {}.",
							self.server_url
						);
						warn!(LOGGER, "{}", status);
						let mut stats = self.stats.write().unwrap();
						stats.client_stats.connection_status = status;
					}
					next_server_retry = time::get_time().sec + server_retry_interval;
				}
			} else {
				// get new job template
				if was_disconnected {
					let _ = self.send_message_get_job_template();
					was_disconnected = false;
				}
				// read messages from server
				if time::get_time().sec > next_server_read {
					match self.read_message() {
						Ok(message) => {
							match message {
								Some(m) => {
									{
										let mut stats = self.stats.write().unwrap();
										stats.client_stats.connected = true;
									}
									// figure out what kind of message,
									// and dispatch appropriately
									debug!(LOGGER, "Received message: {}", m);
									// Deserialize to see what type of object it is
									let v: serde_json::Value = serde_json::from_str(&m).unwrap();
									// Is this a response or request?
									if v["id"] == String::from("Stratum") {
										// this is a request
										let request: types::RpcRequest = serde_json::from_str(&m).unwrap();
										let _ = self.handle_request(request);
										continue;
									} else {
										// this is a response
										let response: types::RpcResponse = serde_json::from_str(&m).unwrap();
										let _ = self.handle_response(response);
										continue;
									}
								}
								None => {} // No messages from the server at this time
							}
						}
						Err(e) => {
							error!(LOGGER, "Error reading message: {:?}", e);
							self.stream = None;
						}
					}
					next_server_read = time::get_time().sec + server_read_interval;
				}

				// Request a status message from the server
				if time::get_time().sec > next_status_request {
					let _ = self.send_message_get_status();
					next_status_request = time::get_time().sec + status_interval;
				}
			}

			// Talk to the cuckoo miner plugin
			while let Some(message) = self.rx.try_iter().next() {
				debug!(LOGGER, "Client recieved message: {:?}", message);
				let result = match message {
					types::ClientMessage::FoundSolution(height, nonce, pow) => {
						self.send_message_submit(height, nonce, pow)
					}
					types::ClientMessage::Shutdown => {
						//TODO: Inform server?
						debug!(LOGGER, "Shutting down client controller");
						return;
					}
				};
				if let Err(e) = result {
					error!(LOGGER, "Mining Controller Error {:?}", e);
				}
			}
		} // loop
	}
}
