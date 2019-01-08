use ocl;
use ocl::enums::ProfilingInfo;
use ocl::flags::CommandQueueProperties;
use ocl::{
	Buffer, Context, Device, Event, EventList, Kernel, Platform, Program, Queue, SpatialDims,
};
use std::time::SystemTime;

const DUCK_SIZE_A: usize = 129; // AMD 126 + 3
const DUCK_SIZE_B: usize = 83;
const BUFFER_SIZE_A1: usize = DUCK_SIZE_A * 1024 * (4096 - 128) * 2;
const BUFFER_SIZE_A2: usize = DUCK_SIZE_A * 1024 * 256 * 2;
const BUFFER_SIZE_B: usize = DUCK_SIZE_B * 1024 * 4096 * 2;
const BUFFER_SIZE_U32: usize = (DUCK_SIZE_A + DUCK_SIZE_B) * 1024 * 4096 * 2;
const INDEX_SIZE: usize = 256 * 256 * 4;

pub struct Trimmer {
	q: Queue,
	program: Program,
	buffer_a1: Buffer<u32>,
	buffer_a2: Buffer<u32>,
	buffer_b: Buffer<u32>,
	buffer_i1: Buffer<u32>,
	buffer_i2: Buffer<u32>,
	buffer_r: Buffer<u32>,
	buffer_nonces: Buffer<u32>,
	pub device_name: String,
	pub device_id: usize,
}

impl Trimmer {
	pub fn build(platform_name: Option<&str>, device_id: Option<usize>) -> ocl::Result<Trimmer> {
		let platform = find_paltform(platform_name)
			.ok_or::<ocl::Error>("Can't find OpenCL platform".into())?;
		let device = find_device(&platform, device_id)?;

		let context = Context::builder()
			.platform(platform)
			.devices(device)
			.build()?;

		let q = Queue::new(
			&context,
			device,
			Some(CommandQueueProperties::PROFILING_ENABLE),
		)?;

		let program = Program::builder()
			.devices(device)
			.src(SRC)
			.build(&context)?;

		let buffer_a1 = Buffer::<u32>::builder()
			.queue(q.clone())
			.len(BUFFER_SIZE_A1)
			.fill_val(0)
			.build()?;

		let buffer_a2 = Buffer::<u32>::builder()
			.queue(q.clone())
			.len(BUFFER_SIZE_A2)
			.fill_val(0)
			.build()?;

		let buffer_b = Buffer::<u32>::builder()
			.queue(q.clone())
			.len(BUFFER_SIZE_B)
			.fill_val(0)
			.build()?;

		let buffer_i1 = Buffer::<u32>::builder()
			.queue(q.clone())
			.len(INDEX_SIZE)
			.fill_val(0)
			.build()?;

		let buffer_i2 = Buffer::<u32>::builder()
			.queue(q.clone())
			.len(INDEX_SIZE)
			.fill_val(0)
			.build()?;

		let buffer_r = Buffer::<u32>::builder()
			.queue(q.clone())
			.len(42 * 2)
			.flags(ocl::flags::MemFlags::READ_ONLY)
			.fill_val(0)
			.build()?;

		let buffer_nonces = Buffer::<u32>::builder()
			.queue(q.clone())
			.len(INDEX_SIZE)
			.fill_val(0)
			.build()?;

		//let result = unsafe {
		//	Buffer::<u32>::builder()
		//		.queue(q.clone())
		//		.len(RES_BUFFER_SIZE)
		//		.fill_val(0)
		//		.use_host_slice(&res_buf[..])
		//		.build()?
		//};

		Ok(Trimmer {
			q,
			program,
			buffer_a1,
			buffer_a2,
			buffer_b,
			buffer_i1,
			buffer_i2,
			buffer_r,
			buffer_nonces,
			device_name: device.name()?,
			device_id: device_id.unwrap_or(0),
		})
	}

