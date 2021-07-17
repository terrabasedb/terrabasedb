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

//! # Unflush routines
//!
//! Routines for unflushing data

use crate::coredb::memstore::ObjectID;
use crate::coredb::table::Table;
use crate::storage::interface::DIR_KSROOT;
use crate::storage::Coremap;
use std::collections::HashMap;
use std::fs;
use std::io::Error as IoError;
use std::io::ErrorKind;
use std::io::Result as IoResult;
use std::sync::Arc;

/// Read a given table into a [`Table`] object
pub fn read_table(ksid: &ObjectID, tblid: &ObjectID, volatile: bool) -> IoResult<Table> {
    let filepath = unsafe { concat_path!(DIR_KSROOT, ksid.as_str(), tblid.as_str()) };
    let read = fs::read(filepath)?;
    // TODO(@ohsayan): Mod this to de based on data model
    let (data, model) = super::de::deserialize_map(read).ok_or_else(|| bad_data!())?;
    match model {
        0 => {
            // kve
            Table::kve_from_model_code_and_data(model, volatile, data).ok_or_else(|| bad_data!())
        }
        _ => {
            // some model that we don't know
            Err(IoError::from(ErrorKind::Unsupported))
        }
    }
}

/// Read an entire keyspace into a Coremap. You'll need to initialize the rest
pub fn read_keyspace(ksid: ObjectID) -> IoResult<Coremap<ObjectID, Arc<Table>>> {
    let filepath = unsafe { concat_path!(DIR_KSROOT, ksid.as_str(), "PARTMAP") };
    let partmap: HashMap<ObjectID, u8> =
        super::de::deserialize_set_ctype_bytemark(&fs::read(filepath)?)
            .ok_or_else(|| bad_data!())?;
    let ks: Coremap<ObjectID, Arc<Table>> = Coremap::with_capacity(partmap.len());
    for (tableid, table_storage_type) in partmap.into_iter() {
        if table_storage_type > 1 {
            return Err(bad_data!());
        }
        let is_volatile = table_storage_type == 1;
        let tbl = self::read_table(&ksid, &tableid, is_volatile)?;
        ks.true_if_insert(tableid, Arc::new(tbl));
    }
    Ok(ks)
}
