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
	pub job_id: u64,
	pub difficulty: u64,
	pub pre_pow: String,
	pub xn: String,
	pub cleanjob: bool,
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
	pub error: Option<RpcError>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RpcError {
	pub code: i32,
	pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoginParams {
	pub login: String,
	pub pass: String,
	pub agent: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SubmitParams {
	pub height: u64,
	pub job_id: u64,
	pub edge_bits: u32,
	pub nonce: u64,
	pub pow: Vec<u64>,
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
	ReceivedJob(u64, u64, u64, String, String, bool),
	StopJob,
	Shutdown,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientMessage{
	// height, job_id, edge_bits, nonce, pow
	FoundSolution(u64, u64, u32, u64, Vec<u64>),
	Shutdown,
}