	pub unsafe fn recover(&self, mut nodes: Vec<u32>, k: &[u64; 4]) -> ocl::Result<Vec<u32>> {
		let mut event_list = EventList::new();
		let mut names = vec![];

		let mut kernel_recovery = Kernel::builder()
			.name("FluffyRecovery")
			.program(&self.program)
			.queue(self.q.clone())
			.global_work_size(2048 * 256)
			.local_work_size(SpatialDims::One(256))
			.arg(k[0])
			.arg(k[1])
			.arg(k[2])
			.arg(k[3])
			.arg(&self.buffer_r)
			.arg(&self.buffer_nonces)
			.build()?;

		nodes.push(nodes[0]);

		println!("Sending nodes {}", nodes.len());
		let edges = nodes.windows(2).flatten().map(|v| *v).collect::<Vec<u32>>();
		println!("Sending edges {}", edges.len());
		self.buffer_r
			.cmd()
			.enew(&mut event_list)
			.write(edges.as_slice())
			.enq()?;
		names.push("write edges");
		self.buffer_nonces
			.cmd()
			.enew(&mut event_list)
			.fill(0, None)
			.enq()?;
		names.push("fill res");
		kernel_recovery.cmd().enew(&mut event_list).enq()?;
		names.push("recovery");
		let mut nonces: Vec<u32> = vec![0; 42];

		self.buffer_nonces
			.cmd()
			.enew(&mut event_list)
			.read(&mut nonces)
			.enq()?;
		self.q.finish()?;
		nonces.sort();
		for i in 0..names.len() {
			print_event(names[i], &event_list[i]);
		}
		Ok(nonces)
	}

	pub unsafe fn run(&self, k: &[u64; 4]) -> ocl::Result<Vec<u32>> {
		let start = SystemTime::now();
		let mut kernel_seed_a = Kernel::builder()
			.name("FluffySeed2A")
			.program(&self.program)
			.queue(self.q.clone())
			.global_work_size(2048 * 128)
			.local_work_size(SpatialDims::One(128))
			.arg(k[0])
			.arg(k[1])
			.arg(k[2])
			.arg(k[3])
			//.arg(&self.buffer_b)
			//.arg(&self.buffer_a1)
			//.arg(&self.buffer_i1)
			.build()?;

		let mut kernel_seed_b1 = Kernel::builder()
			.name("FluffySeed2B")
			.program(&self.program)
			.queue(self.q.clone())
			.global_work_size(1024 * 128)
			.local_work_size(SpatialDims::One(128))
			.arg(&self.buffer_a1)
			.arg(&self.buffer_a1)
			.arg(&self.buffer_a2)
			.arg(&self.buffer_i1)
			.arg(&self.buffer_i2)
			.arg(32)
			.build()?;

		let mut kernel_seed_b2 = Kernel::builder()
			.name("FluffySeed2B")
			.program(&self.program)
			.queue(self.q.clone())
			.global_work_size(1024 * 128)
			.local_work_size(SpatialDims::One(128))
			.arg(&self.buffer_b)
			.arg(&self.buffer_a1)
			.arg(&self.buffer_a2)
			.arg(&self.buffer_i1)
			.arg(&self.buffer_i2)
			.arg(0)
			.build()?;

		let mut kernel_round1 = Kernel::builder()
			.name("FluffyRound1")
			.program(&self.program)
			.queue(self.q.clone())
			.global_work_size(4096 * 1024)
			.local_work_size(SpatialDims::One(1024))
			.arg(&self.buffer_a1)
			.arg(&self.buffer_a2)
			.arg(&self.buffer_b)
			.arg(&self.buffer_i2)
			.arg(&self.buffer_i1)
			.arg((DUCK_SIZE_A * 1024) as i32)
			.arg((DUCK_SIZE_B * 1024) as i32)
			.build()?;

		let mut kernel_round_na = Kernel::builder()
			.name("FluffyRoundN")
			.program(&self.program)
			.queue(self.q.clone())
			.global_work_size(4096 * 1024)
			.local_work_size(SpatialDims::One(1024))
			.arg(&self.buffer_b)
			.arg(&self.buffer_a1)
			.arg(&self.buffer_i1)
			.arg(&self.buffer_i2)
			.build()?;

		let mut kernel_round_nb = Kernel::builder()
			.name("FluffyRoundN")
			.program(&self.program)
			.queue(self.q.clone())
			.global_work_size(4096 * 1024)
			.local_work_size(SpatialDims::One(1024))
			.arg(&self.buffer_a1)
			.arg(&self.buffer_b)
			.arg(&self.buffer_i2)
			.arg(&self.buffer_i1)
			.build()?;

		let mut kernel_tail = Kernel::builder()
			.name("FluffyTail")
			.program(&self.program)
			.queue(self.q.clone())
			.global_work_size(4096 * 1024)
			.local_work_size(SpatialDims::One(1024))
			.arg(&self.buffer_b)
			.arg(&self.buffer_a1)
			.arg(&self.buffer_i1)
			.arg(&self.buffer_i2)
			.build()?;

		let end = SystemTime::now();
		let elapsed = end.duration_since(start).unwrap();
		println!("Time preparing kernels: {:?}", elapsed);

		//macro_rules! kernel_enq (
		//($num:expr) => (
		//for i in 0..$num {
		//    offset = i * GLOBAL_WORK_SIZE;
		//    unsafe {
		//        kernel
		//            .set_default_global_work_offset(SpatialDims::One(offset))
		//            .enq()?;
		//    }
		//}
		//));

		macro_rules! clear_buf (
	($buf:expr) => (
		$buf.cmd().fill(0, None).enq()?;
	));
		let mut event_list = EventList::new();
		let mut names = vec![];

		let mut edges_count: Vec<u32> = vec![0; 1];
		kernel_seed_a.cmd().enew(&mut event_list).enq()?;
		names.push("seedA");
		kernel_seed_b1.cmd().enew(&mut event_list).enq()?;
		names.push("seedB1");
		kernel_seed_b2.cmd().enew(&mut event_list).enq()?;
		names.push("seedB2");
		clear_buf!(self.buffer_i1);
		kernel_round1.enq()?;

		for _ in 0..80 {
			clear_buf!(self.buffer_i2);
			kernel_round_na.cmd().enew(&mut event_list).enq()?;
			names.push("seedNA");
			clear_buf!(self.buffer_i1);
			kernel_round_nb.cmd().enew(&mut event_list).enq()?;
			names.push("seedNB");
		}
		clear_buf!(self.buffer_i2);
		kernel_tail.cmd().enew(&mut event_list).enq()?;
		names.push("tail");

		self.buffer_i2
			.cmd()
			.enew(&mut event_list)
			.read(&mut edges_count)
			.enq()?;
		names.push("read I2");
		//self.buffer_a1.map().enq()?;

		let mut edges_left: Vec<u32> = vec![0; (edges_count[0] * 2) as usize];

		self.buffer_a1
			.cmd()
			.enew(&mut event_list)
			.read(&mut edges_left)
			.enq()?;
		names.push("read A2");
		self.q.finish()?;
		println!("Event list {:?}", event_list);
		for i in 0..names.len() {
			print_event(names[i], &event_list[i]);
		}
		println!("edges {}", edges_count[0]);
		println!(
			"nodes {}: ({}, {})",
			edges_left.len(),
			edges_left[0],
			edges_left[1]
		);
		clear_buf!(self.buffer_i1);
		clear_buf!(self.buffer_i2);
		self.q.finish()?;
		Ok(edges_left)
	}
}

