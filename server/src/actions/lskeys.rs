/*
 * Created on Thu May 13 2021
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

use crate::dbnet::connection::prelude::*;
use crate::protocol::responses;
use crate::queryengine::ActionIter;
use crate::resp::BytesWrapper;
use bytes::Bytes;

/// Run an `LSKEYS` query
pub async fn lskeys<T, Strm>(
    handle: &crate::coredb::CoreDB,
    con: &mut T,
    mut act: ActionIter,
) -> std::io::Result<()>
where
    T: ProtocolConnectionExt<Strm>,
    Strm: AsyncReadExt + AsyncWriteExt + Unpin + Send + Sync,
{
    err_if_len_is!(act, con, gt 1);
    let item_count = if let Some(cnt) = act.next() {
        if let Ok(cnt) = cnt.parse::<usize>() {
            cnt
        } else {
            return con
                .write_response(responses::groups::WRONGTYPE_ERR)
                .await;
        }
    } else {
        10
    };
    let items: Vec<Bytes>;
    {
        let reader = handle.get_ref();
        items = reader.get_keys(item_count);
    }
    con.write_flat_array_length(items.len()).await?;
    for item in items {
        con.write_response(BytesWrapper(item)).await?;
    }
    Ok(())
}
