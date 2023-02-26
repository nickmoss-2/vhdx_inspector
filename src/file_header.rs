// SPDX-License-Identifier: MIT
// Copyright (c) Nick Moss.

use std::{
	fs::File,
	io::{Seek, SeekFrom},
};

use failure::{ensure,Fallible};

use crate::reader::ReadValue;

const FILE_HEADER_OFFSET: usize = 0x0;
const FILE_HEADER_SIG: [u8; FILE_HEADER_SIG_LEN] = [0x76, 0x68, 0x64, 0x78, 0x66, 0x69, 0x6c, 0x65];
const FILE_HEADER_SIG_LEN: usize = 0x8;
const FILE_HEADER_CREATOR_LEN: usize = 0x200;

#[derive(PartialEq)]
pub struct Header
{
	pub creator: String
}

fn check_file_header_valid(signature: &[u8]) -> Fallible<()>
{
	ensure!(signature == FILE_HEADER_SIG, "File signature is invalid.");
	return Ok(());
}

pub fn read_file_header(data: &mut File) -> Fallible<Header>
{
	data.seek(SeekFrom::Start(FILE_HEADER_OFFSET as u64))?;

	let mut signature:Vec<u8> = vec![0;FILE_HEADER_SIG_LEN];
	signature.read_value(data)?;
	check_file_header_valid(&signature)?;

	let mut creator = String::with_capacity(FILE_HEADER_CREATOR_LEN / 2);
	creator.read_value(data)?;

	return Ok(Header{creator: creator});
}