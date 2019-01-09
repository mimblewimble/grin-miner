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

//! Mining status view definition

use std::cmp::Ordering;
use std::sync::{Arc, RwLock};

use cursive::direction::Orientation;
use cursive::traits::*;
use cursive::view::View;
use cursive::views::{BoxView, Dialog, LinearLayout, StackView, TextView};
use cursive::Cursive;

use tui::constants::*;
use tui::types::*;

use plugin::SolverStats;
use stats;
use tui::table::{TableView, TableViewItem};

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
enum MiningDeviceColumn {
	Plugin,
	DeviceId,
	DeviceName,
	EdgeBits,
	ErrorStatus,
	LastGraphTime,
	GraphsPerSecond,
}

impl MiningDeviceColumn {
	fn _as_str(&self) -> &str {
		match *self {
			MiningDeviceColumn::Plugin => "Plugin",
			MiningDeviceColumn::DeviceId => "Device ID",
			MiningDeviceColumn::DeviceName => "Name",
			MiningDeviceColumn::EdgeBits => "Graph Size",
			MiningDeviceColumn::ErrorStatus => "Status",
			MiningDeviceColumn::LastGraphTime => "Last Graph Time",
			MiningDeviceColumn::GraphsPerSecond => "GPS",
		}
	}
}

impl TableViewItem<MiningDeviceColumn> for SolverStats {
	fn to_column(&self, column: MiningDeviceColumn) -> String {
		let last_solution_time_secs = self.last_solution_time as f64 / 1000000000.0;
		match column {
			MiningDeviceColumn::Plugin => self.get_plugin_name(),
			MiningDeviceColumn::DeviceId => format!("{}", self.device_id).to_owned(),
			MiningDeviceColumn::DeviceName => self.get_device_name(),
			MiningDeviceColumn::EdgeBits => format!("{}", self.edge_bits).to_owned(),
			MiningDeviceColumn::ErrorStatus => match self.has_errored {
				false => String::from("OK"),
				_ => String::from("Errored"),
			},
			MiningDeviceColumn::LastGraphTime => {
				String::from(format!("{}s", last_solution_time_secs))
			}
			MiningDeviceColumn::GraphsPerSecond => {
				String::from(format!("{:.*}", 4, 1.0 / last_solution_time_secs))
			}
		}
	}

	fn cmp(&self, other: &Self, column: MiningDeviceColumn) -> Ordering
	where
		Self: Sized,
	{
		let last_solution_time_secs_self = self.last_solution_time as f64 / 1000000000.0;
		let gps_self = 1.0 / last_solution_time_secs_self;
		let last_solution_time_secs_other = other.last_solution_time as f64 / 1000000000.0;
		let gps_other = 1.0 / last_solution_time_secs_other;
		match column {
			MiningDeviceColumn::Plugin => self.plugin_name.cmp(&other.plugin_name),
			MiningDeviceColumn::DeviceId => self.device_id.cmp(&other.device_id),
			MiningDeviceColumn::DeviceName => self.device_name.cmp(&other.device_name),
			MiningDeviceColumn::EdgeBits => self.edge_bits.cmp(&other.edge_bits),
			MiningDeviceColumn::ErrorStatus => self.has_errored.cmp(&other.has_errored),
			MiningDeviceColumn::LastGraphTime => {
				self.last_solution_time.cmp(&other.last_solution_time)
			}
			MiningDeviceColumn::GraphsPerSecond => gps_self.partial_cmp(&gps_other).unwrap(),
		}
	}
}

/// Mining status view
pub struct TUIMiningView;

