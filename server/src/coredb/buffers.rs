/*
 * Created on Mon Jul 12 2021
 *
 * This file is a part of Skytable
 * Skytable (formerly known as TerrabaseDB or Skybase) is a free and open-source
 * NoSQL database written by Sayan Nandan ("the Author") with the
 * vision to provide flexibility in data modelling without compromising
 * on performance, queryability or scalability.
 *
 * Copyright (c) 2021, Sayan Nandan <ohsayan@outlook.com>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 *
*/

use super::array::Array;
use core::ops::Deref;
use core::str;

macro_rules! push_self {
    ($self:expr, $what:expr) => {
        $self.inner_stack.push_unchecked($what);
    };
}

macro_rules! lut {
    ($e:expr) => {
        PAIR_MAP_LUT[($e) as usize]
    };
}

const PAIR_MAP_LUT: [u8; 200] = [
    0x30, 0x30, 0x30, 0x31, 0x30, 0x32, 0x30, 0x33, 0x30, 0x34, 0x30, 0x35, 0x30, 0x36, 0x30, 0x37,
    0x30, 0x38, 0x30, 0x39, // 0x30
    0x31, 0x30, 0x31, 0x31, 0x31, 0x32, 0x31, 0x33, 0x31, 0x34, 0x31, 0x35, 0x31, 0x36, 0x31, 0x37,
    0x31, 0x38, 0x31, 0x39, // 0x31
    0x32, 0x30, 0x32, 0x31, 0x32, 0x32, 0x32, 0x33, 0x32, 0x34, 0x32, 0x35, 0x32, 0x36, 0x32, 0x37,
    0x32, 0x38, 0x32, 0x39, // 0x32
    0x33, 0x30, 0x33, 0x31, 0x33, 0x32, 0x33, 0x33, 0x33, 0x34, 0x33, 0x35, 0x33, 0x36, 0x33, 0x37,
    0x33, 0x38, 0x33, 0x39, // 0x33
    0x34, 0x30, 0x34, 0x31, 0x34, 0x32, 0x34, 0x33, 0x34, 0x34, 0x34, 0x35, 0x34, 0x36, 0x34, 0x37,
    0x34, 0x38, 0x34, 0x39, // 0x34
    0x35, 0x30, 0x35, 0x31, 0x35, 0x32, 0x35, 0x33, 0x35, 0x34, 0x35, 0x35, 0x35, 0x36, 0x35, 0x37,
    0x35, 0x38, 0x35, 0x39, // 0x35
    0x36, 0x30, 0x36, 0x31, 0x36, 0x32, 0x36, 0x33, 0x36, 0x34, 0x36, 0x35, 0x36, 0x36, 0x36, 0x37,
    0x36, 0x38, 0x36, 0x39, // 0x36
    0x37, 0x30, 0x37, 0x31, 0x37, 0x32, 0x37, 0x33, 0x37, 0x34, 0x37, 0x35, 0x37, 0x36, 0x37, 0x37,
    0x37, 0x38, 0x37, 0x39, // 0x37
    0x38, 0x30, 0x38, 0x31, 0x38, 0x32, 0x38, 0x33, 0x38, 0x34, 0x38, 0x35, 0x38, 0x36, 0x38, 0x37,
    0x38, 0x38, 0x38, 0x39, // 0x38
    0x39, 0x30, 0x39, 0x31, 0x39, 0x32, 0x39, 0x33, 0x39, 0x34, 0x39, 0x35, 0x39, 0x36, 0x39, 0x37,
    0x39, 0x38, 0x39, 0x39, // 0x39
];

/// A 32-bit integer buffer with one extra byte
pub type Integer32Buffer = Integer32BufferRaw<11>;

#[derive(Debug)]
/// A buffer for unsigned 32-bit integers with one _extra byte_ of memory reserved for
/// adding characters. On initialization (through [`Self::init`]), your integer will be
/// encoded and stored into the _unsafe array_
pub struct Integer32BufferRaw<const N: usize> {
    inner_stack: Array<u8, 11>,
}

