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

#![deny(non_upper_case_globals)]
#![deny(non_camel_case_types)]
#![deny(non_snake_case)]
#![deny(unused_mut)]
#![warn(missing_docs)]

use libc::*;
use std::{fmt, cmp};

use blake2::blake2b::Blake2b;
use byteorder::{ByteOrder, BigEndian};

pub const PROOFSIZE: usize = 42;
pub const MAX_DEVICE_NAME_LEN: usize = 256;
pub const MAX_SOLS: usize = 4;

/// A solver context, opaque reference to C++ type underneath
#[derive(Clone, Debug)]
pub enum SolverCtx {}

/// Common parameters for a solver
#[derive(Clone, Debug, Serialize, Deserialize)]
#[repr(C)]
pub struct SolverParams {
	/// threads
	pub nthreads: uint32_t,
	/// trims
	pub ntrims: uint32_t,
	/// Whether to show cycle (should be true to get solutions)
	pub showcycle: bool,
	/// allrounds
	pub allrounds: bool,
	/// whether to apply the nonce to the header, or leave as is,
	/// letting caller mutate nonce
	pub mutate_nonce: bool,
}

impl Default for SolverParams {
	fn default() -> SolverParams {
		SolverParams {
			nthreads: 0,
			ntrims: 0,
			showcycle: true,
			allrounds: false,
			mutate_nonce: false,
		}
	}
}

/// Common stats collected by solvers
#[derive(Clone)]
#[repr(C)]
pub struct SolverStats {
	/// device Id
	pub device_id: uint32_t,
	/// graph size
	pub edge_bits: uint32_t,
	/// device name
	pub device_name: [c_uchar; MAX_DEVICE_NAME_LEN],
	/// last solution start time
	pub last_start_time: uint64_t,
	/// last solution end time
	pub last_end_time: uint64_t,
	/// last solution elapsed time
	pub last_solution_time: uint64_t,
}

impl Default for SolverStats {
	fn default() -> SolverStats {
		SolverStats {
			device_id: 0,
			edge_bits: 0,
			device_name: [0; MAX_DEVICE_NAME_LEN],
			last_start_time: 0,
			last_end_time: 0,
			last_solution_time: 0,
		}
	}
}

/// A single solution
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Solution {
	/// Nonce 
	pub nonce: uint64_t,
	/// Proof
	pub proof: [uint64_t; PROOFSIZE],
}

impl Default for Solution {
	fn default() -> Solution {
		Solution {
			nonce: 0,
			proof: [0u64; PROOFSIZE],
		}
	}
}

impl fmt::Display for Solution {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let mut comma_separated = String::new();

		for num in &self.proof[0..self.proof.len()] {
			comma_separated.push_str(&format!("0x{:X}", &num));
			comma_separated.push_str(", ");
		}
		comma_separated.pop();
		comma_separated.pop();

		write!(f, "Nonce:{} [{}]", self.nonce, comma_separated)
	}
}

impl fmt::Debug for Solution {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{:?}", &self.proof[..])
	}
}

impl cmp::PartialEq for Solution {
	fn eq(&self, other: &Solution) -> bool {
		for i in 0..PROOFSIZE {
			if self.proof[i] != other.proof[i] {
				return false;
			}
		}
		return true;
	}
}

impl Solution {
	/// Converts the proof to a vector of u64s
	pub fn to_u64s(&self) -> Vec<u64> {
		let mut nonces = Vec::with_capacity(PROOFSIZE);
		for n in self.proof.iter() {
			nonces.push(*n as u64);
		}
		nonces
	}

	/// Returns the hash of the solution, as performed in
	/// grin
	/// TODO: Check whether grin sticks to u32s like this
	pub fn hash(&self) -> [u8; 32] {
		// Hash
		let mut blake2b = Blake2b::new(32);
		for n in 0..self.proof.len() {
			let mut bytes = [0; 4];
			BigEndian::write_u32(&mut bytes, self.proof[n] as u32);
			blake2b.update(&bytes);
		}
		let mut ret = [0; 32];
		ret.copy_from_slice(blake2b.finalize().as_bytes());
		ret
	}
}

/// All solutions returned
#[derive(Clone, Copy)]
#[repr(C)]
pub struct SolverSolutions {
	/// graph size
	pub edge_bits: u32,
	/// number of solutions
	pub num_sols: u32,
	/// solutions themselves
	pub sols: [Solution; MAX_SOLS],
}

impl Default for SolverSolutions {
	fn default() -> SolverSolutions {
		SolverSolutions {
			edge_bits: 0,
			num_sols: 0,
			sols: [Solution::default(); MAX_SOLS],
		}
	}
}
