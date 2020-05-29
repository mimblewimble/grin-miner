// Copyright 2020 The Grin Developers
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

/// Difficulty calculation as from Grin
use blake2::blake2b::Blake2b;
use byteorder::{BigEndian, ByteOrder};
use std::cmp::{max, min};
use std::fmt;

// constants from grin
const PROOF_SIZE: usize = 42;

/// The difficulty is defined as the maximum target divided by the block hash.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub struct Difficulty {
	num: u64,
}

impl Difficulty {
	/// Convert a `u32` into a `Difficulty`
	pub fn from_num(num: u64) -> Difficulty {
		// can't have difficulty lower than 1
		Difficulty { num: max(num, 1) }
	}

	/// unscaled proof
	fn from_proof_unscaled(proof: &Proof) -> Difficulty {
		Difficulty::from_num(proof.scaled_difficulty(1u64))
	}

	/// Converts the difficulty into a u64
	pub fn to_num(self) -> u64 {
		self.num
	}
}

impl fmt::Display for Difficulty {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.num)
	}
}

/// A Cuck(at)oo Cycle proof of work, consisting of the edge_bits to get the graph
/// size (i.e. the 2-log of the number of edges) and the nonces
/// of the graph solution. While being expressed as u64 for simplicity,
/// nonces a.k.a. edge indices range from 0 to (1 << edge_bits) - 1
///
/// The hash of the `Proof` is the hash of its packed nonces when serializing
/// them at their exact bit size. The resulting bit sequence is padded to be
/// byte-aligned.

#[derive(Clone, PartialOrd, PartialEq)]
pub struct Proof {
	/// Power of 2 used for the size of the cuckoo graph
	pub edge_bits: u8,
	/// The nonces
	pub nonces: Vec<u64>,
}

impl fmt::Debug for Proof {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "Cuckoo{}(", self.edge_bits)?;
		for (i, val) in self.nonces[..].iter().enumerate() {
			write!(f, "{:x}", val)?;
			if i < self.nonces.len() - 1 {
				write!(f, " ")?;
			}
		}
		write!(f, ")")
	}
}

impl Eq for Proof {}

impl Proof {
	/// Difficulty achieved by this proof with given scaling factor
	fn scaled_difficulty(&self, scale: u64) -> u64 {
		let diff = ((scale as u128) << 64) / (max(1, self.hash().to_u64()) as u128);
		min(diff, <u64>::max_value() as u128) as u64
	}

	/// Hash, as in Grin
	fn hash(&self) -> Hash {
		let nonce_bits = self.edge_bits as usize;
		let mut bitvec = BitVec::new(nonce_bits * PROOF_SIZE);
		for (n, nonce) in self.nonces.iter().enumerate() {
			for bit in 0..nonce_bits {
				if nonce & (1 << bit) != 0 {
					bitvec.set_bit_at(n * nonce_bits + (bit as usize))
				}
			}
		}
		let mut blake2b = Blake2b::new(32);
		blake2b.update(&bitvec.bits);
		let mut ret = [0; 32];
		ret.copy_from_slice(blake2b.finalize().as_bytes());
		Hash(ret)
	}

	/// unscaled difficulty
	pub fn to_difficulty_unscaled(&self) -> Difficulty {
		Difficulty::from_proof_unscaled(&self)
	}
}

struct BitVec {
	bits: Vec<u8>,
}

impl BitVec {
	/// Number of bytes required to store the provided number of bits
	fn bytes_len(bits_len: usize) -> usize {
		(bits_len + 7) / 8
	}

	fn new(bits_len: usize) -> BitVec {
		BitVec {
			bits: vec![0; BitVec::bytes_len(bits_len)],
		}
	}

	fn set_bit_at(&mut self, pos: usize) {
		self.bits[pos / 8] |= 1 << (pos % 8) as u8;
	}
}

impl Hash {
	/// to u64
	pub fn to_u64(&self) -> u64 {
		BigEndian::read_u64(&self.0)
	}

	/// to hex
	pub fn to_hex(&self) -> String {
		util::to_hex(self.0.to_vec())
	}
}

/// A hash to uniquely (or close enough) identify one of the main blockchain
/// constructs. Used pervasively for blocks, transactions and outputs.
#[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub struct Hash([u8; 32]);

impl fmt::Debug for Hash {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let hash_hex = self.to_hex();
		const NUM_SHOW: usize = 12;

		write!(f, "{}", &hash_hex[..NUM_SHOW])
	}
}

impl fmt::Display for Hash {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Debug::fmt(self, f)
	}
}
