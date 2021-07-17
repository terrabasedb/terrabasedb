/*
 * Created on Tue Oct 13 2020
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

use crate::dbnet::connection::prelude::*;
use crate::diskstore;
use crate::diskstore::snapshot::SnapshotEngine;
use crate::diskstore::snapshot::DIR_SNAPSHOT;
use crate::protocol::responses;
use crate::queryengine::ActionIter;
use std::path::{Component, PathBuf};

/// Create a snapshot
///
pub async fn mksnap<T, Strm>(
    handle: &crate::coredb::CoreDB,
    con: &mut T,
    mut act: ActionIter,
) -> std::io::Result<()>
where
    T: ProtocolConnectionExt<Strm>,
    Strm: AsyncReadExt + AsyncWriteExt + Unpin + Send + Sync,
{
    if act.len() == 0 {
        if !handle.is_snapshot_enabled() {
            // Since snapshotting is disabled, we can't create a snapshot!
            // We'll just return an error returning the same
            return con
                .write_response(responses::groups::SNAPSHOT_DISABLED)
                .await;
        }
        // We will just follow the standard convention of creating snapshots
        let mut was_engine_error = false;
        let mut succeeded = None;

        let snaphandle = handle.shared.clone();
        let snapstatus = unsafe {
            // UNSAFE(@ohsayan) This is safe as we've already checked
            // if snapshots are enabled or not with `is_snapshot_enabled`
            snaphandle.snapcfg.as_ref().unsafe_unwrap()
        };
        let snapengine = SnapshotEngine::new(snapstatus.max, handle, None);
        if snapengine.is_err() {
            was_engine_error = true;
        } else if snapstatus.is_busy() {
            succeeded = None;
        } else {
            let snapengine = unsafe {
                // UNSAFE(@ohsayan) This is safe as we've already checked
                // if snapshots are enabled or not with `is_snapshot_enabled`
                snapengine.unsafe_unwrap()
            };
            succeeded = Some(snapengine);
        }
        if was_engine_error {
            return con
                .write_response(responses::groups::SERVER_ERR.to_owned())
                .await;
        }
        if let Some(mut succeeded) = succeeded {
            let succeeded = succeeded.mksnap().await;
            if succeeded {
                // Snapshotting succeeded, return Okay
                return con.write_response(responses::groups::OKAY.to_owned()).await;
            } else {
                // Nope, something happened while creating a snapshot
                // return a server error
                return con
                    .write_response(responses::groups::SERVER_ERR.to_owned())
                    .await;
            }
        } else {
            return con
                .write_response(responses::groups::SNAPSHOT_BUSY)
                .await;
        }
    } else if act.len() == 1 {
        // This means that the user wants to create a 'named' snapshot
        let snapname = unsafe {
            // UNSAFE(@ohsayan): We've already checked that the action
            // contains a second argument, so this can't be reached
            act.next().unsafe_unwrap()
        };
        let mut path = PathBuf::from(DIR_SNAPSHOT);
        path.push("remote");
        path.push(snapname.to_owned() + ".snapshot");
        let illegal_snapshot = path
            .components()
            .filter(|dir| {
                // Sanitize snapshot name, to avoid directory traversal attacks
                // If the snapshot name has any root directory or parent directory, then
                // we'll allow it to pass through this adaptor.
                // As a result, this iterator will give us a count of the 'bad' components
                dir == &Component::RootDir || dir == &Component::ParentDir
            })
            .count()
            != 0;
        if illegal_snapshot {
            return con
                .write_response(responses::groups::SNAPSHOT_ILLEGAL_NAME)
                .await;
        }
        let failed;
        {
            let tbl_ref = handle.get_ref();
            match diskstore::write_to_disk(&path, &*tbl_ref) {
                Ok(_) => failed = false,
                Err(e) => {
                    log::error!("Error while creating snapshot: {}", e);
                    failed = true;
                }
            }
            // end of table lock state critical section
        }
        if failed {
            return con
                .write_response(responses::groups::SERVER_ERR.to_owned())
                .await;
        } else {
            return con.write_response(responses::groups::OKAY.to_owned()).await;
        }
    } else {
        return con
            .write_response(responses::groups::ACTION_ERR.to_owned())
            .await;
    }
}