fn print_event(name: &str, ev: &Event) {
	let submit = ev
		.profiling_info(ProfilingInfo::Submit)
		.unwrap()
		.time()
		.unwrap();
	let queued = ev
		.profiling_info(ProfilingInfo::Queued)
		.unwrap()
		.time()
		.unwrap();
	let start = ev
		.profiling_info(ProfilingInfo::Start)
		.unwrap()
		.time()
		.unwrap();
	let end = ev
		.profiling_info(ProfilingInfo::End)
		.unwrap()
		.time()
		.unwrap();
	println!(
		"{}\t total {}ms \t sub {}mc \t start {}ms \t exec {}ms",
		name,
		(end - queued) / 1_000_000,
		(submit - queued) / 1_000,
		(start - submit) / 1_000_000,
		(end - start) / 1_000_000
	);
}

fn find_paltform(selector: Option<&str>) -> Option<Platform> {
	match selector {
		None => Some(Platform::default()),
		Some(sel) => Platform::list().into_iter().find(|p| {
			if let Ok(vendor) = p.name() {
				vendor.contains(sel)
			} else {
				false
			}
		}),
	}
}

fn find_device(platform: &Platform, selector: Option<usize>) -> ocl::Result<Device> {
	match selector {
		None => Device::first(platform),
		Some(index) => Device::by_idx_wrap(platform, index),
	}
}

const SRC: &str = r#"
// Cuckaroo Cycle, a memory-hard proof-of-work by John Tromp and team Grin
// Copyright (c) 2018 Jiri Photon Vadura and John Tromp
// This GGM miner file is covered by the FAIR MINING license

