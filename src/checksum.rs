// SPDX-License-Identifier: MIT
// Copyright (c) Nick Moss.

use crc32c::crc32c;
use failure::{ensure,Fallible};

pub const CHECKSUM_LENGTH: usize = 0x4;

pub fn check_checksum(mut data: Vec<u8>, offset: usize, expected: u32, type_name: &str) -> Fallible<()>
{
	data.splice(offset..(offset + CHECKSUM_LENGTH), [0 as u8;CHECKSUM_LENGTH]);
	let check = crc32c(&data);

	ensure!(expected == check, format!("{} signature is invalid.", type_name));
	return Ok(());
}