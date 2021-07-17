/*
 * Created on Mon Jun 14 2021
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

use crate::coredb;
use crate::dbnet::connection::prelude::*;
use crate::protocol::responses;
use crate::queryengine::ActionIter;
use crate::resp::BytesWrapper;

/// Run a POP action
pub async fn pop<T, Strm>(
    handle: &coredb::CoreDB,
    con: &mut T,
    act: ActionIter,
) -> std::io::Result<()>
where
    T: ProtocolConnectionExt<Strm>,
    Strm: AsyncReadExt + AsyncWriteExt + Unpin + Send + Sync,
{
    err_if_len_is!(act, con, eq 0);
    if handle.is_poisoned() {
        // don't begin the operation at all if the database is poisoned
        return con.write_response(responses::groups::SERVER_ERR).await;
    }
    con.write_array_length(act.len()).await?;
    for key in act {
        if handle.is_poisoned() {
            // we keep this check just in case the server fails in-between running a
            // pop operation
            con.write_response(responses::groups::SERVER_ERR).await?;
        } else if let Some((_key, val)) = handle.get_ref().remove(key.as_bytes()) {
            con.write_response(BytesWrapper(val.into_inner())).await?;
        } else {
            con.write_response(responses::groups::NIL).await?;
        }
    }
    Ok(())
}