impl<const N: usize> Integer32BufferRaw<N> {
    /// Initialize a buffer
    pub fn init(integer: u32) -> Self {
        let mut slf = Self {
            inner_stack: Array::new(),
        };
        unsafe {
            slf._init_integer(integer);
        }
        slf
    }
    /// Initialize an integer. This is unsafe to be called outside because you'll be
    /// pushing in another integer and might end up corrupting your own stack as all
    /// pushes are unchecked!
    unsafe fn _init_integer(&mut self, mut val: u32) {
        if val < 10_000 {
            let d1 = (val / 100) << 1;
            let d2 = (val % 100) << 1;
            if val >= 1000 {
                push_self!(self, lut!(d1));
            }
            if val >= 100 {
                push_self!(self, lut!(d1 + 1));
            }
            if val >= 10 {
                push_self!(self, lut!(d2));
            }
            push_self!(self, lut!(d2 + 1));
        } else if val < 100_000_000 {
            let b = val / 10000;
            let c = val % 10000;
            let d1 = (b / 100) << 1;
            let d2 = (b % 100) << 1;
            let d3 = (c / 100) << 1;
            let d4 = (c % 100) << 1;

            if val > 10_000_000 {
                push_self!(self, lut!(d1));
            }
            if val > 1_000_000 {
                push_self!(self, lut!(d1 + 1));
            }
            if val > 100_000 {
                push_self!(self, lut!(d2));
            }
            push_self!(self, lut!(d2 + 1));
            push_self!(self, lut!(d3));
            push_self!(self, lut!(d3 + 1));
            push_self!(self, lut!(d4));
            push_self!(self, lut!(d4 + 1));
        } else {
            // worst, 1B or more
            let a = val / 100000000;
            val %= 100000000;

            if a >= 10 {
                let i = a << 1;
                push_self!(self, lut!(i));
                push_self!(self, lut!(i + 1));
            } else {
                push_self!(self, 0x30);
            }
            let b = val / 10000;
            let c = val % 10000;
            let d1 = (b / 100) << 1;
            let d2 = (b % 100) << 1;
            let d3 = (c / 100) << 1;
            let d4 = (c % 100) << 1;
            // write back
            push_self!(self, lut!(d1));
            push_self!(self, lut!(d1 + 1));
            push_self!(self, lut!(d2));
            push_self!(self, lut!(d2 + 1));
            push_self!(self, lut!(d3));
            push_self!(self, lut!(d3 + 1));
            push_self!(self, lut!(d4));
            push_self!(self, lut!(d4 + 1));
        }
    }
    /// **This is very unsafe** Only push something when you know that the capacity won't overflow
    /// your allowance of 11 bytes. Oh no, there's no panic for you because you'll silently
    /// corrupt your own memory (or others' :/)
    pub unsafe fn push(&mut self, val: u8) {
        push_self!(self, val)
    }
}

impl<const N: usize> Deref for Integer32BufferRaw<N> {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        unsafe { str::from_utf8_unchecked(&self.inner_stack) }
    }
}

impl<const N: usize> AsRef<str> for Integer32BufferRaw<N> {
    fn as_ref(&self) -> &str {
        &self
    }
}

impl<T, const N: usize> PartialEq<T> for Integer32BufferRaw<N>
where
    T: AsRef<str>,
{
    fn eq(&self, other_str: &T) -> bool {
        self.as_ref() == other_str.as_ref()
    }
}

#[test]
fn test_int32_buffer() {
    let buffer = Integer32Buffer::init(256);
    assert_eq!(buffer, 256.to_string());
}

#[test]
fn test_int32_buffer_push() {
    let mut buffer = Integer32Buffer::init(278);
    unsafe {
        buffer.push(b'?');
    }
    assert_eq!(buffer, "278?");
}

/// A 64-bit integer buffer with **no extra byte**
pub type Integer64 = Integer64BufferRaw<20>;

#[derive(Debug)]
pub struct Integer64BufferRaw<const N: usize> {
    inner_stack: Array<u8, N>,
}

const Z_8: u64 = 100_000_000;
const Z_9: u64 = Z_8 * 10;
const Z_10: u64 = Z_9 * 10;
const Z_11: u64 = Z_10 * 10;
const Z_12: u64 = Z_11 * 10;
const Z_13: u64 = Z_12 * 10;
const Z_14: u64 = Z_13 * 10;
const Z_15: u64 = Z_14 * 10;
const Z_16: u64 = Z_15 * 10;

