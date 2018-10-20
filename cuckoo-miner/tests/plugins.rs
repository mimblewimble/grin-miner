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

/// Tests exercising the loading and unloading of plugins, as well as the
/// existence and correct functionality of each plugin function

extern crate rand;
extern crate cuckoo_miner as cuckoo;

use std::path::PathBuf;
use std::{thread, time};
use std::time::Instant;

use cuckoo::CuckooMinerError;
use cuckoo::PluginLibrary;

/*pub mod common;

use common::{
	KNOWN_30_HASH_1,
	KNOWN_16_HASH_1};*/

static DLL_SUFFIX: &str = ".cuckooplugin";

const TEST_PLUGIN_LIBS_CORE : [&str;1] = [
	"cuckatoo_mean_compat_cpu_19",
];

const TEST_PLUGIN_LIBS_OPTIONAL : [&str;1] = [
	"lean_cuda_30",
];

//Helper to convert from hex string
fn from_hex_string(in_str: &str) -> Vec<u8> {
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

//Helper to load a plugin library
fn load_plugin_lib(plugin:&str) -> Result<PluginLibrary, CuckooMinerError> {
	let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	d.push(format!("../target/debug/plugins/{}{}", plugin, DLL_SUFFIX).as_str());
	PluginLibrary::new(d.to_str().unwrap())
}

//Helper to load all plugin libraries specified above
fn load_all_plugins() -> Vec<PluginLibrary>{
	let mut plugin_libs:Vec<PluginLibrary> = Vec::new();
	for p in TEST_PLUGIN_LIBS_CORE.into_iter(){
		plugin_libs.push(load_plugin_lib(p).unwrap());
	}
	for p in TEST_PLUGIN_LIBS_OPTIONAL.into_iter(){
		let pl = load_plugin_lib(p);
		if let Ok(p) = pl {
			plugin_libs.push(p);
		}
	}
	plugin_libs
}

//loads and unloads a plugin many times
#[test]
fn on_commit_plugin_loading(){
	//core plugins should be built on all systems, fail if they don't exist
	for _ in 0..100 {
		for p in TEST_PLUGIN_LIBS_CORE.into_iter() {
			let pl = load_plugin_lib(p).unwrap();
			pl.unload();
		}
	}
	//only test these if they do exist (cuda, etc)
	for _ in 0..100 {
		for p in TEST_PLUGIN_LIBS_OPTIONAL.into_iter() {
			let pl = load_plugin_lib(p);
			if let Err(_) = pl {
				break;
			}
			pl.unwrap().unload();
		}
	}
}

//Loads all plugins at once
#[test]
fn on_commit_plugin_multiple_loading(){
	let _p=load_all_plugins();
}

//tests cuckoo_call() on all available plugins
/*#[test]
fn on_commit_cuckoo_call(){
	let iterations = 1;
	let plugins = load_all_plugins();
	for p in plugins.into_iter() {
		for _ in 0..iterations {
			//Only do 16
			if p.lib_full_path.contains("16"){
				cuckoo_call_tests(&p);
			}
		}
	}
	/*let pl = load_plugin_lib("mean_cpu_30").unwrap();
	cuckoo_call_tests(&pl);*/
	//pl.unload();
	/*let pl2 = load_plugin_lib("mean_cpu_16").unwrap();
	cuckoo_call_tests(&pl2);*/

}

// Helper to test call_cuckoo_start_processing
// Starts up queue, lets it spin for a bit, 
// then shuts it down. Should be no segfaults
// and everything cleared up cleanly

fn call_cuckoo_start_processing_tests(pl: &PluginLibrary){
	println!("Plugin: {}", pl.lib_full_path);
	//Just start processing
	let ret_val=pl.call_cuckoo_start_processing();

	let wait_time = time::Duration::from_millis(25);

	thread::sleep(wait_time);
	pl.call_cuckoo_stop_processing();

	//wait for internal processing to finish
	while pl.call_cuckoo_has_processing_stopped()==0{};
	pl.call_cuckoo_reset_processing();

	println!("{}",ret_val);
	assert!(ret_val==0);
}

//tests call_cuckoo_start_processing 
//on all available plugins
#[test]
fn on_commit_call_cuckoo_start_processing(){
	let iterations = 10;
	let plugins = load_all_plugins();
	for p in plugins.into_iter() {
		for _ in 0..iterations {
			call_cuckoo_start_processing_tests(&p);
		}
	}
}

