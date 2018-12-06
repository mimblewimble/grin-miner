// Copyright 2017 The Grin Developers
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

//! header manipulation utility functions

use byteorder::{BigEndian, ByteOrder};
use plugin::Solution;
use rand::{self, Rng};

/// From grin
/// The target is the 8-bytes hash block hashes must be lower than.
const MAX_TARGET: [u8; 8] = [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff];

pub fn header_data(pre_nonce: &str, post_nonce: &str, nonce: u64) -> Vec<u8> {
	// Turn input strings into vectors
	let mut pre_vec = from_hex_string(pre_nonce);
	let mut post_vec = from_hex_string(post_nonce);

	let mut nonce_bytes = [0; 8];
	BigEndian::write_u64(&mut nonce_bytes, nonce);
	let mut nonce_vec = nonce_bytes.to_vec();

	// Generate new header
	pre_vec.append(&mut nonce_vec);
	pre_vec.append(&mut post_vec);

	pre_vec
}

pub fn get_next_header_data(pre_nonce: &str, post_nonce: &str) -> (u64, Vec<u8>) {
	let nonce: u64 = rand::OsRng::new().unwrap().gen();
	(nonce, header_data(pre_nonce, post_nonce, nonce))
}

/// Helper to determing whether a solution meets a target difficulty
/// based on same algorithm from grin
pub fn meets_difficulty(in_difficulty: u64, sol: Solution) -> bool {
	let max_target = BigEndian::read_u64(&MAX_TARGET);
	let num = BigEndian::read_u64(&sol.hash()[0..8]);
	max_target / num >= in_difficulty
}

/// Helper to convert a hex string
pub fn from_hex_string(in_str: &str) -> Vec<u8> {
	let mut bytes = Vec::new();
	for i in 0..(in_str.len() / 2) {
		let res = u8::from_str_radix(&in_str[2 * i..2 * i + 2], 16);
		match res {
			Ok(v) => bytes.push(v),
			Err(e) => println!("Problem with hex: {}", e),
		}
	}
	bytes
}