#pragma OPENCL EXTENSION cl_khr_int64_base_atomics : enable
#pragma OPENCL EXTENSION cl_khr_int64_extended_atomics : enable

typedef uint8 u8;
typedef uint16 u16;
typedef uint u32;
typedef ulong u64;

typedef u32 node_t;
typedef u64 nonce_t;


#define DUCK_SIZE_A 129L
#define DUCK_SIZE_B 83L

#define DUCK_A_EDGES (DUCK_SIZE_A * 1024L)
#define DUCK_A_EDGES_64 (DUCK_A_EDGES * 64L)

#define DUCK_B_EDGES (DUCK_SIZE_B * 1024L)
#define DUCK_B_EDGES_64 (DUCK_B_EDGES * 64L)

#define EDGE_BLOCK_SIZE (64)
#define EDGE_BLOCK_MASK (EDGE_BLOCK_SIZE - 1)

#define EDGEBITS 29
// number of edges
#define NEDGES ((node_t)1 << EDGEBITS)
// used to mask siphash output
#define EDGEMASK (NEDGES - 1)

#define CTHREADS 1024
#define BKTMASK4K (4096-1)
#define BKTGRAN 32

#define SIPROUND \
  do { \
    v0 += v1; v2 += v3; v1 = rotate(v1,(ulong)13); \
    v3 = rotate(v3,(ulong)16); v1 ^= v0; v3 ^= v2; \
    v0 = rotate(v0,(ulong)32); v2 += v1; v0 += v3; \
    v1 = rotate(v1,(ulong)17);   v3 = rotate(v3,(ulong)21); \
    v1 ^= v2; v3 ^= v0; v2 = rotate(v2,(ulong)32); \
  } while(0)


void Increase2bCounter(__local u32 * ecounters, const int bucket)
{
	int word = bucket >> 5;
	unsigned char bit = bucket & 0x1F;
	u32 mask = 1 << bit;

	u32 old = atomic_or(ecounters + word, mask) & mask;

	if (old > 0)
		atomic_or(ecounters + word + 4096, mask);
}

bool Read2bCounter(__local u32 * ecounters, const int bucket)
{
	int word = bucket >> 5;
	unsigned char bit = bucket & 0x1F;
	u32 mask = 1 << bit;

	return (ecounters[word + 4096] & mask) > 0;
}

