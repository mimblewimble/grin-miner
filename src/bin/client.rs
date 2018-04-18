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
use std::{thread};
use std::io::{ErrorKind, BufRead, Write};
use std::sync::mpsc;

use serde_json;
use bufstream::BufStream;
use time;

use types;
use util::LOGGER;


#[derive(Debug)]
pub enum Error {
	ConnectionError(String),
}

pub struct Controller {
	_id: u32,
	stream: BufStream<TcpStream>,
	rx: mpsc::Receiver<types::ClientMessage>,
	pub tx: mpsc::Sender<types::ClientMessage>,
	miner_tx: mpsc::Sender<types::MinerMessage>,
	last_request_id: u32,
}

impl Controller {

	pub fn new(server_url: &str, miner_tx: mpsc::Sender<types::MinerMessage>) -> Result<Controller, Error> {
		let (tx, rx) = mpsc::channel::<types::ClientMessage>();
		match TcpStream::connect(server_url){
			Ok(conn) => {
				let _ = conn.set_nonblocking(true);
				Ok(Controller {
						_id: 0,
						stream: BufStream::new(conn),
						tx: tx,
						rx: rx,
						miner_tx: miner_tx,
						last_request_id: 0,
					})
			}
			Err(e) => return Err(Error::ConnectionError(e.to_string())),
		}
	}

	fn read_message(&mut self) -> Option<String> {
		let mut line = String::new();
		match self.stream.read_line(&mut line){
			Ok(_) => {
				return Some(line);
			}
			Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
				return None;
			}
			Err(e) => {
				error!(
					LOGGER,
					"Communication error with stratum server: {}", e
				);
				return None;
			}
		}
	}

	fn send_message(&mut self, message: &str) -> Result<(), Error> {
		debug!(LOGGER, "sending request: {}", message);
		let _ = self.stream.write(message.as_bytes()).unwrap();
		let _ = self.stream.write("\n".as_bytes()).unwrap();
		let _ = self.stream.flush().unwrap();
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
		self.send_message(&req_str)
	}

	fn send_message_submit(&mut self, height: u64, nonce: u64, pow: Vec<u32>) -> Result<(), Error> {
		let params = types::SubmitParams{
			height: height,
			nonce: nonce,
			pow: pow,
		};
		let params = serde_json::to_string(&params).unwrap();
		let req = types::RpcRequest {
				id: self.last_request_id.to_string(),
				jsonrpc: "2.0".to_string(),
				method: "submit".to_string(),
				params: Some(params),
			};
		let req_str = serde_json::to_string(&req).unwrap();
		self.send_message(&req_str)
	}

	fn send_miner_job(&mut self, params:String) -> Result<(), Error>{
		let params:types::JobTemplate = serde_json::from_str(&params).unwrap();
		let miner_message = types::MinerMessage::ReceivedJob (
			params.height,
			params.difficulty,
			params.pre_pow,
		);
		self.miner_tx.send(miner_message).unwrap();
		Ok(())
	}

	pub fn handle_request(&mut self, req: types::RpcRequest) -> Result<(), Error> {
		debug!(LOGGER, "Received request type: {}", req.method);
		let _ = match req.method.as_str() {
			"job" => {
				self.send_miner_job(req.params.unwrap())
			}
			_ => {Ok(())}
		};
		Ok(())
	}

	pub fn handle_response(&mut self, res: types::RpcResponse) -> Result<(), Error> {
		debug!(LOGGER, "Received response with id: {}", res.id);
		
		//TODO: this response needs to be matched up with the request somehow.. for the moment
		//assume it's just a response to a get_job_template request
		if res.result.is_some() {
			if res.result.as_ref().unwrap() == "ok" {
				debug!(LOGGER, "Received OK response from server");
				return Ok(());
			}
			self.send_miner_job(res.result.unwrap())
		} else {
			Ok(())
		}
	}

	pub fn run(mut self) {
		let server_read_interval = 1;
		let mut next_server_read = time::get_time().sec + server_read_interval;
		// Request the first job template
		thread::sleep(std::time::Duration::from_secs(1));
		let _ = self.send_message_get_job_template();
		
		loop {
			// read messages from server
			if time::get_time().sec > next_server_read {
				match self.read_message() {
					Some(m) => {
						// figure out what kind of message,
						// and dispatch appropriately
						debug!(LOGGER, "Received message: {}", m);
						// Is this a request?
						let request:Result<types::RpcRequest, serde_json::Error> = serde_json::from_str(&m);
						if let Ok(r) = request {
							let _ = self.handle_request(r);
							continue;
						}
						// Is this a response?
						let response:Result<types::RpcResponse, serde_json::Error> = serde_json::from_str(&m);
						if let Ok(r) = response {
							let _ = self.handle_response(r);
							continue;
						}
						
						error!(
							LOGGER,
							"Failed to parse JSON RPC: {}",
							m,
						);
						
					}
					None => {}
				}
				next_server_read = time::get_time().sec + server_read_interval;
			}

			while let Some(message) = self.rx.try_iter().next() {
				debug!(LOGGER, "Client recieved message from miner: {:?}", message);
				let result = match message {
					types::ClientMessage::FoundSolution(height, nonce, pow) => {
						self.send_message_submit(height, nonce, pow)
					},
					types::ClientMessage::Shutdown => {
						//TODO: Inform server?
						return;
					},
				};
				if let Err(e) = result {
					error!(LOGGER, "Mining Controller Error {:?}", e);
				}
			}


		}

			
	}
}


