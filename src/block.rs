// SPDX-License-Identifier: MIT
// Copyright (c) Nick Moss.

use std::{
	fmt,
	fs::File,
	io::{Seek, Read, SeekFrom},
};

use failure::{ensure,Fallible};

use crate::maths::*;
use crate::metadata::Metadata;
use crate::region::{RegionTableEntry,RegionType};
use crate::reader::ReadValue;

const CHUNK_RATIO_MULTIPLIER: u64 = 2_u32.pow(23) as u64;

const BAT_ENTRY_LEN: usize = 0x20;
const BAT_ENTRY_STATE_MASK: u64 = 0b0000000000000000000000000000000000000000000000000000000000000111;
const BAT_ENTRY_OFFSET_MASK: u64 = 0b1111111111111111111111111111111111111111111100000000000000000000;

#[derive(PartialEq, Default)]
pub enum PayloadBlockState
{
	#[default]
	NotPresent = 0,
	Undefined = 1,
	Zero = 2,
	Unmapped = 3,
	FullyPresent = 6,
	PartiallyPresent = 7,
}

impl TryFrom<u64> for PayloadBlockState
{
	type Error = ();

	fn try_from(value: u64) -> Result<Self, Self::Error>
	{
		match value {
			x if x == PayloadBlockState::NotPresent as u64 => Ok(PayloadBlockState::NotPresent),
			x if x == PayloadBlockState::Undefined as u64 => Ok(PayloadBlockState::Undefined),
			x if x == PayloadBlockState::Zero as u64 => Ok(PayloadBlockState::Zero),
			x if x == PayloadBlockState::Unmapped as u64 => Ok(PayloadBlockState::Unmapped),
			x if x == PayloadBlockState::FullyPresent as u64 => Ok(PayloadBlockState::FullyPresent),
			x if x == PayloadBlockState::PartiallyPresent as u64 => Ok(PayloadBlockState::PartiallyPresent),
			_ => Err(()),
		}
	}
}

impl fmt::Display for PayloadBlockState {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			PayloadBlockState::NotPresent => write!(f, "not present"),
			PayloadBlockState::Undefined => write!(f, "undefined"),
			PayloadBlockState::Zero => write!(f, "zero"),
			PayloadBlockState::Unmapped => write!(f, "unmapped"),
			PayloadBlockState::FullyPresent => write!(f, "fully present"),
			PayloadBlockState::PartiallyPresent => write!(f, "partially present"),
		}
	}
}

#[derive(PartialEq, Default)]
pub enum SectorBlockState
{
	#[default]
	NotPresent = 0,
	Present = 6,
}

impl TryFrom<u64> for SectorBlockState
{
	type Error = ();

	fn try_from(value: u64) -> Result<Self, Self::Error>
	{
		match value {
			x if x == SectorBlockState::NotPresent as u64 => Ok(SectorBlockState::NotPresent),
			x if x == SectorBlockState::Present as u64 => Ok(SectorBlockState::Present),
			_ => Err(()),
		}
	}
}

impl fmt::Display for SectorBlockState {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			SectorBlockState::NotPresent => write!(f, "not present"),
			SectorBlockState::Present => write!(f, "present"),
		}
	}
}

#[derive(PartialEq, Default)]
pub struct PayloadEntry
{
	pub state: PayloadBlockState,
	pub file_offset_mb: u64
}

impl PayloadEntry
{
	pub fn new(data: &mut (impl Read + Seek)) -> Self
	{
		let mut value: u64 = 0;
		value.read_value(data).unwrap_or_else(|error| {
			panic!("Failed to read BAT entry bit field: {:?}", error)});

		let mut result = PayloadEntry::default();
		result.state = PayloadBlockState::try_from(value & BAT_ENTRY_STATE_MASK).unwrap_or_else(|_| {
			panic!("Value {:?} is not a valid PayloadBlockState", value & BAT_ENTRY_STATE_MASK)});
		result.file_offset_mb = (value & BAT_ENTRY_OFFSET_MASK) >> 20;

		return result;
	}
}

