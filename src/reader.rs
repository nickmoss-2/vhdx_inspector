// SPDX-License-Identifier: MIT
// Copyright (c) Nick Moss.

use std::io::{Seek, Read, SeekFrom};

use byteorder::{LittleEndian,ReadBytesExt};
use failure::Fallible;
use num::PrimInt;
use uuid::Uuid;

pub fn read_into(data: &mut (impl Read + Seek), offset: usize, buffer: &mut Vec<u8>) -> Fallible<()>
{
	data.seek(SeekFrom::Start(offset as u64))?;
	let _ = data.read_exact(buffer)?;

	return Ok(());
}

pub trait ReadValueOtherTyped
{
	fn read_value<T>(&mut self, data: &mut (impl Read + Seek)) -> Fallible<()> where Self: Sized, T: Default + ReadValue + PrimInt;
	fn read_value_off<T>(&mut self, data: &mut (impl Read + Seek), offset: usize) -> Fallible<()> where Self: Sized, T: Default + ReadValue + PrimInt;
}

impl ReadValueOtherTyped for bool
{
	fn read_value<T: Default + ReadValue + PrimInt>(&mut self, data: &mut (impl Read + Seek)) -> Fallible<()>
	{
		let mut hop_value: T = T::default();
		hop_value.read_value(data)?;
		*self = hop_value != T::zero();
		return Ok(());
	}

	fn read_value_off<T: Default + ReadValue + PrimInt>(&mut self, data: &mut (impl Read + Seek), offset: usize) -> Fallible<()>
	{
		data.seek(SeekFrom::Start(offset as u64))?;
		self.read_value::<T>(data)?;
		return Ok(());
	}
}

pub trait ReadValue
{
	fn read_value(&mut self, data: &mut (impl Read + Seek)) -> Fallible<()> where Self: Sized;
	fn read_value_off(&mut self, data: &mut (impl Read + Seek), offset: usize) -> Fallible<()> where Self: Sized;
}

impl ReadValue for u16
{
	fn read_value(&mut self, data: &mut (impl Read + Seek)) -> Fallible<()>
	{
		*self = data.read_u16::<LittleEndian>()?;
		return Ok(());
	}

	fn read_value_off(&mut self, data: &mut (impl Read + Seek), offset: usize) -> Fallible<()>
	{
		data.seek(SeekFrom::Start(offset as u64))?;
		self.read_value(data)?;
		return Ok(());
	}
}

impl ReadValue for u32
{
	fn read_value(&mut self, data: &mut (impl Read + Seek)) -> Fallible<()>
	{
		*self = data.read_u32::<LittleEndian>()?;
		return Ok(());
	}

	fn read_value_off(&mut self, data: &mut (impl Read + Seek), offset: usize) -> Fallible<()>
	{
		data.seek(SeekFrom::Start(offset as u64))?;
		self.read_value(data)?;
		return Ok(());
	}
}

impl ReadValue for u64
{
	fn read_value(&mut self, data: &mut (impl Read + Seek)) -> Fallible<()>
	{
		*self = data.read_u64::<LittleEndian>()?;
		return Ok(());
	}

	fn read_value_off(&mut self, data: &mut (impl Read + Seek), offset: usize) -> Fallible<()>
	{
		data.seek(SeekFrom::Start(offset as u64))?;
		self.read_value(data)?;
		return Ok(());
	}
}

impl ReadValue for u128
{
	fn read_value(&mut self, data: &mut (impl Read + Seek)) -> Fallible<()>
	{
		*self = data.read_u128::<LittleEndian>()?;
		return Ok(());
	}

	fn read_value_off(&mut self, data: &mut (impl Read + Seek), offset: usize) -> Fallible<()>
	{
		data.seek(SeekFrom::Start(offset as u64))?;
		self.read_value(data)?;
		return Ok(());
	}
}

impl ReadValue for usize
{
	fn read_value(&mut self, data: &mut (impl Read + Seek)) -> Fallible<()>
	{
		*self = data.read_u64::<LittleEndian>()? as usize;
		return Ok(());
	}

	fn read_value_off(&mut self, data: &mut (impl Read + Seek), offset: usize) -> Fallible<()>
	{
		data.seek(SeekFrom::Start(offset as u64))?;
		self.read_value(data)?;
		return Ok(());
	}
}

impl ReadValue for Uuid
{
	fn read_value(&mut self, data: &mut (impl Read + Seek)) -> Fallible<()>
	{
		let mut u_val:Vec<u8> = vec![0;16];
		u_val.read_value(data)?;
		*self = Uuid::from_slice_le(u_val.as_slice())?;
		return Ok(());
	}

	fn read_value_off(&mut self, data: &mut (impl Read + Seek), offset: usize) -> Fallible<()>
	{
		data.seek(SeekFrom::Start(offset as u64))?;
		self.read_value(data)?;
		return Ok(());
	}
}

impl ReadValue for Vec<u8>
{
	fn read_value(&mut self, data: &mut (impl Read + Seek)) -> Fallible<()>
	{
		data.read_exact(self)?;
		return Ok(());
	}

	fn read_value_off(&mut self, data: &mut (impl Read + Seek), offset: usize) -> Fallible<()>
	{
		data.seek(SeekFrom::Start(offset as u64))?;
		self.read_value(data)?;
		return Ok(());
	}
}

impl ReadValue for Vec<u16>
{
	fn read_value(&mut self, data: &mut (impl Read + Seek)) -> Fallible<()>
	{
		(data).read_u16_into::<LittleEndian>(self)?;
		return Ok(());
	}

	fn read_value_off(&mut self, data: &mut (impl Read + Seek), offset: usize) -> Fallible<()>
	{
		data.seek(SeekFrom::Start(offset as u64))?;
		self.read_value(data)?;
		return Ok(());
	}
}

impl ReadValue for String
{
	fn read_value(&mut self, data: &mut (impl Read + Seek)) -> Fallible<()>
	{
		let mut creator_u16:Vec<u16> = vec![0;self.capacity()];
		creator_u16.read_value(data)?;
		*self = String::from_utf16(&creator_u16)?;
		return Ok(());
	}

	fn read_value_off(&mut self, data: &mut (impl Read + Seek), offset: usize) -> Fallible<()>
	{
		data.seek(SeekFrom::Start(offset as u64))?;
		self.read_value(data)?;
		return Ok(());
	}
}