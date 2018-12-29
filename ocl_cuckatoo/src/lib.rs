extern crate blake2_rfc;
extern crate byteorder;
extern crate grin_miner_plugin as plugin;
extern crate hashbrown;
extern crate libc;
extern crate ocl;
#[macro_use]
extern crate slog;

extern crate grin_miner_util as util;

use blake2_rfc::blake2b::blake2b;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::Cursor;
use std::io::Error;
use std::time::{Duration, SystemTime};
use util::{init_logger, LOGGER};

mod finder;
mod trimmer;
use std::mem;
use std::ptr;

pub use self::finder::Graph;
pub use self::trimmer::Trimmer;

use libc::*;
use plugin::*;

#[repr(C)]
struct Solver {
	trimmer: Trimmer,
	graph: Option<Graph>,
	mutate_nonce: bool,
}

#[no_mangle]
pub unsafe extern "C" fn create_solver_ctx(params: *mut SolverParams) -> *mut SolverCtx {
	let platform = match (*params).platform {
		1 => Some("AMD"),
		2 => Some("NVIDIA"),
		_ => None,
	};
	let device_id = Some((*params).device as usize);
	let mut edge_bits = (*params).edge_bits as u8;
	if edge_bits < 31 || edge_bits > 64 {
		edge_bits = 31;
	}
	println!(
		"Platform {:?}, device {:?} bits {}",
		platform, device_id, edge_bits
	);
	let trimmer = Trimmer::build(platform, device_id, edge_bits).expect("can't build trimmer");
	let solver = Solver {
		trimmer: trimmer,
		graph: None,
		mutate_nonce: (*params).mutate_nonce,
	};
	let solver_box = Box::new(solver);
	let solver_ref = Box::leak(solver_box);
	mem::transmute::<&mut Solver, *mut SolverCtx>(solver_ref)
}

#[no_mangle]
pub unsafe extern "C" fn destroy_solver_ctx(solver_ctx_ptr: *mut SolverCtx) {
	// create box to clear memory
	let solver_ptr = mem::transmute::<*mut SolverCtx, *mut Solver>(solver_ctx_ptr);
	let _solver_box = Box::from_raw(solver_ptr);
}

#[no_mangle]
pub unsafe extern "C" fn stop_solver(solver_ctx_ptr: *mut SolverCtx) {}

#[no_mangle]
pub unsafe extern "C" fn fill_default_params(params: *mut SolverParams) {}

#[no_mangle]
pub unsafe extern "C" fn run_solver(
	ctx: *mut SolverCtx,
	header_ptr: *const c_uchar,
	header_length: uint32_t,
	nonce: uint64_t,
	range: uint32_t,
	solutions: *mut SolverSolutions,
	stats: *mut SolverStats,
) -> uint32_t {
	info!(LOGGER, "XXX Solving");
	let start = SystemTime::now();
	let solver_ptr = mem::transmute::<*mut SolverCtx, *mut Solver>(ctx);
	let solver = &*solver_ptr;
	let mut header = Vec::with_capacity(header_length as usize);
	let r_ptr = header.as_mut_ptr();
	ptr::copy_nonoverlapping(header_ptr, r_ptr, header_length as usize);
	header.set_len(header_length as usize);
	let n = nonce as u32;
	let k = match set_header_nonce(&header, Some(n), solver.mutate_nonce) {
		Err(e) => {
			debug!(LOGGER, "can't process header");
			return 2;
		}
		Ok(v) => v,
	};
	//println!("K is {:x?}", k);
	let res = solver.trimmer.run(&k).unwrap();
	debug!(LOGGER, "Trimmed to {}", res.len());

	let sols = Graph::search(&res).unwrap();
	let end = SystemTime::now();
	let elapsed = end.duration_since(start).unwrap();
	let mut i = 0;
	(*solutions).edge_bits = 31;
	(*solutions).num_sols = sols.len() as u32;
	for sol in sols {
		debug!(LOGGER, "Solution: {:x?}", sol.nonces);
		(*solutions).sols[i].nonce = nonce;
		(*solutions).sols[i]
			.proof
			.copy_from_slice(&sol.nonces[..sol.nonces.len()]);
		i += 1;
	}
	(*stats).edge_bits = 31;
	(*stats).device_id = solver.trimmer.device_id as u32;
	let name_bytes = solver.trimmer.device_name.as_bytes();
	let n = std::cmp::min((*stats).device_name.len(), name_bytes.len());
	(*stats).device_name[..n].copy_from_slice(&solver.trimmer.device_name.as_bytes()[..n]);
	(*stats).last_solution_time = duration_to_u64(elapsed);
	(*stats).last_start_time =
		duration_to_u64(start.duration_since(SystemTime::UNIX_EPOCH).unwrap());
	(*stats).last_end_time = duration_to_u64(end.duration_since(SystemTime::UNIX_EPOCH).unwrap());
	0
}

fn duration_to_u64(elapsed: Duration) -> u64 {
	elapsed.as_secs() * 1_000_000_000 + elapsed.subsec_nanos() as u64
}

pub fn set_header_nonce(
	header: &[u8],
	nonce: Option<u32>,
	mutate_nonce: bool,
) -> Result<[u64; 4], Error> {
	if let Some(n) = nonce {
		let len = header.len();
		let mut header = header.to_owned();
		if mutate_nonce {
			header.truncate(len - 4); // drop last 4 bytes (u32) off the end
			header.write_u32::<LittleEndian>(n)?;
		}
		create_siphash_keys(&header)
	} else {
		create_siphash_keys(&header)
	}
}

pub fn create_siphash_keys(header: &[u8]) -> Result<[u64; 4], Error> {
	let h = blake2b(32, &[], &header);
	let hb = h.as_bytes();
	let mut rdr = Cursor::new(hb);
	Ok([
		rdr.read_u64::<LittleEndian>()?,
		rdr.read_u64::<LittleEndian>()?,
		rdr.read_u64::<LittleEndian>()?,
		rdr.read_u64::<LittleEndian>()?,
	])
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn test_solve() {
		let trimmer = Trimmer::build(None, None, 29).expect("can't build trimmer");
		let k = [
			0x5947f1297c7cd34a,
			0x802becf646e29b67,
			0xe0c878d10d3af2ed,
			0x5843ba0843699326,
		];

		let res = trimmer.run(&k).unwrap();
		debug!(LOGGER, "Trimmed to {}", res.len());

		let sols = Graph::search(&res).unwrap();
		assert_eq!(1, sols.len());
		for sol in sols {
			println!("Solution: {:x?}", sol.nonces);
		}
	}
}
