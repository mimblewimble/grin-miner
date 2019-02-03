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

use std;
use std::io::{self, BufRead, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::sync::{mpsc, Arc, RwLock};
use std::thread;

use bufstream::BufStream;
use native_tls::{TlsConnector, TlsStream};
use serde_json;
use time;

use stats;
use types;
use util::LOGGER;

#[derive(Debug)]
pub enum Error {
	ConnectionError(String),
}

struct Stream {
	stream: Option<BufStream<TcpStream>>,
	tls_stream: Option<BufStream<TlsStream<TcpStream>>>,
}

impl Stream {
	fn new() -> Stream {
		Stream {
			stream: None,
			tls_stream: None,
		}
	}
	fn try_connect(&mut self, server_url: &str, tls: Option<bool>) -> Result<(), Error> {
		match TcpStream::connect(server_url) {
			Ok(conn) => {
				if tls.is_some() && tls.unwrap() {
					let connector = TlsConnector::new().unwrap();
					let url_port: Vec<&str> = server_url.split(":").collect();
					let splitted_url: Vec<&str> = url_port[0].split(".").collect();
					let base_host = format!(
						"{}.{}",
						splitted_url[splitted_url.len() - 2],
						splitted_url[splitted_url.len() - 1]
					);
					let mut stream = connector.connect(&base_host, conn).unwrap();
					let _ = stream.get_mut().set_nonblocking(true);
					self.tls_stream = Some(BufStream::new(stream));
					Ok(())
				} else {
					let _ = conn.set_nonblocking(true);
					self.stream = Some(BufStream::new(conn));
					Ok(())
				}
			}
			Err(e) => Err(Error::ConnectionError(format!("{}", e))),
		}
	}
}

impl Write for Stream {
	fn write(&mut self, b: &[u8]) -> Result<usize, std::io::Error> {
		if self.tls_stream.is_some() {
			self.tls_stream.as_mut().unwrap().write(b)
		} else {
			self.stream.as_mut().unwrap().write(b)
		}
	}
	fn flush(&mut self) -> Result<(), std::io::Error> {
		if self.tls_stream.is_some() {
			self.tls_stream.as_mut().unwrap().flush()
		} else {
			self.stream.as_mut().unwrap().flush()
		}
	}
}
impl Read for Stream {
	fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
		if self.tls_stream.is_some() {
			self.tls_stream.as_mut().unwrap().read(buf)
		} else {
			self.stream.as_mut().unwrap().read(buf)
		}
	}
}

impl BufRead for Stream {
	fn fill_buf(&mut self) -> io::Result<&[u8]> {
		if self.tls_stream.is_some() {
			self.tls_stream.as_mut().unwrap().fill_buf()
		} else {
			self.stream.as_mut().unwrap().fill_buf()
		}
	}
	fn consume(&mut self, amt: usize) {
		if self.tls_stream.is_some() {
			self.tls_stream.as_mut().unwrap().consume(amt)
		} else {
			self.stream.as_mut().unwrap().consume(amt)
		}
	}
	fn read_until(&mut self, byte: u8, buf: &mut Vec<u8>) -> io::Result<usize> {
		if self.tls_stream.is_some() {
			self.tls_stream.as_mut().unwrap().read_until(byte, buf)
		} else {
			self.stream.as_mut().unwrap().read_until(byte, buf)
		}
	}
	fn read_line(&mut self, string: &mut String) -> io::Result<usize> {
		if self.tls_stream.is_some() {
			self.tls_stream.as_mut().unwrap().read_line(string)
		} else {
			self.stream.as_mut().unwrap().read_line(string)
		}
	}
}

pub struct Controller {
	_id: u32,
	server_url: String,
	server_login: Option<String>,
	server_password: Option<String>,
	server_tls_enabled: Option<bool>,
	stream: Option<Stream>,
	rx: mpsc::Receiver<types::ClientMessage>,
	pub tx: mpsc::Sender<types::ClientMessage>,
	miner_tx: mpsc::Sender<types::MinerMessage>,
	last_request_id: u32,
	stats: Arc<RwLock<stats::Stats>>,
}

