/*
 * Created on Wed Jul 07 2021
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

/*
 Encoding and decoding tested on 32-bit/64-bit Little Endian (Intel x86) and Big Endian
 (MIPS). Also tested UB with miri and memory leaks with valgrind
 -- Sayan on July 8, 2021
*/

#![allow(dead_code)] // TODO(@ohsayan): Remove this once we're done

//! # Storage engine
//!
//! This module contains code to rapidly encode/decode data. All sizes are encoded into unsigned
//! 64-bit integers for compatibility across 16/32/64 bit platforms. This means that a
//! data file generated on a 32-bit machine will work seamlessly on a 64-bit machine
//! and vice versa. Of course, provided that On a 32-bit system, 32 high bits are just zeroed.
//!
//! ## Endianness
//!
//! All sizes are stored in little endian. How everything else is stored is not worth
//! discussing here. Byte swaps just need one instruction on most architectures
//!
//! ## Safety
//!
//! > Trust me, all methods are bombingly unsafe. They do such crazy things that you might not
//! think of using them anywhere outside. This is a specialized parser built for the database.
//! -- Sayan (July 2021)

use crate::coredb::array::Array;
use crate::coredb::htable::Coremap;
use crate::coredb::Data;
use core::hash::Hash;
use core::mem;
use core::ptr;
use core::slice;
use std::collections::HashSet;
use std::io::Write;
// for some astronomical reasons do not mess with this
#[macro_use]
mod macros;
// endof do not mess
pub mod flush;
pub mod interface;
pub mod preload;
pub mod unflush;
// test
#[cfg(test)]
mod tests;

/// The ID of the partition in a keyspace. Using too many keyspaces is an absolute anti-pattern
/// on Skytable, something that it has inherited from prior experience in large scale systems. As
/// such, the maximum number of tables in a keyspace is limited to 4.1 billion tables and ideally,
/// you should never hit that limit.
pub type PartitionID = u32;

/*
    Endian and pointer "appendix":
    We assume a fixed size of 1 for all the cases. All sizes don't hit over isize::MAX as
    guaranteed by our allocation methods. Also, 32-bit to 64-bit and vice versa aren't
    worth discussing here (irrespective of endianness). That's because all sizes are stored
    as unsigned 64-bit integers. So, get the correct byte order, raw cast to u64, down cast
    or up cast as required by the target's pointer width.

    Current limitations:
    - 32-bit is compatible with 64-bit
    - 16-bit is compatible with 64-bit
    - But 64-bit may not be compatible with 32/16 bit due to the difference in sizes

    ------------------------------------------------------
    Appendix I: Same endian, different pointer width (R/W)
    ------------------------------------------------------
    (1) Little endian on little endian (64-bit)
    (A) Writing
    In-memory size: [0, 0, 0, 0, 0, 0, 0, 1] =(u64)> [0, 0, 0, 0, 0, 0, 0, 1] (no op)
    We write to file: [0, 0, 0, 0, 0, 0, 0, 1]
    (B) Reading
    This is read: [0, 0, 0, 0, 0, 0, 0, 1]
    Raw cast =(usize)> [0, 0, 0, 0, 0, 0, 0, 1] (one memcpy)

    (2) Little endian on little endian (32-bit)
    (A) Writing
    In-memory size: [0, 0, 0, 1] =(u64)> [0, 0, 0, 0, 0, 0, 0, 1] (up cast)
    We write to file: [0, 0, 0, 0, 0, 0, 0, 1]
    (B) Reading
    This is read: [0, 0, 0, 0, 0, 0, 0, 1]
    Raw cast =(u64)> [0, 0, 0, 0, 0, 0, 0, 1] (one memcpy)
    Lossy cast =(usize)> [0, 0, 0, 1]

    (3) Big endian on big endian (64-bit)
    (A) Writing
    In-memory size: [1, 0, 0, 0, 0, 0, 0, 0] =(u64)> [1, 0, 0, 0, 0, 0, 0, 0] (no op)
    We write to file: [1, 0, 0, 0, 0, 0, 0, 0]
    (B) Reading
    This is read: [1, 0, 0, 0, 0, 0, 0, 0]
    Raw cast =(usize)> [1, 0, 0, 0, 0, 0, 0, 0] (one memcpy)

    (4) Big endian (64-bit) on big endian (32-bit)
    (A) Writing
    In-memory size: [1, 0, 0, 0] =(u64)> [1, 0, 0, 0, 0, 0, 0, 0] (up cast)
    We write to file: [1, 0, 0, 0, 0, 0, 0, 0]
    (B) Reading
    This is read: [1, 0, 0, 0, 0, 0, 0, 0]
    Raw cast =(u64)> [1, 0, 0, 0, 0, 0, 0, 0] (one memcpy)
    Lossy cast =(usize)> [1, 0, 0, 0]

    ------------------------------------------------------
    Appendix II: Different endian, same pointer width (R/W)
    ------------------------------------------------------
    (1) Little endian on big endian (64-bit)
    (A) Writing
    ^^ See Appendix I/1/A
    (B) Reading
    This is read: [0, 0, 0, 0, 0, 0, 0, 1]
    Raw cast =(u64)> [0, 0, 0, 0, 0, 0, 0, 1] (one memcpy)
    Reverse the bits: [1, 0, 0, 0, 0, 0, 0, 0] (constant time ptr swap)
    Cast =(usize)> [1, 0, 0, 0, 0, 0, 0, 0] (no op)

    (2) Big endian on little endian (64-bit)
    (A) Writing
    ^^ See Appendix I/3/A
    (B) Reading
    This is read: [1, 0, 0, 0, 0, 0, 0, 0]
    Raw cast =(u64)> [1, 0, 0, 0, 0, 0, 0, 0] (one memcpy)
    Reverse the bits: [0, 0, 0, 0, 0, 0, 0, 1] (constant time ptr swap)
    Cast =(usize)> [0, 0, 0, 0, 0, 0, 0, 1] (no op)

    (3) Little endian on big endian (32-bit)
    (A) Writing
    ^^ See Appendix I/2/A
    (B) Reading
    This is read: [0, 0, 0, 0, 0, 0, 0, 1]
    Raw cast =(u64)> [0, 0, 0, 0, 0, 0, 0, 1] (one memcpy)
    Reverse the bits: [1, 0, 0, 0, 0, 0, 0, 0] (constant time ptr swap)
    Lossy cast =(usize)> [1, 0, 0, 0]

    (4) Big endian on little endian (32-bit)
    (A) Writing
    ^^ See Appendix I/4/A
    (B) Reading
    This is read: [1, 0, 0, 0, 0, 0, 0, 0]
    Raw cast =(u64)> [1, 0, 0, 0, 0, 0, 0, 0] (one memcpy)
    Reverse the bits: [0, 0, 0, 0, 0, 0, 0, 1] (constant time ptr swap)
    Lossy cast =(usize)> [0, 0, 0, 1]

    ------------------------------------------------------
    Appendix III: Warnings
    ------------------------------------------------------
    (1) Gotchas on 32-bit big endian
    (A) While writing
    Do not swap bytes before up cast
    (B) While reading
    Do not down cast before swapping bytes
*/

