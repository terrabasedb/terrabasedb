/*
 * Created on Mon Sep 21 2020
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

//! # Strong Actions
//! Strong actions are like "do all" or "fail all" actions, built specifically for
//! multiple keys. So let's say you used `SSET` instead of `MSET` for setting keys:
//! what'd be the difference?
//! In this case, if all the keys are non-existing, which is a requirement for `MSET`,
//! only then would the keys be set. That is, only if all the keys can be set, will the action
//! run and return code `0` - otherwise the action won't do anything and return an overwrite error.
//! There is no point of using _strong actions_ for a single key/value pair, since it will only
//! slow things down due to the checks performed.
//! Do note that this isn't the same as the gurantees provided by ACID transactions

use crate::coredb::Data;
use crate::dbnet::connection::prelude::*;
use crate::protocol::responses;
use crate::queryengine::ActionIter;
use core::hint::unreachable_unchecked;

/// Run an `SSET` query
///
/// This either returns `Okay` if all the keys were set, or it returns an
/// `Overwrite Error` or code `2`
pub async fn sset<T, Strm>(
    handle: &crate::coredb::CoreDB,
    con: &mut T,
    mut act: ActionIter,
) -> std::io::Result<()>
where
    T: ProtocolConnectionExt<Strm>,
    Strm: AsyncReadExt + AsyncWriteExt + Unpin + Send + Sync,
{
    let howmany = act.len();
    if is_lowbit_set!(howmany) || howmany == 0 {
        return con.write_response(responses::groups::ACTION_ERR).await;
    }
    let failed;
    {
        // We use this additional scope to tell the compiler that the write lock
        // doesn't go beyond the scope of this function - and is never used across
        // an await: cause, the compiler ain't as smart as we are ;)

        // This iterator gives us the keys and values, skipping the first argument which
        // is the action name
        let mut key_iter = act.as_ref().iter();
        if handle.is_poisoned() {
            failed = None;
        } else {
            let mut_table = handle.get_ref();
            if key_iter.all(|key| !mut_table.contains_key(key.as_bytes())) {
                failed = Some(false);
                // Since the failed flag is false, none of the keys existed
                // So we can safely set the keys
                while let (Some(key), Some(value)) = (act.next(), act.next()) {
                    if !mut_table.true_if_insert(Data::from(key), Data::from_string(value)) {
                        // Tell the compiler that this will never be the case
                        unsafe {
                            // UNSAFE(@ohsayan): As none of the keys exist in the table, no
                            // value will ever be returned by the `insert`. Hence, this is a
                            // completely safe operation
                            unreachable_unchecked()
                        }
                    }
                }
            } else {
                failed = Some(true);
            }
        }
    }
    if let Some(failed) = failed {
        if failed {
            con.write_response(responses::groups::OVERWRITE_ERR)
                .await
        } else {
            con.write_response(responses::groups::OKAY).await
        }
    } else {
        con.write_response(responses::groups::SERVER_ERR).await
    }
}

/// Run an `SDEL` query
///
/// This either returns `Okay` if all the keys were `del`eted, or it returns a
/// `Nil`, which is code `1`
pub async fn sdel<T, Strm>(
    handle: &crate::coredb::CoreDB,
    con: &mut T,
    act: ActionIter,
) -> std::io::Result<()>
where
    T: ProtocolConnectionExt<Strm>,
    Strm: AsyncReadExt + AsyncWriteExt + Unpin + Send + Sync,
{
    let howmany = act.len();
    if howmany == 0 {
        return con.write_response(responses::groups::ACTION_ERR).await;
    }
    let failed;
    {
        // We use this additional scope to tell the compiler that the write lock
        // doesn't go beyond the scope of this function - and is never used across
        // an await: cause, the compiler ain't as smart as we are ;)
        let mut key_iter = act.as_ref().iter();
        if handle.is_poisoned() {
            failed = None;
        } else {
            let mut_table = handle.get_ref();
            if key_iter.all(|key| mut_table.contains_key(key.as_bytes())) {
                failed = Some(false);
                // Since the failed flag is false, all of the keys exist
                // So we can safely delete the keys
                act.into_iter().for_each(|key| {
                    // Since we've already checked that the keys don't exist
                    // We'll tell the compiler to optimize this
                    unsafe {
                        // UNSAFE(@ohsayan): Since all the values exist, all of them will return
                        // some value. Hence, this branch won't ever be reached. Hence, this is safe.
                        let _ = mut_table.remove(key.as_bytes()).unsafe_unwrap();
                    }
                });
            } else {
                failed = Some(true);
            }
        }
    }
    if let Some(failed) = failed {
        if failed {
            con.write_response(responses::groups::NIL).await
        } else {
            con.write_response(responses::groups::OKAY).await
        }
    } else {
        con.write_response(responses::groups::SERVER_ERR).await
    }
}

/// Run an `SUPDATE` query
///
/// This either returns `Okay` if all the keys were updated, or it returns `Nil`
/// or code `1`
pub async fn supdate<T, Strm>(
    handle: &crate::coredb::CoreDB,
    con: &mut T,
    mut act: ActionIter,
) -> std::io::Result<()>
where
    T: ProtocolConnectionExt<Strm>,
    Strm: AsyncReadExt + AsyncWriteExt + Unpin + Send + Sync,
{
    let howmany = act.len();
    if is_lowbit_set!(howmany) || howmany == 0 {
        return con.write_response(responses::groups::ACTION_ERR).await;
    }
    let mut failed = Some(false);
    {
        // We use this additional scope to tell the compiler that the write lock
        // doesn't go beyond the scope of this function - and is never used across
        // an await: cause, the compiler ain't as smart as we are ;)
        let mut key_iter = act.as_ref().iter();
        if handle.is_poisoned() {
            failed = None;
        } else {
            let mut_table = handle.get_ref();
            while let Some(key) = key_iter.next() {
                if !mut_table.contains_key(key.as_bytes()) {
                    // With one of the keys failing to exist - this action can't clearly be done
                    // So we'll set `failed` to true and ensure that we check this while
                    // writing a response back to the client
                    failed = Some(true);
                    break;
                }
                // Skip the next value that is coming our way, as we don't need it
                // right now
                unsafe {
                    let _ = key_iter.next().unsafe_unwrap();
                }
            }
            // clippy thinks we're doing something complex when we aren't, at all!
            #[allow(clippy::blocks_in_if_conditions)]
            if unsafe { !failed.unsafe_unwrap() } {
                // Since the failed flag is false, none of the keys existed
                // So we can safely update the keys
                while let (Some(key), Some(value)) = (act.next(), act.next()) {
                    if !mut_table.true_if_update(Data::from(key), Data::from_string(value)) {
                        // Tell the compiler that this will never be the case
                        unsafe { unreachable_unchecked() }
                    }
                }
            }
        }
    }
    if let Some(failed) = failed {
        if failed {
            con.write_response(responses::groups::NIL).await
        } else {
            con.write_response(responses::groups::OKAY).await
        }
    } else {
        con.write_response(responses::groups::SERVER_ERR).await
    }
}