fn invlalid_error_response() -> types::RpcError {
	types::RpcError {
		code: 0,
		message: "Invalid error response received".to_owned(),
	}
}

impl Controller {
	pub fn new(
		server_url: &str,
		server_login: Option<String>,
		server_password: Option<String>,
		server_tls_enabled: Option<bool>,
		miner_tx: mpsc::Sender<types::MinerMessage>,
		stats: Arc<RwLock<stats::Stats>>,
	) -> Result<Controller, Error> {
		let (tx, rx) = mpsc::channel::<types::ClientMessage>();
		Ok(Controller {
			_id: 0,
			server_url: server_url.to_string(),
			server_login: server_login,
			server_password: server_password,
			server_tls_enabled: server_tls_enabled,
			stream: None,
			tx: tx,
			rx: rx,
			miner_tx: miner_tx,
			last_request_id: 0,
			stats: stats,
		})
	}

	pub fn try_connect(&mut self) -> Result<(), Error> {
		self.stream = Some(Stream::new());
		self.stream
			.as_mut()
			.unwrap()
			.try_connect(&self.server_url, self.server_tls_enabled)?;
		Ok(())
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
		let _ = self.stream.as_mut().unwrap().write(message.as_bytes());
		let _ = self.stream.as_mut().unwrap().write("\n".as_bytes());
		let _ = self.stream.as_mut().unwrap().flush();
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

	fn send_login(&mut self) -> Result<(), Error> {
		// only send the login request if a login string is configured
		let login_str = match self.server_login.clone() {
			None => "".to_string(),
			Some(server_login) => server_login.clone(),
		};
		if login_str == "" {
			return Ok(());
		}
		let password_str = match self.server_password.clone() {
			None => "".to_string(),
			Some(server_password) => server_password.clone(),
		};
		let params = types::LoginParams {
			login: login_str,
			pass: password_str,
			agent: "grin-miner".to_string(),
		};
		let req = types::RpcRequest {
			id: self.last_request_id.to_string(),
			jsonrpc: "2.0".to_string(),
			method: "login".to_string(),
			params: Some(serde_json::to_value(params).unwrap()),
		};
		let req_str = serde_json::to_string(&req).unwrap();
		{
			let mut stats = self.stats.write().unwrap();
			stats.client_stats.last_message_sent = format!("Last Message Sent: Login");
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

	fn send_message_submit(
		&mut self,
		height: u64,
		job_id: u64,
		edge_bits: u32,
		nonce: u64,
		pow: Vec<u64>,
	) -> Result<(), Error> {
		let params_in = types::SubmitParams {
			height: height,
			job_id: job_id,
			edge_bits: edge_bits,
			nonce: nonce,
			pow: pow,
		};
		let params = serde_json::to_string(&params_in).unwrap();
		let req = types::RpcRequest {
			id: self.last_request_id.to_string(),
			jsonrpc: "2.0".to_string(),
			method: "submit".to_string(),
			params: Some(serde_json::from_str(&params).unwrap()),
		};
		let req_str = serde_json::to_string(&req).unwrap();
		{
			let mut stats = self.stats.write().unwrap();
			stats.client_stats.last_message_sent = format!(
				"Last Message Sent: Found share for height: {} - nonce: {}",
				params_in.height, params_in.nonce
			);
		}
		self.send_message(&req_str)
	}

	fn send_miner_job(&mut self, job: types::JobTemplate) -> Result<(), Error> {
		let miner_message =
			types::MinerMessage::ReceivedJob(job.height, job.job_id, job.difficulty, job.pre_pow, job.xn, job.cleanjob);
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
		let _ = self.miner_tx.send(miner_message);
		Ok(())
	}

	pub fn handle_request(&mut self, req: types::RpcRequest) -> Result<(), Error> {
		debug!(LOGGER, "Received request type: {}", req.method);
		let _ = match req.method.as_str() {
			"job" => {
				let job: types::JobTemplate = serde_json::from_value(req.params.unwrap()).unwrap();
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
						serde_json::from_value(res.result.unwrap()).unwrap();
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
					// Add these status to the stats
					let mut stats = self.stats.write().unwrap();
					stats.client_stats.last_message_received = format!(
						"Last Message Received: Accepted: {}, Rejected: {}, Stale: {}",
						st.accepted, st.rejected, st.stale
					);
				} else {
					let err = res.error.unwrap_or_else(|| invlalid_error_response());
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
						serde_json::from_value(res.result.unwrap()).unwrap();
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
					let err = res.error.unwrap_or_else(|| invlalid_error_response());
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
					info!(LOGGER, "Share Accepted!!");
					let mut stats = self.stats.write().unwrap();
					stats.client_stats.last_message_received =
						format!("Last Message Received: Share Accepted!!");
					stats.mining_stats.solution_stats.num_shares_accepted += 1;
					let result = serde_json::to_string(&res.result).unwrap();
					if result.contains("blockfound") {
						info!(LOGGER, "Block Found!!");
						stats.client_stats.last_message_received =
							format!("Last Message Received: Block Found!!");
						stats.mining_stats.solution_stats.num_blocks_found += 1;
					}
				} else {
					let err = res.error.unwrap_or_else(|| invlalid_error_response());
					let mut stats = self.stats.write().unwrap();
					stats.client_stats.last_message_received = format!(
						"Last Message Received: Failed to submit a solution: {:?}",
						err.message
					);
					if err.message.contains("too late") {
						stats.mining_stats.solution_stats.num_staled += 1;
					} else {
						stats.mining_stats.solution_stats.num_rejected += 1;
					}
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
					let err = res.error.unwrap_or_else(|| invlalid_error_response());
					let mut stats = self.stats.write().unwrap();
					stats.client_stats.last_message_received = format!(
						"Last Message Received: Failed to request keepalive: {:?}",
						err
					);
					error!(LOGGER, "Failed to request keepalive: {:?}", err);
				}
			}
			// "login" response
			"login" => {
				if res.result.is_some() {
					// Nothing to do for login "ok"
					// dont update last_message_received with good login response
				} else {
					// This is a fatal error
					let err = res.error.unwrap_or_else(|| invlalid_error_response());
					let mut stats = self.stats.write().unwrap();
					stats.client_stats.last_message_received =
						format!("Last Message Received: Failed to log in: {:?}", err);
					stats.client_stats.connection_status =
						"Connection Status: Server requires login".to_string();
					stats.client_stats.connected = false;
					error!(LOGGER, "Failed to log in: {:?}", err);
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
						self.stream = None;
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
					if let None = self.stream {
						thread::sleep(std::time::Duration::from_secs(1));
						continue;
					}
				}
			} else {
				// get new job template
				if was_disconnected {
					let _ = self.send_login();
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
									// According to https://github.com/mimblewimble/grin/blob/master/doc/stratum.md
									// job is the only request and id can be anything
									if v["method"] == String::from("job") {
										// this is a request
										let request: types::RpcRequest =
											serde_json::from_str(&m).unwrap();
										let _ = self.handle_request(request);
										continue;
									} else {
										// this is a response
										let response: types::RpcResponse =
											serde_json::from_str(&m).unwrap();
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
							continue;
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
				debug!(LOGGER, "Client received message: {:?}", message);
				let result = match message {
					types::ClientMessage::FoundSolution(height, job_id, edge_bits, nonce, pow) => {
						self.send_message_submit(height, job_id, edge_bits, nonce, pow)
					}
					types::ClientMessage::Shutdown => {
						//TODO: Inform server?
						debug!(LOGGER, "Shutting down client controller");
						return;
					}
				};
				if let Err(e) = result {
					error!(LOGGER, "Mining Controller Error {:?}", e);
					self.stream = None;
				}
			}
			thread::sleep(std::time::Duration::from_millis(10));
		} // loop
	}
}