// Helper to test call_cuckoo_push_to_input_queue

fn call_cuckoo_push_to_input_queue_tests(pl: &PluginLibrary){
	println!("Plugin: {}", pl.lib_full_path);

	//hash too long
	let hash:[u8;42]=[0;42];
	let nonce:[u8;8]=[0;8];
	println!("HASH LEN {}", hash.len());
	let result=pl.call_cuckoo_push_to_input_queue(0, &hash, &nonce);
	println!("Result: {}",result);
	assert!(result==2);

	//basic push
	let hash:[u8;32]=[0;32];
	let nonce:[u8;8]=[0;8];
	let result=pl.call_cuckoo_push_to_input_queue(1, &hash, &nonce);
	assert!(result==0);

	//push until queue is full
	for i in 0..10000 {
		let result=pl.call_cuckoo_push_to_input_queue(i+2, &hash, &nonce);
		if result==1 {
			break;
		}
		//Should have been full long before now
		assert!(i!=10000);
	}

	//should be full
	let result=pl.call_cuckoo_push_to_input_queue(3, &hash, &nonce);
	assert!(result==1);

	//only do this on smaller test cuckoo, or we'll be here forever
	if pl.lib_full_path.contains("16"){
		pl.call_cuckoo_start_processing();
		let wait_time = time::Duration::from_millis(100);
		thread::sleep(wait_time);
		pl.call_cuckoo_stop_processing();
		//wait for internal processing to finish
		while pl.call_cuckoo_has_processing_stopped()==0{};
	}

	//Clear queues and reset internal 'should_quit' flag
	pl.call_cuckoo_clear_queues();
	pl.call_cuckoo_reset_processing();
}

//tests call_cuckoo_push_to_input_queue
//on all available plugins
#[test]
fn on_commit_call_cuckoo_push_to_input_queue(){
	let iterations = 10;
	let plugins = load_all_plugins();
	for p in plugins.into_iter() {
		for _ in 0..iterations {
			call_cuckoo_push_to_input_queue_tests(&p);
		}
	}
}

// Helper to test call_cuckoo_stop_processing
// basically, when a plugin is told to shut down,
// it should immediately stop its processing,
// clean up all alocated memory, and terminate 
// its processing thread. This will check to ensure each plugin 
// does so, and does so within a reasonable time frame 

fn call_cuckoo_stop_processing_tests(pl: &PluginLibrary){
	println!("Plugin: {}", pl.lib_full_path);

	//start processing, which should take non-trivial time
	//in most cases
	let ret_val=pl.call_cuckoo_start_processing();
	assert!(ret_val==0);

	//push anything to input queue
	let mut hash:[u8;32]=[0;32];
	let nonce:[u8;8]=[0;8];
	//push a few hashes into the queue
	for i in 0..100 {
		hash[0]=i;
		let result=pl.call_cuckoo_push_to_input_queue(i as u32, &hash, &nonce);
		assert!(result==0);
	}

	//Give it a bit to start up and process a bit
	let wait_time = time::Duration::from_millis(2500);
	thread::sleep(wait_time);

	let start=Instant::now();

	//Now stop
	pl.call_cuckoo_stop_processing();

	//wait for internal processing to finish
	while pl.call_cuckoo_has_processing_stopped()==0{};
	pl.call_cuckoo_reset_processing();

	let elapsed=start.elapsed();
	let elapsed_ms=(elapsed.as_secs() * 1_000) + (elapsed.subsec_nanos() / 1_000_000) as u64;
	println!("Shutdown elapsed_ms: {}",elapsed_ms);

	//will give each plugin a few seconds for now
	//but give cuda libs a pass for now, as they're hard to stop
	if !pl.lib_full_path.contains("cuda"){
		//assert!(elapsed_ms<=5000);
	}
}

//tests call_cuckoo_stop_processing
//on all available plugins
#[test]
fn on_commit_call_cuckoo_stop_processing(){
	let iterations = 1;
	let plugins = load_all_plugins();
	for p in plugins.into_iter() {
		for _ in 0..iterations {
			if p.lib_full_path.contains("16"){
				call_cuckoo_stop_processing_tests(&p);
			}
		}
	}

	//let pl = load_plugin_lib("lean_cuda_30").unwrap();
	//call_cuckoo_stop_processing_tests(&pl);
}

