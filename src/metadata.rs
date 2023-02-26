// SPDX-License-Identifier: MIT
// Copyright (c) Nick Moss.

use std::{
	fs::File,
	io::{Seek, Read, SeekFrom},
};

use failure::{ensure,Fallible};
use uuid::{Uuid,uuid,};

use crate::region::{RegionType,RegionTableEntry,};
use crate::reader::ReadValue;

const METADATA_HEADER_LEN: usize = 0x20;
const METADATA_HEADER_SIG: [u8; METADATA_HEADER_SIG_LEN] = [0x6d, 0x65, 0x74, 0x61, 0x64, 0x61, 0x74, 0x61];
const METADATA_HEADER_SIG_LEN: usize = 0x8;
const METADATA_HEADER_RESERVED_1_LEN: usize = 0x2;

const METADATA_ENTRY_LEN: usize = 0x20;

const METADATA_PARENT_LOCATOR_HEADER_LEN: usize = 0x14;
const METADATA_PARENT_LOCATOR_ENTRY_LEN: usize = 0xc;

const METADATA_FILE_PARAMETERS: Uuid = uuid!("CAA16737-FA36-4D43-B3B6-33F0AA44E76B");
const METADATA_VIRTUAL_DISK_SIZE: Uuid = uuid!("2FA54224-CD1B-4876-B211-5DBED83BF4B8");
const METADATA_VIRTUAL_DISK_ID: Uuid = uuid!("BECA12AB-B2E6-4523-93EF-C309E000C746");
const METADATA_LOGICAL_SECTOR_SIZE: Uuid = uuid!("8141BF1D-A96F-4709-BA47-F233A8FAAB5F");
const METADATA_PHYSICAL_SECTOR_SIZE: Uuid = uuid!("CDA348C7-445D-4471-9CC9-E9885251C556");
const METADATA_PARENT_LOCATOR: Uuid = uuid!("A8D35F2D-B30B-454D-ABF7-D3D84834AB0C");
const METADATA_ENTRY_IS_USER_FLAG:u32 = 0b00000001;
const METADATA_ENTRY_IS_VIRTUAL_DISK_FLAG:u32 = 0b00000010;
const METADATA_ENTRY_IS_REQUIRED_FLAG:u32 = 0b00000100;

const METADATA_LEAVE_ALLOCATED_FLAG:u32 = 0b00000001;
const METADATA_HAS_PARENT_FLAG:u32 = 0b00000010;
const METADATA_PARENT_LOCATOR_HEADER_RESERVED_1_LEN: usize = 0x2;
const METADATA_PARENT_LOCATOR_VHDX: Uuid = uuid!("B04AEFB7-D19E-4A81-B789-25B8E9445913");

const PARENT_LOCATOR_LINKAGE1_KEY: &str = "parent_linkage";
const PARENT_LOCATOR_LINKAGE2_KEY: &str = "parent_linkage2";
const PARENT_LOCATOR_RELATIVE_PATH_KEY: &str = "relative_path";
const PARENT_LOCATOR_VOLUME_PATH_KEY: &str = "volume_path";
const PARENT_LOCATOR_ABSOLUTE_PATH_KEY: &str = "absolute_win32_path";

#[derive(PartialEq, Default)]
pub enum MetadataType
{
	#[default]
	Unknown,
	FileParameters,
	VirtualDiskSize,
	VirtualDiskId,
	LogicalSectorSize,
	PhysicalSectorSize,
	ParentLocator,
}

#[derive(PartialEq, Default)]
pub struct MetadataTableEntry
{
	pub metadata_type: MetadataType,
	pub object_id: Uuid,
	pub object_offset: u32,
	pub object_length: u32,
	pub is_user: bool,
	pub is_virtual_disk: bool,
	pub is_required: bool,
}

impl MetadataTableEntry
{
	pub fn new(data: &mut (impl Read + Seek)) -> Self
	{
		let mut result = MetadataTableEntry::default();
		
		result.object_id.read_value(data).unwrap_or_else(|error| {
			panic!("Failed to read metadata table entry object ID Uuid: {:?}", error)});
		result.object_offset.read_value(data).unwrap_or_else(|error| {
			panic!("Failed to read metadata table entry object offset u64: {:?}", error)});
		result.object_length.read_value(data).unwrap_or_else(|error| {
			panic!("Failed to read metadata table entry object length u32: {:?}", error)});

		let mut flags:u32 = 0;
		flags.read_value(data).unwrap_or_else(|error| {
			panic!("Failed to read file parameter flags u32: {:?}", error)});

		result.is_user = flags & METADATA_ENTRY_IS_USER_FLAG != 0;
		result.is_virtual_disk = flags & METADATA_ENTRY_IS_VIRTUAL_DISK_FLAG != 0;
		result.is_required = flags & METADATA_ENTRY_IS_REQUIRED_FLAG != 0;
		
		return result;
	}
}

