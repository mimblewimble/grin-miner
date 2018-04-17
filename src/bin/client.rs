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
use std::{thread, time};
use std::io::{ErrorKind, BufRead, Write};
use std::sync::mpsc;

use serde_json;
use bufstream::BufStream;

use types;
use util::LOGGER;

#[derive(Debug)]
pub enum Error {
	ConnectionError(String),
}

pub struct Controller {
	id: u32,
	server_url: String,
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
						id: 0,
						server_url: server_url.to_string(),
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

	pub fn handle_request(&mut self, req: types::RpcRequest) -> Result<(), Error> {
	/*let (response, err) = match response.method {
						"result" => {
							debug!(LOGGER, "result matched: {:?}", response);
							(response, true)
						}
						_ => {
							let e = r#"{"code": -32601, "message": "Method not found"}"#;
							let err = e.to_string();
							(err, true)
						}
					};*/
		Ok(())
	}

	pub fn handle_response(&mut self, res: types::RpcResponse) -> Result<(), Error> {
		debug!(LOGGER, "Received response with id: {}", res.id);
		//TODO: this response needs to be matched up with the request somehow.. for the moment
		//assume it's just a response to a get_job_template request
		let params = res.result.unwrap();
		let params:types::JobTemplate = serde_json::from_str(&params).unwrap();
		let miner_message = types::MinerMessage {
			m_type: types::MinerMessageType::ReceivedJob,
			difficulty: params.difficulty,
			pre_pow: params.pre_pow,
		};
		self.miner_tx.send(miner_message).unwrap();
	
		Ok(())
	}

	pub fn run(mut self) {
		// Request the first job template
		thread::sleep(time::Duration::from_secs(1));
		let _ = self.send_message_get_job_template();
		
		loop {
			thread::sleep(time::Duration::from_secs(1));
			match self.read_message() {
				Some(m) => {
					// figure out what kind of message,
					// and dispatch appropriately
					debug!(LOGGER, "Received message: {}", m);
					// Is this a request?
					let request:Result<types::RpcRequest, serde_json::Error> = serde_json::from_str(&m);
					if let Ok(r) = request {
						self.handle_request(r);
						continue;
					}
					// Is this a response?
					let response:Result<types::RpcResponse, serde_json::Error> = serde_json::from_str(&m);
					if let Ok(r) = response {
						self.handle_response(r);
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
		}

			
	}
}


