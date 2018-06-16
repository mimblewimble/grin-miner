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

use serde_json::Value;

/// Types used for stratum

#[derive(Serialize, Deserialize, Debug)]
pub struct JobTemplate {
	pub height: u64,
	pub difficulty: u64,
	pub pre_pow: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RpcRequest {
	pub id: String,
	pub jsonrpc: String,
	pub method: String,
	pub params: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RpcResponse {
	pub id: String,
	pub method: String,
	pub jsonrpc: String,
	pub result: Option<Value>,
	pub error: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RpcError {
	code: i32,
	message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoginParams {
	login: String,
	pass: String,
	agent: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SubmitParams {
	pub height: u64,
	pub nonce: u64,
	pub pow: Vec<u32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WorkerStatus {
        pub id: String,
        pub height: u64,
        pub difficulty: u64,
        pub accepted: u64,
        pub rejected: u64,
        pub stale: u64,
}

/// Types used for internal communication from stratum client to miner
#[derive(Serialize, Deserialize, Debug)]
pub enum MinerMessage{
	// Height, difficulty, pre_pow
	ReceivedJob(u64, u64, String),
	StopJob,
	Shutdown,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientMessage{
	// height, nonce, pow
	FoundSolution(u64, u64, Vec<u32>),
	Shutdown,
}