__attribute__((reqd_work_group_size(128, 1, 1)))
__kernel  void FluffySeed2A(const u64 v0i, const u64 v1i, const u64 v2i, const u64 v3i, __global ulong4 * bufferA, __global ulong4 * buffer_b, __global u32 * indexes)
{
	const int gid = get_global_id(0);
	const short lid = get_local_id(0);

	__global ulong4 * buffer;
	__local u64 tmp[64][16];
	__local u32 counters[64];
	u64 sipblock[64];

	u64 v0;
	u64 v1;
	u64 v2;
	u64 v3;

	if (lid < 64)
		counters[lid] = 0;

	barrier(CLK_LOCAL_MEM_FENCE);

	for (int i = 0; i < 1024 * 2; i += EDGE_BLOCK_SIZE)
	{
		u64 blockNonce = gid * (1024 * 2) + i;

		v0 = v0i;
		v1 = v1i;
		v2 = v2i;
		v3 = v3i;

		for (u32 b = 0; b < EDGE_BLOCK_SIZE; b++)
		{
			v3 ^= blockNonce + b;
			for (int r = 0; r < 2; r++)
				SIPROUND;
			v0 ^= blockNonce + b;
			v2 ^= 0xff;
			for (int r = 0; r < 4; r++)
				SIPROUND;

			sipblock[b] = (v0 ^ v1) ^ (v2  ^ v3);

		}
		u64 last = sipblock[EDGE_BLOCK_MASK];

		for (short s = 0; s < EDGE_BLOCK_SIZE; s++)
		{
			u64 lookup = s == EDGE_BLOCK_MASK ? last : sipblock[s] ^ last;
			uint2 hash = (uint2)(lookup & EDGEMASK, (lookup >> 32) & EDGEMASK);
			int bucket = hash.x & 63;

			barrier(CLK_LOCAL_MEM_FENCE);

			int counter = atomic_add(counters + bucket, (u32)1);
			int counterLocal = counter % 16;
			tmp[bucket][counterLocal] = hash.x | ((u64)hash.y << 32);

			barrier(CLK_LOCAL_MEM_FENCE);

			if ((counter > 0) && (counterLocal == 0 || counterLocal == 8))
			{
				int cnt = min((int)atomic_add(indexes + bucket, 8), (int)(DUCK_A_EDGES_64 - 8));
				int idx = ((bucket < 32 ? bucket : bucket - 32) * DUCK_A_EDGES_64 + cnt) / 4;
				buffer = bucket < 32 ? bufferA : buffer_b;

				buffer[idx] = (ulong4)(
					atom_xchg(&tmp[bucket][8 - counterLocal], (u64)0),
					atom_xchg(&tmp[bucket][9 - counterLocal], (u64)0),
					atom_xchg(&tmp[bucket][10 - counterLocal], (u64)0),
					atom_xchg(&tmp[bucket][11 - counterLocal], (u64)0)
				);
				buffer[idx + 1] = (ulong4)(
					atom_xchg(&tmp[bucket][12 - counterLocal], (u64)0),
					atom_xchg(&tmp[bucket][13 - counterLocal], (u64)0),
					atom_xchg(&tmp[bucket][14 - counterLocal], (u64)0),
					atom_xchg(&tmp[bucket][15 - counterLocal], (u64)0)
				);
			}

		}
	}

	barrier(CLK_LOCAL_MEM_FENCE);

	if (lid < 64)
	{
		int counter = counters[lid];
		int counterBase = (counter % 16) >= 8 ? 8 : 0;
		int counterCount = (counter % 8);
		for (int i = 0; i < (8 - counterCount); i++)
			tmp[lid][counterBase + counterCount + i] = 0;
		int cnt = min((int)atomic_add(indexes + lid, 8), (int)(DUCK_A_EDGES_64 - 8));
		int idx = ( (lid < 32 ? lid : lid - 32) * DUCK_A_EDGES_64 + cnt) / 4;
		buffer = lid < 32 ? bufferA : buffer_b;
		buffer[idx] = (ulong4)(tmp[lid][counterBase], tmp[lid][counterBase + 1], tmp[lid][counterBase + 2], tmp[lid][counterBase + 3]);
		buffer[idx + 1] = (ulong4)(tmp[lid][counterBase + 4], tmp[lid][counterBase + 5], tmp[lid][counterBase + 6], tmp[lid][counterBase + 7]);
	}

}

__attribute__((reqd_work_group_size(128, 1, 1)))
__kernel  void FluffySeed2B(const __global uint2 * source, __global ulong4 * destination1, __global ulong4 * destination2, const __global int * sourceIndexes, __global int * destinationIndexes, int startBlock)
{
	const int lid = get_local_id(0);
	const int group = get_group_id(0);

	__global ulong4 * destination = destination1;
	__local u64 tmp[64][16];
	__local int counters[64];

	if (lid < 64)
		counters[lid] = 0;

	barrier(CLK_LOCAL_MEM_FENCE);

	int offsetMem = startBlock * DUCK_A_EDGES_64;
	int offsetBucket = 0;
	const int myBucket = group / BKTGRAN;
	const int microBlockNo = group % BKTGRAN;
	const int bucketEdges = min(sourceIndexes[myBucket + startBlock], (int)(DUCK_A_EDGES_64));
	const int microBlockEdgesCount = (DUCK_A_EDGES_64 / BKTGRAN);
	const int loops = (microBlockEdgesCount / 128);

	if ((startBlock == 32) && (myBucket >= 30))
	{
		offsetMem = 0;
		destination = destination2;
		offsetBucket = 30;
	}

	for (int i = 0; i < loops; i++)
	{
		int edgeIndex = (microBlockNo * microBlockEdgesCount) + (128 * i) + lid;

		{
			uint2 edge = source[/*offsetMem + */(myBucket * DUCK_A_EDGES_64) + edgeIndex];
			bool skip = (edgeIndex >= bucketEdges) || (edge.x == 0 && edge.y == 0);

			int bucket = (edge.x >> 6) & (64 - 1);

			barrier(CLK_LOCAL_MEM_FENCE);

			int counter = 0;
			int counterLocal = 0;

			if (!skip)
			{
				counter = atomic_add(counters + bucket, (u32)1);
				counterLocal = counter % 16;
				tmp[bucket][counterLocal] = edge.x | ((u64)edge.y << 32);
			}

			barrier(CLK_LOCAL_MEM_FENCE);

			if ((counter > 0) && (counterLocal == 0 || counterLocal == 8))
			{
				int cnt = min((int)atomic_add(destinationIndexes + startBlock * 64 + myBucket * 64 + bucket, 8), (int)(DUCK_A_EDGES - 8));
				int idx = (offsetMem + (((myBucket - offsetBucket) * 64 + bucket) * DUCK_A_EDGES + cnt)) / 4;

				destination[idx] = (ulong4)(
					atom_xchg(&tmp[bucket][8 - counterLocal], 0),
					atom_xchg(&tmp[bucket][9 - counterLocal], 0),
					atom_xchg(&tmp[bucket][10 - counterLocal], 0),
					atom_xchg(&tmp[bucket][11 - counterLocal], 0)
				);
				destination[idx + 1] = (ulong4)(
					atom_xchg(&tmp[bucket][12 - counterLocal], 0),
					atom_xchg(&tmp[bucket][13 - counterLocal], 0),
					atom_xchg(&tmp[bucket][14 - counterLocal], 0),
					atom_xchg(&tmp[bucket][15 - counterLocal], 0)
				);
			}
		}
	}

	barrier(CLK_LOCAL_MEM_FENCE);

	if (lid < 64)
	{
		int counter = counters[lid];
		int counterBase = (counter % 16) >= 8 ? 8 : 0;
		int cnt = min((int)atomic_add(destinationIndexes + startBlock * 64 + myBucket * 64 + lid, 8), (int)(DUCK_A_EDGES - 8));
		int idx = (offsetMem + (((myBucket - offsetBucket) * 64 + lid) * DUCK_A_EDGES + cnt)) / 4;
		destination[idx] = (ulong4)(tmp[lid][counterBase], tmp[lid][counterBase + 1], tmp[lid][counterBase + 2], tmp[lid][counterBase + 3]);
		destination[idx + 1] = (ulong4)(tmp[lid][counterBase + 4], tmp[lid][counterBase + 5], tmp[lid][counterBase + 6], tmp[lid][counterBase + 7]);
	}
}

