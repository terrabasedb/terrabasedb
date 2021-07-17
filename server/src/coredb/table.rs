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

#![allow(dead_code)] // TODO(@ohsayan): Remove this once we're done

use crate::coredb::htable::Coremap;
use crate::coredb::Data;
use crate::kvengine::KVEngine;

#[derive(Debug)]
pub enum DataModel {
    KV(KVEngine),
}

// same 8 byte ptrs; any chance of optimizations?

#[derive(Debug)]
/// The underlying table type. This is the place for the other data models (soon!)
pub struct Table {
    /// a key/value store
    model_store: DataModel,
    volatile: bool,
}

impl Table {
    /// Get the key/value store if the table is a key/value store
    pub const fn get_kvstore(&self) -> Option<&KVEngine> {
        #[allow(irrefutable_let_patterns)]
        if let DataModel::KV(kvs) = &self.model_store {
            Some(&kvs)
        } else {
            None
        }
    }
    pub const fn storage_type(&self) -> u8 {
        self.volatile as u8
    }
    pub fn kve_from_model_code_and_data(
        modelcode: u8,
        volatile: bool,
        data: Coremap<Data, Data>,
    ) -> Option<Self> {
        let data = match modelcode {
            0 => KVEngine::init_with_data(false, false, data),
            1 => KVEngine::init_with_data(false, true, data),
            2 => KVEngine::init_with_data(true, true, data),
            3 => KVEngine::init_with_data(true, false, data),
            _ => return None,
        };
        Some(Self {
            model_store: DataModel::KV(data),
            volatile,
        })
    }
    pub fn kve_from_model_code(modelcode: u8) -> Option<Self> {
        Self::kve_from_model_code_and_data(modelcode, false, Coremap::new())
    }
    pub fn new_default_kve() -> Self {
        match Self::kve_from_model_code(0) {
            Some(k) => k,
            None => unsafe { core::hint::unreachable_unchecked() },
        }
    }
    pub fn get_model_code(&self) -> u8 {
        match &self.model_store {
            DataModel::KV(kvs) => {
                /*
                bin,bin => 0
                bin,str => 1
                str,str => 2
                str,bin => 3
                */
                let (kbin, vbin) = kvs.get_encoding();
                if kbin {
                    if vbin {
                        // both k + v are str
                        2
                    } else {
                        // only k is str
                        3
                    }
                } else if vbin {
                    // k is bin, v is str
                    1
                } else {
                    // both are bin
                    0
                }
            }
        }
    }
    pub fn get_model_ref(&self) -> &DataModel {
        &self.model_store
    }
}
