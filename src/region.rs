// SPDX-License-Identifier: MIT
// Copyright (c) Nick Moss.

use std::{
	fs::File,
	io::{Seek, Read, SeekFrom},
};

use failure::{ensure,Fallible};
use uuid::{Uuid,uuid,};

use crate::checksum::*;
use crate::reader::{read_into,ReadValue,ReadValueOtherTyped};

const REGION_TAB_LEN: usize = 0x10000;
const FIRST_REGION_TAB_OFFSET: usize = 0x30000;
const SECOND_REGION_TAB_OFFSET: usize = 0x40000;
const REGION_TAB_HEADER_LEN: usize = 0x10;
const REGION_TAB_HEADER_SIG: [u8; REGION_TAB_HEADER_SIG_LEN] = [0x72, 0x65, 0x67, 0x69];
const REGION_TAB_HEADER_SIG_LEN: usize = 0x4;
const REGION_TAB_HEADER_CHECKSUM_LEN: usize = CHECKSUM_LENGTH;

const REGION_TAB_ENTRY_LEN: usize = 0x20;

const MAX_REGION_ENTRIES: u32 = 2047;
const MIN_REGION_OFFSET: u64 = u64::pow(1024, 2);
const REGION_OFFSET_FACTOR: u64 = u64::pow(1024, 2);
const REGION_SIZE_FACTOR: u32 = u32::pow(1024, 2);
const REGION_BAT: Uuid = uuid!("2DC27766-F623-4200-9D64-115E9BFD4A08");
const REGION_METADATA: Uuid = uuid!("8B7CA206-4790-4B9A-B8FE-575F050F886E");

#[derive(PartialEq, Default)]
pub enum RegionType
{
	#[default]
	Unknown,
	BAT,
	Metadata,
}

#[derive(PartialEq, Default)]
pub struct RegionTableEntry
{
	pub region_type: RegionType,
	pub object_id: Uuid,
	pub object_offset: u64,
	pub object_length: u32,
	pub required: bool
}

impl RegionTableEntry
{
	pub fn new(data: &mut (impl Read + Seek)) -> Self
	{
		let mut result = RegionTableEntry::default();
		
		result.object_id.read_value(data).unwrap_or_else(|error| {
			panic!("Failed to read Region entry object ID Uuid: {:?}", error)});
		result.object_offset.read_value(data).unwrap_or_else(|error| {
			panic!("Failed to read Region entry object offset u64: {:?}", error)});
		result.object_length.read_value(data).unwrap_or_else(|error| {
			panic!("Failed to read Region entry file object length u32: {:?}", error)});
		result.required.read_value::<u32>(data).unwrap_or_else(|error| {
			panic!("Failed to read Region entry data required bool: {:?}", error)});
		
		return result;
	}
}

#[derive(PartialEq, Default)]
pub struct RegionTable
{
	pub checksum: u32,
	pub entry_count: u32,
	pub entries: Vec<RegionTableEntry>
}

impl RegionTable
{
	pub fn new(data: &mut (impl Read + Seek)) -> Self
	{
		let mut result = RegionTable::default();
		
		result.checksum.read_value(data).unwrap_or_else(|error| {
			panic!("Failed to read Region Header checksum u32: {:?}", error)});
		result.entry_count.read_value(data).unwrap_or_else(|error| {
			panic!("Failed to read Region Header sequence number u32: {:?}", error)});
		result.entries.reserve(result.entry_count as usize);
		
		return result;
	}

	pub fn add_entry(self: &mut Self, entry: RegionTableEntry) -> ()
	{
		self.entries.push(entry);
	}
}

fn check_region_entry_valid(entry: &RegionTableEntry) -> Fallible<()>
{
	ensure!(entry.object_offset >= MIN_REGION_OFFSET,
		format!("Region object offset is smaller than the specified minimum {}.", MIN_REGION_OFFSET));
	ensure!(entry.object_offset % REGION_OFFSET_FACTOR == 0,
		format!("Region object offset is not a multiple of the specified {}.", REGION_OFFSET_FACTOR));
	ensure!(entry.object_length % REGION_SIZE_FACTOR == 0,
		format!("Region object size is not a multiple of the specified {}.", REGION_SIZE_FACTOR));

	ensure!(entry.region_type != RegionType::Unknown || !entry.required,
		format!("Required object ID {} is not recognised by this version of this program.", entry.object_id));
	
	return Ok(());
}

fn read_region_entry(data: &mut File, entry_offset: usize) -> Fallible<RegionTableEntry>
{
	data.seek(SeekFrom::Start(entry_offset as u64))?;
	let mut entry = RegionTableEntry::new(data);
	match entry.object_id
	{
		REGION_BAT => {entry.region_type = RegionType::BAT}
		REGION_METADATA => {entry.region_type = RegionType::Metadata}
		_ => {entry.region_type = RegionType::Unknown}
	}

	check_region_entry_valid(&entry)?;

	return Ok(entry);
}

fn check_region_header_valid(data: &mut (impl Read + Seek), header_offset: usize, signature: &[u8], table: &RegionTable) -> Fallible<()>
{
	ensure!(signature == REGION_TAB_HEADER_SIG, "Region header signature is invalid.");

	let mut header_buf: Vec<u8> = vec![0;REGION_TAB_LEN];
	read_into(data, header_offset, &mut header_buf)?;
	header_buf[REGION_TAB_HEADER_SIG_LEN..(REGION_TAB_HEADER_SIG_LEN + REGION_TAB_HEADER_CHECKSUM_LEN)].as_mut().fill(0);
	
	check_checksum(header_buf, REGION_TAB_HEADER_SIG_LEN, table.checksum, "VHDX header")?;
	ensure!(table.entry_count < MAX_REGION_ENTRIES,
		format!("Region table entry count exceeds the specified maximum {}.", MAX_REGION_ENTRIES));
	
	return Ok(());
}

fn read_specific_region(data: &mut File, table_offset: usize) -> Fallible<RegionTable>
{
	data.seek(SeekFrom::Start(table_offset as u64))?;

	let mut signature:Vec<u8> = vec![0;REGION_TAB_HEADER_SIG_LEN];
	signature.read_value(data)?;

	let mut table = RegionTable::new(data);
	
	check_region_header_valid(data, table_offset, &signature, &table)?;

	for n in 0..table.entry_count as usize
	{
		table.add_entry(read_region_entry(data, table_offset + REGION_TAB_HEADER_LEN + (n * REGION_TAB_ENTRY_LEN))?);
	}

	return Ok(table);
}

pub fn read_region(data: &mut File) -> Fallible<RegionTable>
{
	let region1 = read_specific_region(data, FIRST_REGION_TAB_OFFSET)?;
	let region2 = read_specific_region(data, SECOND_REGION_TAB_OFFSET)?;

	ensure!(region1 == region2, "Regions do not match.");

	return Ok(region1);
}