/// Get the raw bytes of anything.
///
/// DISCLAIMER: THIS FUNCTION CAN DO TERRIBLE THINGS
unsafe fn raw_byte_repr<'a, T: 'a>(len: &'a T) -> &'a [u8] {
    {
        let ptr: *const u8 = mem::transmute(len);
        slice::from_raw_parts::<'a>(ptr, mem::size_of::<T>())
    }
}

mod se {
    use super::*;
    use crate::coredb::memstore::Keyspace;
    /// Serialize a map into a _writable_ thing
    pub fn serialize_map(
        map: &Coremap<Data, Data>,
        model_code: u8,
    ) -> Result<Vec<u8>, std::io::Error> {
        /*
        [1B: Model Mark][LEN:8B][KLEN:8B|VLEN:8B][K][V][KLEN:8B][VLEN:8B]...
        */
        // write the len header first
        let mut w = Vec::with_capacity(128);
        self::raw_serialize_map(map, &mut w, model_code)?;
        Ok(w)
    }

    /// Serialize a map and write it to a provided buffer
    pub fn raw_serialize_map<W: Write>(
        map: &Coremap<Data, Data>,
        w: &mut W,
        model_code: u8,
    ) -> std::io::Result<()> {
        unsafe {
            w.write_all(raw_byte_repr(&model_code))?;
            w.write_all(raw_byte_repr(&to_64bit_little_endian!(map.len())))?;
            // now the keys and values
            for kv in map.iter() {
                let (k, v) = (kv.key(), kv.value());
                w.write_all(raw_byte_repr(&to_64bit_little_endian!(k.len())))?;
                w.write_all(raw_byte_repr(&to_64bit_little_endian!(v.len())))?;
                w.write_all(k)?;
                w.write_all(v)?;
            }
        }
        Ok(())
    }

    /// Serialize a set and write it to a provided buffer
    pub fn raw_serialize_set<W, K, V>(map: &Coremap<K, V>, w: &mut W) -> std::io::Result<()>
    where
        W: Write,
        K: Eq + Hash + AsRef<[u8]>,
    {
        unsafe {
            w.write_all(raw_byte_repr(&to_64bit_little_endian!(map.len())))?;
            // now the keys and values
            for kv in map.iter() {
                let key = kv.key().as_ref();
                w.write_all(raw_byte_repr(&to_64bit_little_endian!(key.len())))?;
                w.write_all(key)?;
            }
        }
        Ok(())
    }

    /// Generate a partition map for the given keyspace
    /// ```text
    /// [8B: EXTENT]([8B: LEN][?B: PARTITION ID][1B: Storage type])*
    /// ```
    pub fn raw_serialize_partmap<W: Write>(w: &mut W, keyspace: &Keyspace) -> std::io::Result<()> {
        unsafe {
            // extent
            w.write_all(raw_byte_repr(&to_64bit_little_endian!(keyspace
                .tables
                .len())))?;
            for table in keyspace.tables.iter() {
                // partition ID len
                w.write_all(raw_byte_repr(&to_64bit_little_endian!(table.key().len())))?;
                // parition ID
                w.write_all(table.key())?;
                // now storage type
                w.write_all(raw_byte_repr(&table.storage_type()))?;
            }
        }
        Ok(())
    }
}

mod de {
    use super::*;
    use std::collections::HashMap;

    pub trait DeserializeFrom {
        fn is_expected_len(clen: usize) -> bool;
        fn from_slice(slice: &[u8]) -> Self;
    }

    impl<const N: usize> DeserializeFrom for Array<u8, N> {
        fn is_expected_len(clen: usize) -> bool {
            clen <= N
        }
        fn from_slice(slice: &[u8]) -> Self {
            unsafe { Self::from_slice(slice) }
        }
    }

    /// Deserialize a set to a custom type
    pub fn deserialize_set_ctype<T>(data: &[u8]) -> Option<HashSet<T>>
    where
        T: DeserializeFrom + Eq + Hash,
    {
        // First read the length header
        if data.len() < 8 {
            // so the file doesn't even have the length header? noice, just return
            None
        } else {
            unsafe {
                // so we have 8B. Just unsafe access and transmute it
                let len = transmute_len(data.as_ptr());
                let mut set = HashSet::with_capacity(len);
                // this is what we have left: [KLEN:8B]*
                // move 8 bytes ahead since we're done with len
                let mut ptr = data.as_ptr().add(8);
                let end_ptr = data.as_ptr().add(data.len());
                for _ in 0..len {
                    if (ptr.add(8)) >= end_ptr {
                        // not enough space and even if there is a len
                        // there is no value. This is even true for ZSTs
                        return None;
                    }
                    let lenkey = transmute_len(ptr);
                    ptr = ptr.add(8);
                    if (ptr.add(lenkey)) > end_ptr {
                        // not enough data left
                        return None;
                    }
                    if !T::is_expected_len(lenkey) {
                        return None;
                    }
                    // get the key as a raw slice, we've already checked if end_ptr is less
                    let key = T::from_slice(slice::from_raw_parts(ptr, lenkey));
                    // move the ptr ahead; done with the key
                    ptr = ptr.add(lenkey);
                    // push it in
                    if !set.insert(key) {
                        // repeat?; that's not what we wanted
                        return None;
                    }
                }
                if ptr == end_ptr {
                    Some(set)
                } else {
                    // nope, someone gave us more data
                    None
                }
            }
        }
    }

    /// Deserializes a map-like set which has an 1B _bytemark_ for every entry
    pub fn deserialize_set_ctype_bytemark<T>(data: &[u8]) -> Option<HashMap<T, u8>>
    where
        T: DeserializeFrom + Eq + Hash,
    {
        // First read the length header
        if data.len() < 8 {
            // so the file doesn't even have the length header? noice, just return
            None
        } else {
            unsafe {
                // so we have 8B. Just unsafe access and transmute it
                let len = transmute_len(data.as_ptr());
                let mut set = HashMap::with_capacity(len);
                // this is what we have left: [KLEN:8B]*
                // move 8 bytes ahead since we're done with len
                let mut ptr = data.as_ptr().add(8);
                let end_ptr = data.as_ptr().add(data.len());
                for _ in 0..len {
                    if (ptr.add(8)) >= end_ptr {
                        // not enough space and even if there is a len
                        // there is no value. This is even true for ZSTs
                        return None;
                    }
                    let lenkey = transmute_len(ptr);
                    ptr = ptr.add(8);
                    if (ptr.add(lenkey + 1)) > end_ptr {
                        // not enough data left
                        return None;
                    }
                    if !T::is_expected_len(lenkey) {
                        return None;
                    }
                    // get the key as a raw slice, we've already checked if end_ptr is less
                    let key = T::from_slice(slice::from_raw_parts(ptr, lenkey));
                    // move the ptr ahead; done with the key
                    ptr = ptr.add(lenkey);
                    let bytemark = ptr::read(ptr);
                    ptr = ptr.add(1);
                    // push it in
                    if set.insert(key, bytemark).is_some() {
                        // repeat?; that's not what we wanted
                        return None;
                    }
                }
                if ptr == end_ptr {
                    Some(set)
                } else {
                    // nope, someone gave us more data
                    None
                }
            }
        }
    }
    /// Deserialize a file that contains a serialized map. This also returns the model code
    pub fn deserialize_map(data: Vec<u8>) -> Option<(Coremap<Data, Data>, u8)> {
        // First read the length header
        if data.len() < 9 {
            // so the file doesn't even have the length/model header? noice, just return
            None
        } else {
            unsafe {
                /*
                 UNSAFE(@ohsayan): Everything done here is unsafely safe. We
                 reinterpret bits of one type as another. What could be worse?
                 nah, it's not that bad. We know that the byte representations
                 would be in the way we expect. If the data is corrupted, we
                 can guarantee that we won't ever read incorrect lengths of data
                 and we won't read into others' memory (or corrupt our own)
                */
                let mut ptr = data.as_ptr();
                let modelcode: u8 = ptr::read(ptr);

                // model check
                if modelcode > 3 {
                    // this model isn't supposed to have more than 3. Corrupted data
                    return None;
                }

                ptr = ptr.add(1);
                // so we have 8B. Just unsafe access and transmute it; nobody cares
                let len = transmute_len(ptr);
                // move 8 bytes ahead since we're done with len
                ptr = ptr.add(8);
                let hm = Coremap::with_capacity(len);
                // this is what we have left: [KLEN:8B][VLEN:8B]
                let end_ptr = data.as_ptr().add(data.len());
                for _ in 0..len {
                    if (ptr.add(16)) >= end_ptr {
                        // not enough space
                        return None;
                    }
                    let lenkey = transmute_len(ptr);
                    ptr = ptr.add(8);
                    let lenval = transmute_len(ptr);
                    ptr = ptr.add(8);
                    if (ptr.add(lenkey + lenval)) > end_ptr {
                        // not enough data left
                        return None;
                    }
                    // get the key as a raw slice, we've already checked if end_ptr is less
                    let key = Data::copy_from_slice(slice::from_raw_parts(ptr, lenkey));
                    // move the ptr ahead; done with the key
                    ptr = ptr.add(lenkey);
                    let val = Data::copy_from_slice(slice::from_raw_parts(ptr, lenval));
                    // move the ptr ahead; done with the value
                    ptr = ptr.add(lenval);
                    // push it in
                    hm.upsert(key, val);
                }
                if ptr == end_ptr {
                    Some((hm, modelcode))
                } else {
                    // nope, someone gave us more data
                    None
                }
            }
        }
    }

    #[allow(clippy::needless_return)] // Clippy really misunderstands this
    pub(super) unsafe fn transmute_len(start_ptr: *const u8) -> usize {
        little_endian!({
            // So we have an LE target
            is_64_bit!({
                // 64-bit LE
                return ptr::read_unaligned(start_ptr.cast());
            });
            not_64_bit!({
                // 32-bit LE
                let ret1: u64 = ptr::read_unaligned(start_ptr.cast());
                // lossy cast
                let ret = ret1 as usize;
                if ret > (isize::MAX as usize) {
                    // this is a backup method for us incase a giant 48-bit address is
                    // somehow forced to be read on this machine
                    panic!("RT panic: Very high size for current pointer width");
                }
                return ret;
            });
        });

        big_endian!({
            // so we have a BE target
            is_64_bit!({
                // 64-bit big endian
                let ret: usize = ptr::read_unaligned(start_ptr.cast());
                // swap byte order
                return ret.swap_bytes();
            });
            not_64_bit!({
                // 32-bit big endian
                let ret: u64 = ptr::read_unaligned(start_ptr.cast());
                // swap byte order and lossy cast
                let ret = (ret.swap_bytes()) as usize;
                // check if overflow
                if ret > (isize::MAX as usize) {
                    // this is a backup method for us incase a giant 48-bit address is
                    // somehow forced to be read on this machine
                    panic!("RT panic: Very high size for current pointer width");
                }
                return ret;
            });
        });
    }
}