#[derive(PartialEq, Default)]
pub struct MetadataTable
{
	pub entry_count: u16,
	pub entries: Vec<MetadataTableEntry>,
}

impl MetadataTable
{
	pub fn new(data: &mut (impl Read + Seek)) -> Self
	{
		let mut result = MetadataTable::default();
		
		result.entry_count.read_value(data).unwrap_or_else(|error| {
			panic!("Failed to read metadata table entry count u32: {:?}", error)});
		result.entries.reserve(result.entry_count as usize);
		
		return result;
	}

	pub fn add_entry(self: &mut Self, entry: MetadataTableEntry) -> ()
	{
		self.entries.push(entry);
	}
}

#[derive(PartialEq, Default)]
pub struct FileParameters
{
	pub block_size: u32,
	pub leave_block_allocated: bool,
	pub has_parent: bool,
}

impl FileParameters
{
	pub fn new(data: &mut (impl Read + Seek)) -> Self
	{
		let mut result = FileParameters::default();
		
		result.block_size.read_value(data).unwrap_or_else(|error| {
			panic!("Failed to read file parameter block size u32: {:?}", error)});

		let mut flags:u32 = 0;
		flags.read_value(data).unwrap_or_else(|error| {
			panic!("Failed to read file parameter flags u32: {:?}", error)});

		result.leave_block_allocated = flags & METADATA_LEAVE_ALLOCATED_FLAG != 0;
		result.has_parent = flags & METADATA_HAS_PARENT_FLAG != 0;
		
		return result;
	}
}

#[derive(PartialEq, Default)]
pub struct ParentLocatorEntry
{
	pub key_offset: u32,
	pub value_offset: u32,
	pub key_length: u16,
	pub value_length: u16,
	pub key: String,
	pub value: String,
}

impl ParentLocatorEntry
{
	pub fn new(data: &mut (impl Read + Seek), table_offset: usize) -> Self
	{
		let mut result = ParentLocatorEntry::default();
		
		result.key_offset.read_value(data).unwrap_or_else(|error| {
			panic!("Failed to read parent locator entry key offset u32: {:?}", error)});
		result.value_offset.read_value(data).unwrap_or_else(|error| {
			panic!("Failed to read parent locator entry value offset u32: {:?}", error)});
		result.key_length.read_value(data).unwrap_or_else(|error| {
			panic!("Failed to read parent locator entry key length u16: {:?}", error)});
		result.value_length.read_value(data).unwrap_or_else(|error| {
			panic!("Failed to read parent locator entry value length u16: {:?}", error)});
			
		result.key = String::with_capacity((result.key_length / 2) as usize);
		result.key.read_value_off(data, table_offset + result.key_offset as usize).unwrap_or_else(|error| {
			panic!("Failed to read parent locator entry key String: {:?}", error)});
		result.value = String::with_capacity((result.value_length / 2) as usize);
		result.value.read_value_off(data, table_offset + result.value_offset as usize).unwrap_or_else(|error| {
			panic!("Failed to read parent locator entry value String: {:?}", error)});
		
		return result;
	}
}

#[derive(PartialEq, Default, Clone)]
pub enum ParentLocatorType
{
	#[default]
	Unknown,
	Vhdx,
}

#[derive(PartialEq, Default)]
pub struct ParentLocatorDict
{
	pub locator_type: ParentLocatorType,
	pub locator_type_id: Uuid,
	pub key_value_count: u16,
	pub entries: Vec<ParentLocatorEntry>,
}

impl ParentLocatorDict
{
	pub fn new(data: &mut (impl Read + Seek)) -> Self
	{
		let mut result = ParentLocatorDict::default();
		
		result.locator_type_id.read_value(data).unwrap_or_else(|error| {
			panic!("Failed to read parent locator type Uuid: {:?}", error)});
		//Skip an internal reserved value...
		data.seek(SeekFrom::Current(METADATA_PARENT_LOCATOR_HEADER_RESERVED_1_LEN as i64)).unwrap_or_else(|error| {
			panic!("Failed to skip reserved region of size 0x{:X}: {:?}", METADATA_PARENT_LOCATOR_HEADER_RESERVED_1_LEN, error)});
		result.key_value_count.read_value(data).unwrap_or_else(|error| {
			panic!("Failed to read parent locator key/value count u16: {:?}", error)});
		result.entries.reserve(result.key_value_count as usize);
		
		return result;
	}

	pub fn add_entry(self: &mut Self, entry: ParentLocatorEntry) -> ()
	{
		self.entries.push(entry);
	}
}

