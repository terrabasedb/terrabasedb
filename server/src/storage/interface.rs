/*
 * Created on Sat Jul 10 2021
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

//! Interfaces with the file system

use crate::coredb::htable::Coremap;
use crate::coredb::htable::Data;
use crate::coredb::memstore::Keyspace;
use crate::coredb::memstore::Memstore;
use std::io::Result as IoResult;
use std::io::{BufWriter, Write};

pub const DIR_KSROOT: &str = "data/ks";
pub const DIR_SNAPROOT: &str = "data/snaps";
pub const DIR_BACKUPS: &str = "data/backups";
pub const DIR_ROOT: &str = "data";

/// This creates the root directory structure:
/// ```
/// data/
///     ks/
///         ks1/
///         ks2/
///         ks3/
///     snaps/
///         ks1/
///             tbl1/
///             tbl2/
///         ks2/
///             tbl1/
///             tbl2/
///         ks3/
///             tbl1/
///             tbl2/
///     backups/
/// ```
///
/// If any directories exist, they are simply ignored
pub fn create_tree(memroot: Memstore) -> IoResult<()> {
    try_dir_ignore_existing!(DIR_ROOT, DIR_KSROOT, DIR_BACKUPS, DIR_SNAPROOT);
    for ks in memroot.keyspaces.iter() {
        unsafe {
            try_dir_ignore_existing!(concat_path!(DIR_KSROOT, ks.key().as_str()))?;
            for tbl in ks.value().tables.iter() {
                try_dir_ignore_existing!(concat_path!(
                    DIR_SNAPROOT,
                    ks.key().as_str(),
                    tbl.key().as_str()
                ))?;
            }
        }
    }
    Ok(())
}

/// Uses a buffered writer under the hood to improve write performance as the provided
/// writable interface might be very slow. The buffer does flush once done, however, it
/// is important that you fsync yourself!
pub fn serialize_map_into_slow_buffer<T: Write>(
    buffer: &mut T,
    map: &Coremap<Data, Data>,
    model_code: u8,
) -> IoResult<()> {
    let mut buffer = BufWriter::new(buffer);
    super::se::raw_serialize_map(map, &mut buffer, model_code)?;
    buffer.flush()?;
    Ok(())
}

pub fn serialize_partmap_into_slow_buffer<T: Write>(buffer: &mut T, ks: &Keyspace) -> IoResult<()> {
    let mut buffer = BufWriter::new(buffer);
    super::se::raw_serialize_partmap(&mut buffer, ks)?;
    buffer.flush()?;
    Ok(())
}

pub fn serialize_preload_into_slow_buffer<T: Write>(
    buffer: &mut T,
    store: &Memstore,
) -> IoResult<()> {
    let mut buffer = BufWriter::new(buffer);
    super::preload::raw_generate_preload(&mut buffer, store)?;
    buffer.flush()?;
    Ok(())
}