#[derive(PartialEq, Default)]
pub struct SectorEntry
{
	pub state: SectorBlockState,
	pub file_offset_mb: u64
}

impl SectorEntry
{
	pub fn new(data: &mut (impl Read + Seek)) -> Self
	{
		let mut value: u64 = 0;
		value.read_value(data).unwrap_or_else(|error| {
			panic!("Failed to read BAT entry bit field: {:?}", error)});

		let mut result = SectorEntry::default();
		result.state = SectorBlockState::try_from(value & BAT_ENTRY_STATE_MASK).unwrap_or_else(|_| {
			panic!("Value {:?} is not a valid SectorBlockState", value & BAT_ENTRY_STATE_MASK)});
		result.file_offset_mb = (value & BAT_ENTRY_OFFSET_MASK) >> 20;

		return result;
	}
}

#[derive(PartialEq, Default)]
pub struct FileBlockValues
{
	pub chunk_ratio: u64,
	pub payload_blocks: u64,
	pub sector_blocks: u64,
	pub total_bat_entries: u64,
}

fn calculate_block_values(file_data: &Metadata) -> Fallible<FileBlockValues>
{
	let chunk_ratio: u64 = (CHUNK_RATIO_MULTIPLIER * file_data.logical_sector_size as u64) / file_data.file_parameters.block_size as u64;
	ensure!(chunk_ratio != 0, "Chunk ratio calculation resulted in 0, cannot calculate BAT.");
	let payload_blocks = u64::ceiling_divide(file_data.virtual_disk_size as u64, file_data.file_parameters.block_size as u64);
	let sector_blocks = u64::ceiling_divide(payload_blocks, chunk_ratio as u64);
	let total_bat_entries;
	if file_data.parent_locator.is_some()
	{
		// Fixed or dynamic disk calculation
		total_bat_entries = payload_blocks + u64::floor_divide(payload_blocks - 1, chunk_ratio as u64);
	}
	else
	{
		// Differencing disk calculation
		total_bat_entries = sector_blocks * (chunk_ratio + 1);
	}

	return Ok(FileBlockValues{chunk_ratio, payload_blocks, sector_blocks, total_bat_entries});
}

fn read_bat_table(data: &mut File, bat_region: &RegionTableEntry, block_values: &FileBlockValues, has_sectors: bool) -> Fallible<(Vec<PayloadEntry>,Vec<SectorEntry>)>
{
	data.seek(SeekFrom::Start(bat_region.object_offset))?;

	let mut payload_blocks = Vec::<PayloadEntry>::new();
	payload_blocks.reserve_exact(block_values.payload_blocks as usize);

	let mut sector_blocks = Vec::<SectorEntry>::new();
	sector_blocks.reserve_exact(block_values.sector_blocks as usize);

	let entry_count = if has_sectors {block_values.total_bat_entries as usize} else {block_values.payload_blocks as usize};

	for n in 0..entry_count
	{
		ensure!(n * BAT_ENTRY_LEN <= bat_region.object_length as usize, "BAT table is longer than recorded in the region table ({} bytes).", bat_region.object_length);
		if has_sectors && n != 0 && (n % (block_values.chunk_ratio + 1) as usize) == 0
		{
			sector_blocks.push(SectorEntry::new(data));
		}
		else
		{
			payload_blocks.push(PayloadEntry::new(data));
		}
	}

	return Ok((payload_blocks, sector_blocks));
}

pub fn read_bat(data: &mut File, bat_region: &RegionTableEntry, file_data: &Metadata, has_sectors: bool) -> Fallible<(Vec<PayloadEntry>,Vec<SectorEntry>)>
{
	ensure!(bat_region.region_type == RegionType::BAT, "Passed region data is not for the BAT region.");

	let block_values = calculate_block_values(file_data)?;
	let bat_entries = read_bat_table(data, bat_region, &block_values, has_sectors)?;
	return Ok(bat_entries);
}