#[derive(PartialEq, Default, Clone)]
pub struct ParentLocator
{
	pub locator_type: ParentLocatorType,
	pub parent_linkage: Uuid,
	pub parent_linkage2: Uuid,
	pub relative_path: String,
	pub volume_path: String,
	pub absolute_win32_path: String,
}

#[derive(PartialEq, Default)]
pub struct Metadata
{
	pub file_parameters: FileParameters,
	pub virtual_disk_size: usize,
	pub virtual_disk_id: Uuid,
	pub logical_sector_size: u32,
	pub physical_sector_size: u32,
	pub parent_locator_dict: Option<ParentLocatorDict>,
	pub parent_locator: Option<ParentLocator>,
}

fn read_file_parameters(data: &mut File, item_data: &MetadataTableEntry, table_offset: usize) -> Fallible<FileParameters>
{
	data.seek(SeekFrom::Start((table_offset + item_data.object_offset as usize) as u64))?;
	return Ok(FileParameters::new(data));
}

fn read_virtual_disk_size(data: &mut File, item_data: &MetadataTableEntry, table_offset: usize) -> Fallible<usize>
{
	data.seek(SeekFrom::Start((table_offset + item_data.object_offset as usize) as u64))?;
	let mut result: usize = 0;
	result.read_value(data)?;
	return Ok(result);
}

fn read_virtual_disk_id(data: &mut File, item_data: &MetadataTableEntry, table_offset: usize) -> Fallible<Uuid>
{
	data.seek(SeekFrom::Start((table_offset + item_data.object_offset as usize) as u64))?;
	let mut result: Uuid = Uuid::default();
	result.read_value(data)?;
	return Ok(result);
}

fn read_logical_sector_size(data: &mut File, item_data: &MetadataTableEntry, table_offset: usize) -> Fallible<u32>
{
	data.seek(SeekFrom::Start((table_offset + item_data.object_offset as usize) as u64))?;
	let mut result: u32 = 0;
	result.read_value(data)?;
	return Ok(result);
}

fn read_physical_sector_size(data: &mut File, item_data: &MetadataTableEntry, table_offset: usize) -> Fallible<u32>
{
	data.seek(SeekFrom::Start((table_offset + item_data.object_offset as usize) as u64))?;
	let mut result: u32 = 0;
	result.read_value(data)?;
	return Ok(result);
}

fn check_parent_locator_entry_valid(entry: &ParentLocatorEntry) -> Fallible<()>
{
	ensure!(!entry.key.contains('\0'), "Parent locator value {} contains a null.");
	ensure!(!entry.value.contains('\0'), "Parent locator value {} contains a null.");
	
	return Ok(());
}

fn read_parent_locator_entry(data: &mut File, item_offset: usize, table_offset: usize) -> Fallible<ParentLocatorEntry>
{
	data.seek(SeekFrom::Start(item_offset as u64))?;
	let entry = ParentLocatorEntry::new(data, table_offset);

	check_parent_locator_entry_valid(&entry)?;

	return Ok(entry);
}

fn read_parent_locator(data: &mut File, item_data: &MetadataTableEntry, table_offset: usize) -> Fallible<(Option<ParentLocatorDict>, Option<ParentLocator>)>
{
	data.seek(SeekFrom::Start((table_offset + item_data.object_offset as usize) as u64))?;

	let mut table = ParentLocatorDict::new(data);
	let mut locator = ParentLocator::default();
	table.locator_type = match table.locator_type_id
	{
		METADATA_PARENT_LOCATOR_VHDX => {ParentLocatorType::Vhdx}
		_ => {ParentLocatorType::Unknown}
	};

	for n in 0..table.key_value_count as usize
	{
		let item_offset = table_offset + item_data.object_offset as usize + METADATA_PARENT_LOCATOR_HEADER_LEN + (n * METADATA_PARENT_LOCATOR_ENTRY_LEN);
		let entry = read_parent_locator_entry(data, item_offset, table_offset + item_data.object_offset as usize)?;

		match entry.key.as_str()
		{
			PARENT_LOCATOR_LINKAGE1_KEY => locator.parent_linkage = Uuid::parse_str(&entry.value)?,
			PARENT_LOCATOR_LINKAGE2_KEY => locator.parent_linkage2 = Uuid::parse_str(&entry.value)?,
			PARENT_LOCATOR_RELATIVE_PATH_KEY => locator.relative_path = entry.value.clone(),
			PARENT_LOCATOR_VOLUME_PATH_KEY => locator.volume_path = entry.value.clone(),
			PARENT_LOCATOR_ABSOLUTE_PATH_KEY => locator.absolute_win32_path = entry.value.clone(),
			&_ => ensure!(false, "Unknown parent locator key '{}'.", entry.key),
		}

		table.add_entry(entry);
	}
	locator.locator_type = table.locator_type.clone();

	return Ok((Some(table), Some(locator)));
}

