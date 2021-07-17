/*
 * Created on Wed Aug 05 2020
 *
 * This file is a part of Skytable
 * Skytable (formerly known as TerrabaseDB or Skybase) is a free and open-source
 * NoSQL database written by Sayan Nandan ("the Author") with the
 * vision to provide flexibility in data modelling without compromising
 * on performance, queryability or scalability.
 *
 * Copyright (c) 2020, Sayan Nandan <ohsayan@outlook.com>
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

//! This module provides tools for handling persistently stored data

use crate::coredb::htable::Coremap;
use crate::coredb::htable::HTable;
use crate::coredb::lazy::Lazy;
use crate::coredb::Data;
use crate::diskstore::snapshot::DIR_SNAPSHOT;
use libsky::TResult;
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::Path;
use std::path::PathBuf;
pub mod flock;
pub mod snapshot;
mod snapstore;

pub static PERSIST_FILE: Lazy<PathBuf, fn() -> PathBuf> =
    Lazy::new(|| PathBuf::from("./data/data.bin"));

fn get_snapshot(path: String) -> TResult<Option<HTable<Data, Data>>> {
    // the path just has the snapshot name, let's improve that
    let mut snap_location = PathBuf::from(DIR_SNAPSHOT);
    snap_location.push(&path);
    let file = match fs::read(snap_location) {
        Ok(f) => f,
        Err(e) => return Err(e.into()),
    };
    let parsed = deserialize(file)?;
    Ok(Some(parsed))
}

/// Try to get the saved data from disk. This returns `None`, if the `data/data.bin` wasn't found
/// otherwise the `data/data.bin` file is deserialized and parsed into a `HTable`
pub fn get_saved(path: Option<String>) -> TResult<Option<HTable<Data, Data>>> {
    if let Some(path) = path {
        get_snapshot(path)
    } else {
        let file = match fs::read(&*PERSIST_FILE) {
            Ok(f) => f,
            Err(e) => match e.kind() {
                ErrorKind::NotFound => {
                    return Ok(None);
                }
                _ => return Err(format!("Couldn't read flushed data from disk: {}", e).into()),
            },
        };
        let parsed = deserialize(file)?;
        Ok(Some(parsed))
    }
}

#[cfg(test)]
impl PartialEq for HTable<Data, Data> {
    fn eq(&self, other: &HTable<Data, Data>) -> bool {
        other.iter().all(|key| self.contains_key(key.key())) && (other.len() == self.len())
    }
}

#[cfg(test)]
pub fn test_deserialize(file: Vec<u8>) -> TResult<HTable<Data, Data>> {
    deserialize(file)
}
fn deserialize(file: Vec<u8>) -> TResult<HTable<Data, Data>> {
    let parsed = Coremap::deserialize(file)?;
    Ok(HTable::from_raw(parsed))
}

/// Flush the in-memory table onto disk
///
/// This functions takes the entire in-memory table and writes it to the disk,
/// to the provided file behind the [`FileLock`]. This method will **automatically fsync**. You
/// do not need to explicitly fsync unless you'd like to waste CPU time
pub fn flush_data(file: &mut flock::FileLock, data: &Coremap<Data, Data>) -> TResult<()> {
    let encoded = data.serialize()?;
    file.write(&encoded)?;
    file.fsync()?;
    Ok(())
}

/// This function will write serialized data to disk and will **automatically fsync**. You
/// do not need to explicitly fsync unless you'd like to waste CPU time
pub fn write_to_disk(file: &Path, data: &Coremap<Data, Data>) -> TResult<()> {
    let mut file = fs::File::create(&file)?;
    let encoded = data.serialize()?;
    file.write_all(&encoded)?;
    file.sync_all()?;
    Ok(())
}