__attribute__((reqd_work_group_size(1024, 1, 1)))
__kernel   void FluffyRound1(const __global uint2 * source1, const __global uint2 * source2, __global uint2 * destination, const __global int * sourceIndexes, __global int * destinationIndexes, const int bktInSize, const int bktOutSize)
{
	const int lid = get_local_id(0);
	const int group = get_group_id(0);

	const __global uint2 * source = group < (62 * 64) ? source1 : source2;
	int groupRead                 = group < (62 * 64) ? group : group - (62 * 64);

	__local u32 ecounters[8192];

	const int edgesInBucket = min(sourceIndexes[group], bktInSize);
	const int loops = (edgesInBucket + CTHREADS) / CTHREADS;

	for (int i = 0; i < 8; i++)
		ecounters[lid + (1024 * i)] = 0;

	barrier(CLK_LOCAL_MEM_FENCE);

	for (int i = 0; i < loops; i++)
	{
		const int lindex = (i * CTHREADS) + lid;

		if (lindex < edgesInBucket)
		{

			const int index = (bktInSize * groupRead) + lindex;

			uint2 edge = source[index];

			if (edge.x == 0 && edge.y == 0) continue;

			Increase2bCounter(ecounters, (edge.x & EDGEMASK) >> 12);
		}
	}

	barrier(CLK_LOCAL_MEM_FENCE);

	for (int i = 0; i < loops; i++)
	{
		const int lindex = (i * CTHREADS) + lid;

		if (lindex < edgesInBucket)
		{
			const int index = (bktInSize * groupRead) + lindex;

			uint2 edge = source[index];

			if (edge.x == 0 && edge.y == 0) continue;

			if (Read2bCounter(ecounters, (edge.x & EDGEMASK) >> 12))
			{
				const int bucket = edge.y & BKTMASK4K;
				const int bktIdx = min(atomic_add(destinationIndexes + bucket, 1), bktOutSize - 1);
				destination[(bucket * bktOutSize) + bktIdx] = (uint2)(edge.y, edge.x);
			}
		}
	}

}

