// SPDX-License-Identifier: MIT
// Copyright (c) Nick Moss.

use std::{
	fs::File,
	io::{Seek, Read, SeekFrom},
};

use failure::{ensure,Fallible};
use uuid::Uuid;

use crate::checksum::*;
use crate::reader::{read_into,ReadValue};

const FIRST_HEADER_OFFSET: usize = 0x10000;
const SECOND_HEADER_OFFSET: usize = 0x20000;
const VHD_HEADER_LEN: usize = 0x1000;
const VHD_HEADER_SIG: [u8; VHD_HEADER_SIG_LEN] = [0x68, 0x65, 0x61, 0x64];
const VHD_HEADER_SIG_LEN: usize = 0x4;
const VHD_HEADER_CHECKSUM_LEN: usize = CHECKSUM_LENGTH;

#[derive(PartialEq, Default)]
pub struct VhdHeader
{
	pub checksum: u32,
	pub sequence_number: u64,
	pub file_write_id: Uuid,
	pub data_write_id: Uuid,
	pub log_id: Uuid,
	pub log_version: u16,
	pub version: u16,
	pub log_length: u32,
	pub log_offset: u64
}

impl VhdHeader
{
	pub fn new(data: &mut (impl Read + Seek)) -> Self
	{
		let mut result = VhdHeader::default();
		
		result.checksum.read_value(data).unwrap_or_else(|error| {
			panic!("Failed to read VHDX Header checksum value: {:?}", error)});
		result.sequence_number.read_value(data).unwrap_or_else(|error| {
			panic!("Failed to read VHDX Header sequence number value: {:?}", error)});
		result.file_write_id.read_value(data).unwrap_or_else(|error| {
			panic!("Failed to read VHDX Header file write id value: {:?}", error)});
		result.data_write_id.read_value(data).unwrap_or_else(|error| {
			panic!("Failed to read VHDX Header data write id value: {:?}", error)});
		result.log_id.read_value(data).unwrap_or_else(|error| {
			panic!("Failed to read VHDX Header log id value: {:?}", error)});
		result.log_version.read_value(data).unwrap_or_else(|error| {
			panic!("Failed to read VHDX Header log version value: {:?}", error)});
		result.version.read_value(data).unwrap_or_else(|error| {
			panic!("Failed to read VHDX Header version value: {:?}", error)});
		result.log_length.read_value(data).unwrap_or_else(|error| {
			panic!("Failed to read VHDX Header log length value: {:?}", error)});
		result.log_offset.read_value(data).unwrap_or_else(|error| {
			panic!("Failed to read VHDX Header log offset value: {:?}", error)});
		
		return result;
	}
}

fn check_vhdx_header_valid(data: &mut (impl Read + Seek), header_offset: usize, checksum: u32, signature: &[u8]) -> Fallible<()>
{
	ensure!(signature == VHD_HEADER_SIG, "VHDX header signature is invalid.");

	let mut header_buf: Vec<u8> = vec![0;VHD_HEADER_LEN];
	read_into(data, header_offset, &mut header_buf)?;
	header_buf[VHD_HEADER_SIG_LEN..(VHD_HEADER_SIG_LEN + VHD_HEADER_CHECKSUM_LEN)].as_mut().fill(0);
	
	check_checksum(header_buf, VHD_HEADER_SIG_LEN, checksum, "VHDX header")?;
	return Ok(());
}

fn read_specific_vhdx_header(data: &mut File, header_offset: usize) -> Fallible<VhdHeader>
{
	data.seek(SeekFrom::Start(header_offset as u64))?;

	let mut sig:Vec<u8> = vec![0;VHD_HEADER_SIG_LEN];
	sig.read_value(data)?;
	
	let mut check_checksum:u32 = 0;
	check_checksum.read_value(data)?;
	check_vhdx_header_valid(data, header_offset, check_checksum, &sig)?;

	data.seek(SeekFrom::Start((header_offset + VHD_HEADER_SIG_LEN) as u64))?;

	return Ok(VhdHeader::new(data));
}

pub fn read_vhdx_header(data: &mut File) -> Fallible<(usize, VhdHeader)>
{
	let header1 = read_specific_vhdx_header(data, FIRST_HEADER_OFFSET)?;
	let header2 = read_specific_vhdx_header(data, SECOND_HEADER_OFFSET)?;

	ensure!(header1.sequence_number != header2.sequence_number, "Header sequence numbers are identical.");

	if header1.sequence_number > header2.sequence_number
	{
		return Ok((FIRST_HEADER_OFFSET, header1));
	}
	else
	{
		return Ok((SECOND_HEADER_OFFSET, header2));
	}
}