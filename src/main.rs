// SPDX-License-Identifier: MIT
// Copyright (c) Nick Moss.

use std::{
	env,
	fmt,
	fs::File,
	ffi::OsStr,
	path::Path,
};

use failure::{Error,ensure,Fallible,};

use crate::block::PayloadBlockState;
use crate::region::RegionType;
use crate::metadata::{MetadataType,ParentLocatorType,ParentLocator};

mod block;
mod checksum;
mod file_header;
mod maths;
mod metadata;
mod reader;
mod region;
mod vhd_header;

enum VhdType
{
	Fixed,
	Dynamic,
	Differencing,
}

impl fmt::Display for VhdType {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			VhdType::Fixed => write!(f, "Fixed"),
			VhdType::Dynamic => write!(f, "Dynamic"),
			VhdType::Differencing => write!(f, "Differencing"),
		}
	}
}

fn print_help()
{
	println!("Vhdx Inspector");
	println!();
	println!("Retrieves VHDX file data for debugging.");
	println!();
	println!("USAGE:");
	println!("\tvhdx_inspector [args] <file name>");
	println!("\t\tDump VHDX information about the given file.");
	println!("Arguments:");
	println!("\t-h, --help");
	println!("\t\tPrint this help message and exit immediately.");
	println!("\t-f, --follow");
	println!("\t\tIf the VHDX file is a differencing disk, print the parent");
	println!("\t\t\tdisk's information and so on up the chain.");
	println!("\t-b, --blocks");
	println!("\t\tPrint the full block status information.");
}

fn calc_parent_path(locator: &ParentLocator, child_path: &Path) -> Fallible<String>
{
	if !locator.relative_path.is_empty() && 
		child_path.parent().unwrap().join(&locator.relative_path).exists()
	{
		println!("Located parent from relative path '{}'.", &locator.relative_path);
		return Ok(child_path.parent().unwrap().join(locator.relative_path.clone()).canonicalize()?.to_str().unwrap().to_string());
	}
	else if !locator.volume_path.is_empty() &&
		Path::new(OsStr::new(locator.volume_path.as_str())).exists()
	{
		println!("Located parent from volume path '{}'.", &locator.volume_path);
		return Ok(locator.volume_path.clone());
	}
	else if !locator.absolute_win32_path.is_empty() &&
		Path::new(OsStr::new(locator.absolute_win32_path.as_str())).exists()
	{
		println!("Located parent from absolute path '{}'.", &locator.absolute_win32_path);
		return Ok(locator.absolute_win32_path.clone());
	}
	else
	{
		ensure!(false, "Could not find parent with any of the relative path '{}', the volume path '{}' or the absolute path '{}'.",
			locator.relative_path, locator.volume_path, locator.absolute_win32_path);
		return Ok(String::new());
	}
}