impl<const N: usize> Integer64BufferRaw<N> {
    pub fn init(integer: u64) -> Self {
        let mut slf = Self {
            inner_stack: Array::new(),
        };
        unsafe {
            slf._init_integer(integer);
        }
        slf
    }
    unsafe fn _init_integer(&mut self, mut int: u64) {
        if int < Z_8 {
            if int < 10_000 {
                let d1 = (int / 100) << 1;
                let d2 = (int % 100) << 1;
                if int >= 1_000 {
                    push_self!(self, lut!(d1));
                }
                if int >= 100 {
                    push_self!(self, lut!(d1 + 1));
                }
                if int >= 10 {
                    push_self!(self, lut!(d2));
                }
                push_self!(self, lut!(d2 + 1));
            } else {
                let b = int / 10000;
                let c = int % 10000;
                let d1 = (b / 100) << 1;
                let d2 = (b % 100) << 1;
                let d3 = (c / 100) << 1;
                let d4 = (c % 100) << 1;
                if int >= 10_000_000 {
                    push_self!(self, lut!(d1));
                }
                if int >= 1_000_000 {
                    push_self!(self, lut!(d1 + 1));
                }
                if int >= 100_000 {
                    push_self!(self, lut!(d2));
                }
                push_self!(self, lut!(d2 + 1));
                push_self!(self, lut!(d3));
                push_self!(self, lut!(d3 + 1));
                push_self!(self, lut!(d4));
                push_self!(self, lut!(d4 + 1));
            }
        } else if int < Z_16 {
            // lets do 8 at a time
            let v0 = int / Z_8;
            let v1 = int & Z_8;
            let b0 = v0 / 10000;
            let c0 = v0 % 10000;
            let d1 = (b0 / 100) << 1;
            let d2 = (b0 % 100) << 1;
            let d3 = (c0 / 100) << 1;
            let d4 = (c0 % 100) << 1;
            let b1 = v1 / 10000;
            let c1 = v1 % 10000;
            let d5 = (b1 / 100) << 1;
            let d6 = (b1 % 100) << 1;
            let d7 = (c1 / 100) << 1;
            let d8 = (c1 % 100) << 1;
            if int >= Z_15 {
                push_self!(self, lut!(d1));
            }
            if int >= Z_14 {
                push_self!(self, lut!(d1 + 1));
            }
            if int >= Z_13 {
                push_self!(self, lut!(d2));
            }
            if int >= Z_12 {
                push_self!(self, lut!(d2 + 1));
            }
            if int >= Z_11 {
                push_self!(self, lut!(d3));
            }
            if int >= Z_10 {
                push_self!(self, lut!(d3 + 1));
            }
            if int >= Z_9 {
                push_self!(self, lut!(d4));
            }
            push_self!(self, lut!(d4 + 1));
            push_self!(self, lut!(d5));
            push_self!(self, lut!(d5 + 1));
            push_self!(self, lut!(d6));
            push_self!(self, lut!(d6 + 1));
            push_self!(self, lut!(d7));
            push_self!(self, lut!(d7 + 1));
            push_self!(self, lut!(d8));
            push_self!(self, lut!(d8 + 1));
        } else {
            let a = int / Z_16;
            int %= Z_16;
            if a < 10 {
                push_self!(self, 0x30 + a as u8);
            } else if a < 100 {
                let i = a << 1;
                push_self!(self, lut!(i));
                push_self!(self, lut!(i + 1));
            } else if a < 1000 {
                push_self!(self, 0x30 + (a / 100) as u8);
                let i = (a % 100) << 1;
                push_self!(self, lut!(i));
                push_self!(self, lut!(i + 1));
            } else {
                let i = (a / 100) << 1;
                let j = (a % 100) << 1;
                push_self!(self, lut!(i));
                push_self!(self, lut!(i + 1));
                push_self!(self, lut!(j));
                push_self!(self, lut!(j + 1));
            }

            let v0 = int / Z_8;
            let v1 = int % Z_8;
            let b0 = v0 / 10000;
            let c0 = v0 % 10000;
            let d1 = (b0 / 100) << 1;
            let d2 = (b0 % 100) << 1;
            let d3 = (c0 / 100) << 1;
            let d4 = (c0 % 100) << 1;
            let b1 = v1 / 10000;
            let c1 = v1 % 10000;
            let d5 = (b1 / 100) << 1;
            let d6 = (b1 % 100) << 1;
            let d7 = (c1 / 100) << 1;
            let d8 = (c1 % 100) << 1;
            push_self!(self, lut!(d1));
            push_self!(self, lut!(d1 + 1));
            push_self!(self, lut!(d2));
            push_self!(self, lut!(d2 + 1));
            push_self!(self, lut!(d3));
            push_self!(self, lut!(d3 + 1));
            push_self!(self, lut!(d4));
            push_self!(self, lut!(d4 + 1));
            push_self!(self, lut!(d5));
            push_self!(self, lut!(d5 + 1));
            push_self!(self, lut!(d6));
            push_self!(self, lut!(d6 + 1));
            push_self!(self, lut!(d7));
            push_self!(self, lut!(d7 + 1));
            push_self!(self, lut!(d8));
            push_self!(self, lut!(d8 + 1));
        }
    }
}

impl<const N: usize> From<usize> for Integer64BufferRaw<N> {
    fn from(val: usize) -> Self {
        Self::init(val as u64)
    }
}

impl<const N: usize> Deref for Integer64BufferRaw<N> {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        &self.inner_stack
    }
}

impl<const N: usize> AsRef<str> for Integer64BufferRaw<N> {
    fn as_ref(&self) -> &str {
        unsafe { str::from_utf8_unchecked(&self.inner_stack) }
    }
}

impl<T, const N: usize> PartialEq<T> for Integer64BufferRaw<N>
where
    T: AsRef<str>,
{
    fn eq(&self, other_str: &T) -> bool {
        self.as_ref() == other_str.as_ref()
    }
}

#[test]
fn test_int64_buffer() {
    assert_eq!(
        9348910481349849081_u64.to_string(),
        Integer64::init(9348910481349849081_u64).as_ref()
    );
    assert_eq!(u64::MAX.to_string(), Integer64::init(u64::MAX).as_ref());
}
