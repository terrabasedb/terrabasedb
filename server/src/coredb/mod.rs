/*
 * Created on Mon Jul 13 2020
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

//! # The core database engine

use crate::config::SnapshotConfig;
use crate::config::SnapshotPref;
use crate::coredb::htable::HTable;
use crate::dbnet::connection::prelude::*;
use crate::diskstore;
use crate::protocol::Query;
use crate::queryengine;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;
pub use htable::Data;
use libsky::TResult;
use std::sync::Arc;
pub mod array;
pub mod buffers;
pub mod htable;
pub mod iarray;
pub mod lazy;
pub mod lock;
pub mod memstore;
pub mod table;

/// This is a thread-safe database handle, which on cloning simply
/// gives another atomic reference to the `shared` which is a `Shared` object
#[derive(Debug, Clone)]
pub struct CoreDB {
    /// The shared object, which contains a `Shared` object wrapped in an atomic RC
    pub shared: Arc<Shared>,
    /// The actual in-memory hashtable
    pub coremap: HTable<Data, Data>,
}

/// A shared _state_
#[derive(Debug)]
pub struct Shared {
    /// Whether the database is poisoned or not
    ///
    /// If the database is poisoned -> the database can no longer accept writes
    /// but can only accept reads
    pub poisoned: AtomicBool,
    /// The number of snapshots that are to be kept at the most
    ///
    /// If this is set to Some(0), then all the snapshots will be kept. Otherwise, if it is set to
    /// Some(n), n ∈ Z<sup>+</sup> — then _n_ snapshots will be kept at the maximum. If set to `None`, snapshotting is disabled.
    pub snapcfg: Option<SnapshotStatus>,
}

/// The status and details of the snapshotting service
///
/// The in_progress field is kept behind a mutex to ensure only one snapshot
/// operation can run at a time. Although on the server side this isn't a problem
/// because we don't have multiple snapshot tasks, but can be an issue when external
/// snapshots are triggered, for example via `MKSNAP`
#[derive(Debug)]
pub struct SnapshotStatus {
    /// The maximum number of recent snapshots to keep
    pub max: usize,
    /// The current state of the snapshot service
    pub in_progress: lock::QuickLock<()>,
}

impl SnapshotStatus {
    /// Create a new `SnapshotStatus` instance with preset values
    pub fn new(max: usize) -> Self {
        SnapshotStatus {
            max,
            in_progress: lock::QuickLock::new(()),
        }
    }

    /// Lock the snapshot service
    pub fn lock_snap(&self) -> lock::QLGuard<'_, ()> {
        self.in_progress.lock()
    }

    /// Check if the snapshot service is busy
    pub fn is_busy(&self) -> bool {
        self.in_progress.is_locked()
    }
}

impl CoreDB {
    pub fn poison(&self) {
        self.shared.poisoned.store(true, Ordering::Release);
    }

    pub fn unpoison(&self) {
        self.shared.poisoned.store(false, Ordering::Release);
    }
    /// Check if snapshotting is enabled
    pub fn is_snapshot_enabled(&self) -> bool {
        self.shared.snapcfg.is_some()
    }

    /// Mark the snapshotting service to be busy
    ///
    /// ## Panics
    /// If snapshotting is disabled, this will panic
    pub fn lock_snap(&self) -> lock::QLGuard<'_, ()> {
        self.shared.snapcfg.as_ref().unwrap().lock_snap()
    }

    /// Execute a query that has already been validated by `Connection::read_query`
    pub async fn execute_query<T, Strm>(&self, query: Query, con: &mut T) -> TResult<()>
    where
        T: ProtocolConnectionExt<Strm>,
        Strm: AsyncReadExt + AsyncWriteExt + Unpin + Send + Sync,
    {
        match query {
            Query::SimpleQuery(q) => {
                con.write_simple_query_header().await?;
                queryengine::execute_simple(self, con, q).await?;
                con.flush_stream().await?;
            }
            // TODO(@ohsayan): Pipeline commands haven't been implemented yet
            Query::PipelinedQuery(_) => unimplemented!(),
        }
        Ok(())
    }

    /// Create a new `CoreDB` instance
    ///
    /// This also checks if a local backup of previously saved data is available.
    /// If it is - it restores the data. Otherwise it creates a new in-memory table
    pub fn new(snapshot_cfg: &SnapshotConfig, restore_file: Option<String>) -> TResult<Self> {
        let coremap = diskstore::get_saved(restore_file)?;
        let mut snap_count = None;
        if let SnapshotConfig::Enabled(SnapshotPref {
            every: _, atmost, ..
        }) = snapshot_cfg
        {
            snap_count = Some(atmost);
        }
        let snapcfg =
            option_unwrap_or!(snap_count.map(|max| Some(SnapshotStatus::new(*max))), None);
        let db = if let Some(coremap) = coremap {
            CoreDB {
                coremap,
                shared: Arc::new(Shared {
                    snapcfg,
                    poisoned: AtomicBool::new(false),
                }),
            }
        } else {
            CoreDB::new_empty(snapcfg)
        };
        Ok(db)
    }
    /// Create an empty in-memory table
    pub fn new_empty(snapcfg: Option<SnapshotStatus>) -> Self {
        CoreDB {
            coremap: HTable::new(),
            shared: Arc::new(Shared {
                poisoned: AtomicBool::new(false),
                snapcfg,
            }),
        }
    }
    /// Check if the database object is poisoned, that is, data couldn't be written
    /// to disk once, and hence, we have disabled write operations
    pub fn is_poisoned(&self) -> bool {
        self.shared.poisoned.load(Ordering::Acquire)
    }
    /// Provides a reference to the shared [`Coremap`] object
    pub fn get_ref(&self) -> &HTable<Data, Data> {
        &self.coremap
    }
}