__attribute__((reqd_work_group_size(1024, 1, 1)))
__kernel   void FluffyRoundN(const __global uint2 * source, __global uint2 * destination, const __global int * sourceIndexes, __global int * destinationIndexes)
{
	const int lid = get_local_id(0);
	const int group = get_group_id(0);

	const int bktInSize = DUCK_B_EDGES;
	const int bktOutSize = DUCK_B_EDGES;

	__local u32 ecounters[8192];

	const int edgesInBucket = min(sourceIndexes[group], bktInSize);
	const int loops = (edgesInBucket + CTHREADS) / CTHREADS;

	for (int i = 0; i < 8; i++)
		ecounters[lid + (1024 * i)] = 0;

	barrier(CLK_LOCAL_MEM_FENCE);

	for (int i = 0; i < loops; i++)
	{
		const int lindex = (i * CTHREADS) + lid;

		if (lindex < edgesInBucket)
		{

			const int index = (bktInSize * group) + lindex;

			uint2 edge = source[index];

			if (edge.x == 0 && edge.y == 0) continue;

			Increase2bCounter(ecounters, (edge.x & EDGEMASK) >> 12);
		}
	}

	barrier(CLK_LOCAL_MEM_FENCE);

	for (int i = 0; i < loops; i++)
	{
		const int lindex = (i * CTHREADS) + lid;

		if (lindex < edgesInBucket)
		{
			const int index = (bktInSize * group) + lindex;

			uint2 edge = source[index];

			if (edge.x == 0 && edge.y == 0) continue;

			if (Read2bCounter(ecounters, (edge.x & EDGEMASK) >> 12))
			{
				const int bucket = edge.y & BKTMASK4K;
				const int bktIdx = min(atomic_add(destinationIndexes + bucket, 1), bktOutSize - 1);
				destination[(bucket * bktOutSize) + bktIdx] = (uint2)(edge.y, edge.x);
			}
		}
	}

}

__attribute__((reqd_work_group_size(1024, 1, 1)))
__kernel void FluffyTail(const __global uint2 * source, __global uint2 * destination, const __global int * sourceIndexes, __global int * destinationIndexes)
{
	const int lid = get_local_id(0);
	const int group = get_group_id(0);

	int myEdges = sourceIndexes[group];
	__local int destIdx;

	if (lid == 0)
		destIdx = atomic_add(destinationIndexes, myEdges);

	barrier(CLK_LOCAL_MEM_FENCE);

	if (lid < myEdges)
	{
		destination[destIdx + lid] = source[group * DUCK_B_EDGES + lid];
	}
}

__attribute__((reqd_work_group_size(256, 1, 1)))
__kernel   void FluffyRecovery(const u64 v0i, const u64 v1i, const u64 v2i, const u64 v3i, const __constant u64 * recovery, __global int * indexes)
{
	const int gid = get_global_id(0);
	const short lid = get_local_id(0);

	__local u32 nonces[42];
	u64 sipblock[64];

	u64 v0;
	u64 v1;
	u64 v2;
	u64 v3;

	if (lid < 42) nonces[lid] = 0;

	barrier(CLK_LOCAL_MEM_FENCE);

	for (int i = 0; i < 1024; i += EDGE_BLOCK_SIZE)
	{
		u64 blockNonce = gid * 1024 + i;

		v0 = v0i;
		v1 = v1i;
		v2 = v2i;
		v3 = v3i;

		for (u32 b = 0; b < EDGE_BLOCK_SIZE; b++)
		{
			v3 ^= blockNonce + b;
			SIPROUND; SIPROUND;
			v0 ^= blockNonce + b;
			v2 ^= 0xff;
			SIPROUND; SIPROUND; SIPROUND; SIPROUND;

			sipblock[b] = (v0 ^ v1) ^ (v2  ^ v3);

		}
		const u64 last = sipblock[EDGE_BLOCK_MASK];

		for (short s = EDGE_BLOCK_MASK; s >= 0; s--)
		{
			u64 lookup = s == EDGE_BLOCK_MASK ? last : sipblock[s] ^ last;
			u64 u = lookup & EDGEMASK;
			u64 v = (lookup >> 32) & EDGEMASK;

			u64 a = u | (v << 32);
			u64 b = v | (u << 32);

			for (int i = 0; i < 42; i++)
			{
				if ((recovery[i] == a) || (recovery[i] == b))
				//if ((1234 == a) || (5679 == b))
					nonces[i] = blockNonce + s;
			}
		}
	}

	barrier(CLK_LOCAL_MEM_FENCE);

	if (lid < 42)
	{
		if (nonces[lid] > 0)
			indexes[lid] = nonces[lid];
	}
}
"#;
