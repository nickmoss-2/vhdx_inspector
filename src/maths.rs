// SPDX-License-Identifier: MIT
// Copyright (c) Nick Moss.

use num::PrimInt;

// Algorithms taken and adapted from https://stackoverflow.com/questions/63436490/divide-integers-with-floor-ceil-and-outwards-rounding-modes-in-c
pub trait SpecificDivide
{
	fn ceiling_divide<T>(n: T, d: T) -> T where T:PrimInt + core::convert::From<bool>;
	fn floor_divide<T>(n: T, d: T) -> T where T:PrimInt + core::convert::From<bool>;
}

macro_rules! divide_unsigned
{
	($t:ident) =>
	{
		impl SpecificDivide for $t
		{
			fn ceiling_divide<T>(n: T, d: T) -> T where T:PrimInt + core::convert::From<bool>
			{
				return n / d + <T as From<bool>>::from(n % d != T::zero());
			}
			
			fn floor_divide<T>(n: T, d: T) -> T where T:PrimInt + core::convert::From<bool>
			{
				return n / d;
			}
		}
	}
}

macro_rules! divide_signed
{
	($t:ident) =>
	{
		impl SpecificDivide for $t
		{
			fn ceiling_divide<T>(n: T, d: T) -> T where T:PrimInt + core::convert::From<bool>
			{
				return n / d + <T as From<bool>>::from(n % d != T::zero() && ((n < T::zero()) == (d < T::zero())));
			}
			
			fn floor_divide<T>(n: T, d: T) -> T where T:PrimInt + core::convert::From<bool>
			{
				return n / d - <T as From<bool>>::from(n % d != T::zero() && ((n < T::zero()) != (d < T::zero())));
			}
		}
	}
}

divide_unsigned!(u64);
divide_signed!(i64);
