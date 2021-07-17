/*
 * Created on Sat Jul 17 2021
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

//! # Flush routines
//!
//! This module contains multiple flush routines: at the memstore level, the keyspace level and
//! the table level

use super::interface;
use crate::coredb::memstore::Keyspace;
use crate::coredb::memstore::Memstore;
use crate::coredb::memstore::ObjectID;
use std::io::Result as IoResult;

/// Flushes the entire **keyspace + partmap**
pub fn flush_keyspace_full(ksid: &ObjectID, keyspace: &Keyspace) -> IoResult<()> {
    self::oneshot::flush_partmap(ksid, keyspace)?;
    self::oneshot::flush_keyspace(ksid, keyspace)
}

/// Flush the entire **preload + keyspaces + their partmaps**
pub fn flush_full(store: &Memstore) -> IoResult<()> {
    self::oneshot::flush_preload(store)?;
    for keyspace in store.keyspaces.iter() {
        self::flush_keyspace_full(keyspace.key(), keyspace.value())?;
    }
    Ok(())
}

pub mod oneshot {
    //! # Irresponsible flushing
    //!
    //! Every function does **exactly what it says** and nothing more. No partition
    //! files et al are handled
    //!
    use super::*;
    use crate::coredb::table::{DataModel, Table};
    use crate::storage::interface::DIR_KSROOT;
    use std::fs::{self, File};

    const PRELOAD_FILE_PATH_TEMP: &str = "data/ks/PRELOAD_";
    const PRELOAD_FILE_PATH_TEMP_LEN_CHOP: usize = PRELOAD_FILE_PATH_TEMP.len() - 1;

    macro_rules! tbl_path {
        ($ksid:expr, $tableid:expr) => {
            unsafe { concat_str!(DIR_KSROOT, "/", $ksid.as_str(), "/", $tableid.as_str()) }
        };
    }

    /// No `partmap` handling. Just flushes the table to the expected location
    pub fn flush_table(tableid: &ObjectID, ksid: &ObjectID, table: &Table) -> IoResult<()> {
        let path = tbl_path!(tableid, ksid);
        let mut file = File::create(&path)?;
        let modelcode = table.get_model_code();
        match table.get_model_ref() {
            DataModel::KV(kve) => super::interface::serialize_map_into_slow_buffer(
                &mut file,
                kve.__get_inner_ref(),
                modelcode,
            )?,
        }
        file.sync_all()?;
        fs::rename(&path, &path[..path.len() - 1])
    }

    /// Flushes an entire keyspace to the expected location. No `partmap` or `preload` handling
    pub fn flush_keyspace(ksid: &ObjectID, keyspace: &Keyspace) -> IoResult<()> {
        for table in keyspace.tables.iter() {
            self::flush_table(table.key(), &ksid, table.value())?;
        }
        Ok(())
    }

    /// Flushes a single partmap
    pub fn flush_partmap(ksid: &ObjectID, keyspace: &Keyspace) -> IoResult<()> {
        let path = unsafe { concat_str!(DIR_KSROOT, "/", ksid.as_str(), "/", "PARTMAP_") };
        let mut file = File::create(&path)?;
        super::interface::serialize_partmap_into_slow_buffer(&mut file, keyspace)?;
        file.sync_all()?;
        fs::rename(&path, &path[..path.len() - 1])?;
        Ok(())
    }

    /// Flushes everything in memory. No `partmap` or `preload` handling
    pub fn flush(store: &Memstore) -> IoResult<()> {
        for keyspace in store.keyspaces.iter() {
            self::flush_keyspace(keyspace.key(), keyspace.value())?;
        }
        Ok(())
    }

    // Flush the `PRELOAD`
    pub fn flush_preload(store: &Memstore) -> IoResult<()> {
        let mut file = File::create(PRELOAD_FILE_PATH_TEMP)?;
        super::interface::serialize_preload_into_slow_buffer(&mut file, store)?;
        file.sync_all()?;
        fs::rename(
            &PRELOAD_FILE_PATH_TEMP,
            &PRELOAD_FILE_PATH_TEMP[..PRELOAD_FILE_PATH_TEMP_LEN_CHOP],
        )?;
        Ok(())
    }
}
