/*
 * Created on Thu Aug 27 2020
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

use crate::coredb::Data;
use crate::dbnet::connection::prelude::*;
use crate::protocol::responses;
use crate::queryengine::ActionIter;

/// Run an `MUPDATE` query
pub async fn mupdate<T, Strm>(
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
        // An odd number of arguments means that the number of keys
        // is not the same as the number of values, we won't run this
        // action at all
        return con.write_response(responses::groups::ACTION_ERR).await;
    }
    let done_howmany: Option<usize>;
    {
        if handle.is_poisoned() {
            done_howmany = None;
        } else {
            let writer = handle.get_ref();
            let mut didmany = 0;
            while let (Some(key), Some(val)) = (act.next(), act.next()) {
                if writer.true_if_update(Data::from(key), Data::from(val)) {
                    didmany += 1;
                }
            }
            done_howmany = Some(didmany);
        }
    }
    if let Some(done_howmany) = done_howmany {
        return con.write_response(done_howmany as usize).await;
    } else {
        return con.write_response(responses::groups::SERVER_ERR).await;
    }
}
