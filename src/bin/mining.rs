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

use std::sync::mpsc;

use {plugin, types};

pub struct Controller {
	plugin_miner: plugin::PluginMiner,
	rx: mpsc::Receiver<String>,
	pub tx: mpsc::Sender<String>,
	client_tx: Option<mpsc::Sender<String>>,
}

impl Controller {
	pub fn new(pm: plugin::PluginMiner) -> Result<Controller, String> {
		let (tx, rx) = mpsc::channel::<String>();
		Ok(Controller {
			plugin_miner: pm,
			rx: rx,
			tx: tx,
			client_tx: None,
		})
	}

	pub fn set_client_tx(&mut self, client_tx: mpsc::Sender<String>) {
		self.client_tx = Some(client_tx);
	}
	
	/// Run the mining controller
	pub fn run(&mut self){
		loop {
			while let Some(message) = self.rx.try_iter().next() {
				/*match message {


				}*/
			}
		}
	}
}
