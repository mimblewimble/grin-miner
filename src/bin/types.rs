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

/// Types used for stratum

#[derive(Serialize, Deserialize, Debug)]
pub struct JobTemplate {
	pub difficulty: u64,
	pub pre_pow: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RpcRequest {
	pub id: String,
	pub jsonrpc: String,
	pub method: String,
	pub params: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RpcResponse {
	pub id: String,
	pub jsonrpc: String,
	pub result: Option<String>,
	pub error: Option<RpcError>,
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
	height: u64,
	nonce: u64,
	pow: Vec<u32>,
}

/// Types used for internal communication from stratum client to miner
#[derive(Serialize, Deserialize, Debug)]
pub enum MinerMessageType{
	ReceivedJob,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MinerMessage {
	pub m_type: MinerMessageType,
	pub difficulty: u64,
	pub pre_pow: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientMessageType{
	FoundSolution,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClientMessage {
	pub m_type: ClientMessageType,
	pub difficulty: u64,
	pub pre_pow: String,
}
