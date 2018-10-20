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

//! Cuckoo-miner's manager manages the loading, unloading, and querying
//! of mining plugins installed on the system. It is meant as a helper
//! to users of cuckoo-miner, to allow quick enumeration of all mining plugins,
//! and return information about whether a particular plugin can be run
//! on the host system.
//!
//! Although a plugin can only return its name and description at the moment,
//! it will be extended in future to allow for other information such as
//! version, and whether it can be run on a particular system.
//!

#![deny(non_upper_case_globals)]
#![deny(non_camel_case_types)]
#![deny(non_snake_case)]
#![deny(unused_mut)]
#![warn(missing_docs)]

extern crate regex;
extern crate glob;

pub mod manager;
