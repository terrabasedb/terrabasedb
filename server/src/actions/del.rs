/*
 * Created on Wed Aug 19 2020
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

//! # `DEL` queries
//! This module provides functions to work with `DEL` queries

use crate::dbnet::connection::prelude::*;
use crate::protocol::responses;
use crate::queryengine::ActionIter;

/// Run a `DEL` query
///
/// Do note that this function is blocking since it acquires a write lock.
/// It will write an entire datagroup, for this `del` action
pub async fn del<T, Strm>(
    handle: &crate::coredb::CoreDB,
    con: &mut T,
    act: ActionIter,
) -> std::io::Result<()>
where
    T: ProtocolConnectionExt<Strm>,
    Strm: AsyncReadExt + AsyncWriteExt + Unpin + Send + Sync,
{
    err_if_len_is!(act, con, eq 0);
    let done_howmany: Option<usize>;
    {
        if handle.is_poisoned() {
            done_howmany = None;
        } else {
            let mut many = 0;
            let cmap = handle.get_ref();
            act.for_each(|key| {
                if cmap.true_if_removed(key.as_bytes()) {
                    many += 1
                }
            });
            done_howmany = Some(many);
        }
    }
    if let Some(done_howmany) = done_howmany {
        con.write_response(done_howmany).await
    } else {
        con.write_response(responses::groups::SERVER_ERR).await
    }
}