impl TUIStatusListener for TUIMiningView {
	/// Create the mining view
	fn create() -> Box<View> {
		let table_view = TableView::<SolverStats, MiningDeviceColumn>::new()
			.column(MiningDeviceColumn::Plugin, "Plugin", |c| {
				c.width_percent(20)
			}).column(MiningDeviceColumn::DeviceId, "Device ID", |c| {
				c.width_percent(5)
			}).column(MiningDeviceColumn::DeviceName, "Device Name", |c| {
				c.width_percent(20)
			}).column(MiningDeviceColumn::EdgeBits, "Size", |c| c.width_percent(5))
			.column(MiningDeviceColumn::ErrorStatus, "Status", |c| {
				c.width_percent(8)
			}).column(MiningDeviceColumn::LastGraphTime, "Graph Time", |c| {
				c.width_percent(10)
			}).column(MiningDeviceColumn::GraphsPerSecond, "GPS", |c| {
				c.width_percent(10)
			});

		let status_view =
			LinearLayout::new(Orientation::Vertical)
				.child(LinearLayout::new(Orientation::Horizontal).child(
					TextView::new("Connection Status: Starting...").with_id("mining_server_status"),
				)).child(
					LinearLayout::new(Orientation::Horizontal)
						.child(TextView::new("Mining Status: ").with_id("mining_status")),
				).child(
					LinearLayout::new(Orientation::Horizontal)
						.child(TextView::new("  ").with_id("network_info")),
				).child(
				LinearLayout::new(Orientation::Horizontal)
					.child(TextView::new("  ").with_id("mining_statistics")),
				).child(
				LinearLayout::new(Orientation::Horizontal)
					.child(TextView::new("Last Message Sent:  ").with_id("last_message_sent")),
				).child(LinearLayout::new(Orientation::Horizontal).child(
				TextView::new("Last Message Received:  ").with_id("last_message_received"),
				));

		let mining_device_view = LinearLayout::new(Orientation::Vertical)
			.child(status_view)
			.child(BoxView::with_full_screen(
				Dialog::around(table_view.with_id(TABLE_MINING_STATUS).min_size((50, 20)))
					.title("Mining Devices"),
			)).with_id("mining_device_view");

		let view_stack = StackView::new()
			.layer(mining_device_view)
			.with_id("mining_stack_view");

		let mining_view = LinearLayout::new(Orientation::Vertical).child(view_stack);

		Box::new(mining_view.with_id(VIEW_MINING))
	}

	/// update
	fn update(c: &mut Cursive, stats: Arc<RwLock<stats::Stats>>) {

		let (client_stats, mining_stats) = {
			let stats = stats.read().unwrap();
			(stats.client_stats.clone(), stats.mining_stats.clone())
		};

		c.call_on_id("mining_server_status", |t: &mut TextView| {
			t.set_content(client_stats.connection_status.clone());
		});

		let (basic_mining_status, basic_network_info) = {
			if client_stats.connected {
				if mining_stats.combined_gps() == 0.0 {
					(
						"Mining Status: Starting miner and awaiting first graph time..."
							.to_string(),
						" ".to_string(),
					)
				} else {
					(
						format!(
							"Mining Status: Mining at height {} at {:.*} GPS",
							mining_stats.block_height, 4, mining_stats.combined_gps()
						),
						format!(
							"Cuck(at)oo - Target Share Difficulty {}",
							mining_stats.target_difficulty.to_string()
						),
					)
				}
			} else {
				(
					"Mining Status: Waiting for server".to_string(),
					"  ".to_string(),
				)
			}
		};

		// device
		c.call_on_id("mining_status", |t: &mut TextView| {
			t.set_content(basic_mining_status);
		});
		c.call_on_id("network_info", |t: &mut TextView| {
			t.set_content(basic_network_info);
		});

		c.call_on_id("last_message_sent", |t: &mut TextView| {
			t.set_content(client_stats.last_message_sent.clone());
		});
		c.call_on_id("last_message_received", |t: &mut TextView| {
			t.set_content(client_stats.last_message_received.clone());
		});

		if mining_stats.solution_stats.num_solutions_found > 0 {
			let sol_stat = format!("Solutions found: {}. Accepted: {}, Rejected: {}, Stale: {}, Blocks found: {}",
								   mining_stats.solution_stats.num_solutions_found,
								   mining_stats.solution_stats.num_shares_accepted,
								   mining_stats.solution_stats.num_rejected,
								   mining_stats.solution_stats.num_staled,
								   mining_stats.solution_stats.num_blocks_found,
			);
			c.call_on_id("mining_statistics", |t: &mut TextView| {
				t.set_content(sol_stat);
			});
		}

		let _ = c.call_on_id(
			TABLE_MINING_STATUS,
			|t: &mut TableView<SolverStats, MiningDeviceColumn>| {
				t.set_items(mining_stats.device_stats);
			},
		);
	}
}
