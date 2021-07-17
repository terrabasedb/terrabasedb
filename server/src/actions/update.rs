/*
 * Created on Mon Aug 17 2020
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

//! # `UPDATE` queries
//! This module provides functions to work with `UPDATE` queries
//!

use crate::coredb::{self};
use crate::dbnet::connection::prelude::*;
use crate::protocol::responses;
use crate::queryengine::ActionIter;
use coredb::Data;

/// Run an `UPDATE` query
pub async fn update<T, Strm>(
    handle: &crate::coredb::CoreDB,
    con: &mut T,
    mut act: ActionIter,
) -> std::io::Result<()>
where
    T: ProtocolConnectionExt<Strm>,
    Strm: AsyncReadExt + AsyncWriteExt + Unpin + Send + Sync,
{
    err_if_len_is!(act, con, not 2);
    let did_we = {
        if handle.is_poisoned() {
            None
        } else {
            let writer = handle.get_ref();
            // clippy thinks we're doing something complex when we aren't, at all!
            #[allow(clippy::blocks_in_if_conditions)]
            if unsafe {
                // UNSAFE(@ohsayan): This is completely safe as we've already checked
                // that there are exactly 2 arguments
                writer.true_if_update(
                    Data::from(act.next().unsafe_unwrap()),
                    Data::from(act.next().unsafe_unwrap()),
                )
            } {
                Some(true)
            } else {
                Some(false)
            }
        }
    };
    if let Some(did_we) = did_we {
        if did_we {
            con.write_response(responses::groups::OKAY).await?;
        } else {
            con.write_response(responses::groups::NIL).await?;
        }
    } else {
        con.write_response(responses::groups::SERVER_ERR).await?;
    }
    Ok(())
}