fn main() -> Result<(), Error>
{
	let args: Vec<String> = env::args().collect();

	if args.len() == 1
	{
		print_help();
		return Ok(());
	}

	let mut file_path:String = String::from("");
	let mut follow_chain = false;
	let mut print_blocks = false;
	let mut disk_type = VhdType::Fixed;
	let mut parent_locator: Option<ParentLocator> = None;

	for arg in args
	{
		if arg == "-h" || arg == "--help"
		{
			print_help();
			return Ok(());
		}
		else if arg == "-f" || arg == "--follow"
		{
			follow_chain = true;
			continue;
		}
		else if arg == "-b" || arg == "--blocks"
		{
			print_blocks = true;
			continue;
		}
		else if arg.starts_with("-")
		{
			print_help();
			return Ok(());
		}
		else
		{
			file_path = String::from(arg);
			continue;
		}
	}

	loop
	{
		println!("Reading VHDX file {}.", &file_path);

		let mut vhdx_file = File::open(&file_path)?;

		let header = file_header::read_file_header(&mut vhdx_file)?;
		let (vhdx_offset, vhdx_header) = vhd_header::read_vhdx_header(&mut vhdx_file)?;
		let region_table = region::read_region(&mut vhdx_file)?;
		let metadata_region = &region_table.entries.iter().find(|x| x.region_type == RegionType::Metadata).unwrap();
		let bat_region = &region_table.entries.iter().find(|x| x.region_type == RegionType::BAT).unwrap();
		let (metadata_table, metadata) = metadata::read_metadata(&mut vhdx_file, metadata_region)?;
		let (payload_blocks,sector_blocks) = block::read_bat(&mut vhdx_file, bat_region, &metadata, parent_locator.is_some())?;

		if parent_locator.is_some()
		{
			disk_type = VhdType::Differencing;
			let parent = parent_locator.unwrap();
			parent_locator = None;
			
			if parent.parent_linkage == vhdx_header.data_write_id
			{
				println!("Parent linkage Data Write GUID {} identified by parent_linkage value.", vhdx_header.data_write_id);
			}
			else if parent.parent_linkage2 == vhdx_header.data_write_id
			{
				println!("Parent linkage Data Write GUID {} identified by parent_linkage2 value.", vhdx_header.data_write_id);
			}
			else
			{
				ensure!(false, "Parent disk located at {} has Data Write GUID {} but metadata expected an ID of either {} or {}.",
					&file_path, vhdx_header.data_write_id, parent.parent_linkage, parent.parent_linkage2);
			}
		}

		if payload_blocks.iter().any(|x| x.state == PayloadBlockState::NotPresent || x.state == PayloadBlockState::PartiallyPresent)
		{
			disk_type = VhdType::Dynamic;
		}

		println!("VHDX file {} is {}.", &file_path, disk_type);
		println!("File signature is created by {}.", header.creator);
		println!();
		println!("VHDX header at 0x{:X} says:", vhdx_offset);
		println!("	Checksum is				0x{:X}.", vhdx_header.checksum);
		println!("	Current sequence number is		0x{:X}.", vhdx_header.sequence_number);
		println!("	File Write GUID is			{}.", vhdx_header.file_write_id);
		println!("	Data Write GUID is			{}.", vhdx_header.data_write_id);
		println!("	Log GUID is				{}.", vhdx_header.log_id);
		println!("	Log version is				{}.", vhdx_header.log_version);
		println!("	Version is				{}.", vhdx_header.version);
		println!("	Log length is				0x{:X}.", vhdx_header.log_length);
		println!("	Log Offset is				0x{:X}.", vhdx_header.log_offset);
		println!();

		println!("Region table contains:");
		println!("	Checksum is				0x{:X}.", region_table.checksum);
		println!("	Entry count is				0x{:X}.", region_table.entry_count);
		println!("	Regions:");
		for entry in region_table.entries
		{
			match entry.region_type
			{
				RegionType::BAT => println!("		Type:				Block Allocation Table"),
				RegionType::Metadata => println!("		Type:				Metadata"),
				RegionType::Unknown => println!("		Type:			Unknown"),
			}
			println!("		Region ID:			{}.", entry.object_id);
			println!("		Region offset:			0x{:X}.", entry.object_offset);
			println!("		Region length:			0x{:X}.", entry.object_length);
			println!("		Required:			{}.", entry.required);
			println!();
		}

		if print_blocks
		{
			println!("Payload blocks:");
			let mut block_index: u64 = 0;
			for payload in payload_blocks
			{
				println!("	Block {} at offset {}MiB is {}.", block_index, payload.file_offset_mb, payload.state);
				block_index += 1;
			}
			println!();

			println!("Sector blocks:");
			block_index = 0;
			for sector in sector_blocks
			{
				println!("	Block {} at offset {}MiB is {}.", block_index, sector.file_offset_mb, sector.state);
				block_index += 1;
			}
			println!();
		}

		println!("Metadata table contains:");
		println!("	Entry count is:				0x{:X}.", metadata_table.entry_count);
		println!("	Metadata entries:");
		for entry in metadata_table.entries
		{
			println!("		Metadata type:			{}.", match entry.metadata_type
				{
					MetadataType::FileParameters => "File Parameters",
					MetadataType::VirtualDiskSize => "Virtual Disk Size",
					MetadataType::VirtualDiskId => "Virtual Disk ID",
					MetadataType::LogicalSectorSize => "Logical Sector Size",
					MetadataType::PhysicalSectorSize => "Physical Sector Size",
					MetadataType::ParentLocator => "Parent Locator",
					MetadataType::Unknown => "Unknown",
				}
			);
			println!("		Metadata ID:			{}.", entry.object_id);
			println!("		Metadata offset:		0x{:X}.", entry.object_offset);
			println!("		Metadata length:		0x{:X}.", entry.object_length);
			println!("		Is User:			{}.", entry.is_user);
			println!("		Is Virtual Disk:		{}.", entry.is_virtual_disk);
			println!("		Is Required:			{}.", entry.is_required);
			println!();
		}

		println!("Metadata contains:");
		println!("	Block size is:				0x{:X}.", metadata.file_parameters.block_size);
		println!("	Leave block allocated:			{}.", metadata.file_parameters.leave_block_allocated);
		println!("	Has parent:				{}.", metadata.file_parameters.has_parent);
		println!("	Virtual disk size:			0x{:X}.", metadata.virtual_disk_size);
		println!("	Virtual disk size on disk:		0x{:X}.", vhdx_file.metadata()?.len());
		println!("	Virtual disk ID:			{}.", metadata.virtual_disk_id);
		println!("	Logical sector size:			0x{:X}.", metadata.logical_sector_size);
		println!("	Physical sector size:			0x{:X}.", metadata.physical_sector_size);
		if metadata.parent_locator.is_some()
		{
			let locator = &metadata.parent_locator_dict.as_ref().unwrap();
			println!("	Parent locator contains:");
			println!("		Locator type:			{}.", match locator.locator_type
				{
					ParentLocatorType::Vhdx => "VHDX",
					ParentLocatorType::Unknown => "Unknown",
				}
			);
			println!("		Locator type ID:		{}.", locator.locator_type_id);
			println!("		Locator key/value count:	0x{:X}.", locator.key_value_count);
			for locatorkv in &locator.entries
			{
				println!("			Key offset:		0x{:X}.", locatorkv.key_offset);
				println!("			Key length:		0x{:X}.", locatorkv.key_length);
				println!("			Key:			{}.", locatorkv.key);
				println!();
				println!("			Value offset:		0x{:X}.", locatorkv.value_offset);
				println!("			Value length:		0x{:X}.", locatorkv.value_length);
				println!("			Value:			{}.", locatorkv.value);
				println!();
			}
		}
		else
		{
			println!();
			println!("	Parent locator absent, disk is the head of its chain.");
			println!();
		}

		if follow_chain && metadata.parent_locator.is_some()
		{
			match metadata.parent_locator.as_ref().unwrap().locator_type
			{
				ParentLocatorType::Vhdx => 
				{
					parent_locator = metadata.parent_locator;
					file_path = calc_parent_path(parent_locator.as_ref().unwrap(), &Path::new(OsStr::new(&file_path)))?;
				},
				ParentLocatorType::Unknown => 
				{
					println!("Could not follow locator for unknown parent type {}.",
						&metadata.parent_locator_dict.unwrap().locator_type_id);
				},
			}
		}
		else
		{
			break;
		}
	}

	return Ok(());
}