// Helper to test call_cuckoo_read_from_output_queue
// will basically test that each plugin comes back
// with a known solution in async mode

fn call_cuckoo_read_from_output_queue_tests(pl: &PluginLibrary){
	println!("Plugin: {}", pl.lib_full_path);

	//Known Hash
	let mut header = from_hex_string(KNOWN_30_HASH_1);
	//or 16, if needed
	if pl.lib_full_path.contains("16") {
		header = from_hex_string(KNOWN_16_HASH_1);
	}
	//Just zero nonce here, for ID
	let nonce:[u8;8]=[0;8];
	let result=pl.call_cuckoo_push_to_input_queue(0, &header, &nonce);
	println!("Result: {}", result);
	assert!(result==0);

	//start processing
	let ret_val=pl.call_cuckoo_start_processing();
	assert!(ret_val==0);
	//Record time now, because we don't want to wait forever
	let start=Instant::now();

	//if 8 minutes has elapsed, there's no solution
	let max_time_ms=480000;

	let mut sols:[u32; 42] = [0; 42];
	let mut nonce: [u8; 8] = [0;8];
	let mut id = 0;
	let mut size = 0;
	loop {
		let found = pl.call_cuckoo_read_from_output_queue(&mut id, &mut sols, &mut size, &mut nonce);
		if found == 1 {
			println!("Found solution");
			break;
		}
		let elapsed=start.elapsed();
		let elapsed_ms=(elapsed.as_secs() * 1_000) + (elapsed.subsec_nanos() / 1_000_000) as u64;
		if elapsed_ms > max_time_ms{
		//stop
			pl.call_cuckoo_stop_processing();

			while pl.call_cuckoo_has_processing_stopped()==0{};
			pl.call_cuckoo_reset_processing();
			//cry about it
			panic!("Known solution not found");
		}
	}
	
	//now stop
	pl.call_cuckoo_stop_processing();

	//wait for internal processing to finish
	while pl.call_cuckoo_has_processing_stopped()==0{};
	pl.call_cuckoo_reset_processing();
	
}

//tests call_cuckoo_read_from_output_queue() on all available
//plugins

#[test]
fn on_commit_call_cuckoo_read_from_output_queue(){
	let iterations = 1;
	let plugins = load_all_plugins();
	for p in plugins.into_iter() {
		for _ in 0..iterations {
			if p.lib_full_path.contains("16"){
				call_cuckoo_read_from_output_queue_tests(&p);
			}
		}
	}
	/*let pl = load_plugin_lib("lean_cuda_30").unwrap();
	call_cuckoo_read_from_output_queue_tests(&pl);*/
}

// Helper to test call_cuckoo_get_stats and return results
// Ensures that all plugins *probably* don't overwrite
// their buffers as they contain an null zero somewhere 
// within the rust-enforced length

fn call_cuckoo_get_stats_test(pl: &PluginLibrary){
	///Test normal value
	const LENGTH:usize = 1024;
	let mut stat_bytes:[u8;LENGTH]=[0;LENGTH];
	let mut stat_bytes_len=stat_bytes.len() as u32;
	let ret_val=pl.call_cuckoo_get_stats(&mut stat_bytes,
		&mut stat_bytes_len);
	let result_list = String::from_utf8(stat_bytes.to_vec()).unwrap();
	let result_list_null_index = result_list.find('\0');
	
	//Check name is less than rust-enforced length,
	//if there's no \0 the plugin is likely overwriting the buffer
	println!("Plugin: {}", pl.lib_full_path);
	assert!(ret_val==0);
	println!("Stat List: **{}**", result_list);
	assert!(result_list.len()>0);
	assert!(result_list_null_index != None);
	println!("Null Index: {}", result_list_null_index.unwrap());

	//Basic form check... json parsing can be checked higher up
	assert!(result_list.contains("["));
	assert!(result_list.contains("]"));

	//Check buffer too small
	const TOO_SMALL:usize = 10;
	let mut stat_bytes:[u8;TOO_SMALL]=[0;TOO_SMALL];
	let mut stat_bytes_len=stat_bytes.len() as u32;
	let ret_val=pl.call_cuckoo_get_stats(&mut stat_bytes,
		&mut stat_bytes_len);
	
	assert!(ret_val==3);

	//Now start up processing and check values
	//Known Hash
	let mut header = from_hex_string(KNOWN_30_HASH_1);
	//or 16, if needed
	if pl.lib_full_path.contains("16") {
		header = from_hex_string(KNOWN_16_HASH_1);
	}
	//Just zero nonce here, for ID
	let nonce:[u8;8]=[0;8];
	let result=pl.call_cuckoo_push_to_input_queue(0, &header, &nonce);
	println!("Result: {}", result);
	assert!(result==0);

	//start processing
	let ret_val=pl.call_cuckoo_start_processing();
	assert!(ret_val==0);

	//Not going to wait around to test values here,
	//will to that higher up as part of other tests
	//in the interests of time

	let wait_time = time::Duration::from_millis(2000);
	thread::sleep(wait_time);

	let mut stat_bytes:[u8;LENGTH]=[0;LENGTH];
	let mut stat_bytes_len=stat_bytes.len() as u32;
	let ret_val=pl.call_cuckoo_get_stats(&mut stat_bytes,
			&mut stat_bytes_len);
	println!("Ret val: {}", ret_val);
	let result_list = String::from_utf8(stat_bytes.to_vec()).unwrap();
	//let result_list_null_index = result_list.find('\0');
	assert!(ret_val==0);
	
	println!("Stats after starting: {}", result_list);

	//now stop
	pl.call_cuckoo_stop_processing();

	//wait for internal processing to finish
	while pl.call_cuckoo_has_processing_stopped()==0{};
	pl.call_cuckoo_reset_processing();
}