fn check_metadata_table_entry_valid(entry: &MetadataTableEntry) -> Fallible<()>
{
	ensure!(entry.metadata_type != MetadataType::Unknown || !entry.is_required, "Metadata header signature is invalid.");
	
	return Ok(());
}

fn read_metadata_entry(data: &mut File, table_offset: usize) -> Fallible<MetadataTableEntry>
{
	data.seek(SeekFrom::Start(table_offset as u64))?;
	let mut entry = MetadataTableEntry::new(data);
	entry.metadata_type = match entry.object_id
	{
		METADATA_FILE_PARAMETERS => MetadataType::FileParameters,
		METADATA_VIRTUAL_DISK_SIZE => MetadataType::VirtualDiskSize,
		METADATA_VIRTUAL_DISK_ID => MetadataType::VirtualDiskId,
		METADATA_LOGICAL_SECTOR_SIZE => MetadataType::LogicalSectorSize,
		METADATA_PHYSICAL_SECTOR_SIZE => MetadataType::PhysicalSectorSize,
		METADATA_PARENT_LOCATOR => MetadataType::ParentLocator,
		_ => MetadataType::Unknown,
	};

	check_metadata_table_entry_valid(&entry)?;

	return Ok(entry);
}

fn check_metadata_table_header_valid(signature: &[u8]) -> Fallible<()>
{
	ensure!(signature == METADATA_HEADER_SIG, "Metadata header signature is invalid.");
	
	return Ok(());
}

fn read_metadata_table(data: &mut File, table_offset: usize, table_length: usize) -> Fallible<MetadataTable>
{
	data.seek(SeekFrom::Start(table_offset as u64))?;

	let mut signature:Vec<u8> = vec![0;METADATA_HEADER_SIG_LEN];
	signature.read_value(data)?;

	//Skip an internal reserved value...
	data.seek(SeekFrom::Current(METADATA_HEADER_RESERVED_1_LEN as i64))?;

	let mut table = MetadataTable::new(data);
	
	check_metadata_table_header_valid(&signature)?;

	for n in 0..table.entry_count as usize
	{
		ensure!(n * METADATA_ENTRY_LEN <= table_length, "Metadata table is longer than recorded in the region table ({} bytes).", table_length);
		table.add_entry(read_metadata_entry(data, table_offset + METADATA_HEADER_LEN + (n * METADATA_ENTRY_LEN))?);
	}

	return Ok(table);
}

fn read_metadata_values(data: &mut File, table: &MetadataTable, table_offset: usize, table_length: usize) -> Fallible<Metadata>
{
	data.seek(SeekFrom::Start(table_offset as u64))?;
	let mut metadata = Metadata::default();

	for item_data in &table.entries
	{
		match item_data.metadata_type
		{
			MetadataType::FileParameters => { metadata.file_parameters = read_file_parameters(data, item_data, table_offset)? }
			MetadataType::VirtualDiskSize => { metadata.virtual_disk_size = read_virtual_disk_size(data, item_data, table_offset)? }
			MetadataType::VirtualDiskId => { metadata.virtual_disk_id = read_virtual_disk_id(data, item_data, table_offset)? }
			MetadataType::LogicalSectorSize => { metadata.logical_sector_size = read_logical_sector_size(data, item_data, table_offset)? }
			MetadataType::PhysicalSectorSize => { metadata.physical_sector_size = read_physical_sector_size(data, item_data, table_offset)? }
			MetadataType::ParentLocator => { (metadata.parent_locator_dict,metadata.parent_locator) = read_parent_locator(data, item_data, table_offset)? }
			MetadataType::Unknown => { ensure!(false, "Unknown metadata type {} encountered.", item_data.object_id); }
		}
	}

	ensure!(data.stream_position()? <= (table_offset + table_length) as u64,
		"Metadata table is longer than recorded in the region table ({} bytes).", table_length);

	return Ok(metadata);
}

fn check_metadata_valid(metadata: &Metadata) -> Fallible<()>
{
	ensure!(!metadata.file_parameters.has_parent || metadata.parent_locator.is_some(),
		"File parameter 'HasParent' is set and the file does not contain a parent locator.");
	
	return Ok(());
}

pub fn read_metadata(data: &mut File, region_data: &RegionTableEntry) -> Fallible<(MetadataTable, Metadata)>
{
	ensure!(region_data.region_type == RegionType::Metadata, "Passed region data is not for the Metadata region.");

	let table = read_metadata_table(data, region_data.object_offset as usize, region_data.object_length as usize)?;
	let metadata = read_metadata_values(data, &table, region_data.object_offset as usize, region_data.object_length as usize)?;

	check_metadata_valid(&metadata)?;

	return Ok((table, metadata));
}