//tests call_cuckoo_parameter_list() on all available plugins
#[test]
fn on_commit_call_cuckoo_get_stats(){
	let iterations = 2;
	let plugins = load_all_plugins();
	for p in plugins.into_iter() {
		for _ in 0..iterations {
			if p.lib_full_path.contains("16"){
				call_cuckoo_get_stats_test(&p);
			}
		}
	}
	/*let pl = load_plugin_lib("lean_cpu_30").unwrap();
	call_cuckoo_get_stats_test(&pl);*/
}

// test specific issues in plugins,
// for instance exercising parameters, etc 
// Known to fail hard at moment due to thread cleanup issues in lean_16 
#[test]
fn specific_lean_cpu_16(){
	let pl = load_plugin_lib("lean_cpu_16").unwrap();
	println!("Plugin: {}", pl.lib_full_path);

	let mut header:[u8;32] = [0;32];
	let mut solution:[u32; 42] = [0;42];
	let max_iterations=10000;
	let return_value=pl.call_cuckoo_set_parameter(String::from("NUM_THREADS").as_bytes(), 0, 4);
	assert!(return_value==0);

	//check specific header on 4 threads
	let known_header = from_hex_string(KNOWN_16_HASH_1);
	let mut size = 0;
	let return_value=pl.call_cuckoo(&known_header, &mut size, &mut solution);
	assert!(return_value==1);

	for i in 0..max_iterations {
		for j in 0..32 {
			header[j]=rand::random::<u8>();
		}
		let _=pl.call_cuckoo(&header, &mut size, &mut solution);
		if i%100 == 0{ 
			println!("Iterations: {}", i);
		}
	}
	let return_value=pl.call_cuckoo(&known_header, &mut size, &mut solution);
	assert!(return_value==1);
}


// test specific issues in plugins,
// for instance exercising parameters, etc 
#[test]
fn on_commit_specific_mean_cpu_16(){
	let pl = load_plugin_lib("mean_cpu_16").unwrap();
	println!("Plugin: {}", pl.lib_full_path);

	let mut header:[u8;32] = [0;32];
	let mut solution:[u32; 42] = [0;42];
	let max_iterations=10000;
	let return_value=pl.call_cuckoo_set_parameter(String::from("NUM_THREADS").as_bytes(), 0, 4);
	let mut size = 0;
	assert!(return_value==0);
	for i in 0..max_iterations {
		for j in 0..32 {
			header[j]=rand::random::<u8>();
		}
		let _=pl.call_cuckoo(&header, &mut size, &mut solution);
		if i%100 == 0{ 
			println!("Iterations: {}", i);
		}
	}
	//check specific header on 4 threads
	let known_header = from_hex_string(KNOWN_16_HASH_1);
	let return_value=pl.call_cuckoo(&known_header, &mut size, &mut solution);
	assert!(return_value==1);
} */
