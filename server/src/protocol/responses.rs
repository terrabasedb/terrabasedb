/*
 * Created on Sat Aug 22 2020
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

//! Primitives for generating Skyhash compatible responses

pub mod groups {
    #![allow(unused)]
    //! # Pre-compiled response **elements**
    //! These are pre-compiled response groups and **not** complete responses. If complete
    //! responses are required, user protocol::responses::fresp
    /// Response code 0 as a datagroup element
    pub const OKAY: &[u8] = "!1\n0\n".as_bytes();
    /// Response code 1 as a datagroup element
    pub const NIL: &[u8] = "!1\n1\n".as_bytes();
    /// Response code 2 as a datagroup element
    pub const OVERWRITE_ERR: &[u8] = "!1\n2\n".as_bytes();
    /// Response code 3 as a datagroup element
    pub const ACTION_ERR: &[u8] = "!1\n3\n".as_bytes();
    /// Response code 4 as a datagroup element
    pub const PACKET_ERR: &[u8] = "!1\n4\n".as_bytes();
    /// Response code 5 as a datagroup element
    pub const SERVER_ERR: &[u8] = "!1\n5\n".as_bytes();
    /// Response code 6 as a datagroup element
    pub const OTHER_ERR_EMPTY: &[u8] = "!1\n6\n".as_bytes();
    /// Response group element with string "HEYA"
    pub const HEYA: &[u8] = "+4\nHEY!\n".as_bytes();
    /// "Unknown action" error response
    pub const UNKNOWN_ACTION: &[u8] = "!14\nUnknown action\n".as_bytes();
    pub const WRONGTYPE_ERR: &[u8] = "!1\n7\n".as_bytes();
    pub const SNAPSHOT_BUSY: &[u8] = "!17\nerr-snapshot-busy\n".as_bytes();
    /// Snapshot disabled (other error)
    pub const SNAPSHOT_DISABLED: &[u8] = "!21\nerr-snapshot-disabled\n".as_bytes();
    /// Snapshot has illegal name (other error)
    pub const SNAPSHOT_ILLEGAL_NAME: &[u8] = "!25\nerr-invalid-snapshot-name\n".as_bytes();
    /// Access after termination signal (other error)
    pub const ERR_ACCESS_AFTER_TERMSIG: &[u8] = "!24\nerr-access-after-termsig\n".as_bytes();
}

pub mod full_responses {
    #![allow(unused)]
    //! # Pre-compiled **responses**
    //! These are pre-compiled **complete** responses. This means that they should
    //! be written off directly to the stream and should **not be preceded by any response metaframe**

    /// Response code: 0 (Okay)
    pub const R_OKAY: &[u8] = "*1\n!1\n0\n".as_bytes();
    /// Response code: 1 (Nil)
    pub const R_NIL: &[u8] = "*1\n!1\n1\n".as_bytes();
    /// Response code: 2 (Overwrite Error)
    pub const R_OVERWRITE_ERR: &[u8] = "*1\n!1\n2\n".as_bytes();
    /// Response code: 3 (Action Error)
    pub const R_ACTION_ERR: &[u8] = "*1\n!1\n3\n".as_bytes();
    /// Response code: 4 (Packet Error)
    pub const R_PACKET_ERR: &[u8] = "*1\n!1\n4\n".as_bytes();
    /// Response code: 5 (Server Error)
    pub const R_SERVER_ERR: &[u8] = "*1\n!1\n5\n".as_bytes();
    /// Response code: 6 (Other Error _without description_)
    pub const R_OTHER_ERR_EMPTY: &[u8] = "*1\n!1\n6\n".as_bytes();
    /// A heya response
    pub const R_HEYA: &[u8] = "*1\n+4\nHEY!\n".as_bytes();
    /// An other response with description: "Unknown action"
    pub const R_UNKNOWN_ACTION: &[u8] = "*1\n!14\nUnknown action\n".as_bytes();
    /// A 0 uint64 reply
    pub const R_ONE_INT_REPLY: &[u8] = "*1\n:1\n1\n".as_bytes();
    /// A 1 uint64 reply
    pub const R_ZERO_INT_REPLY: &[u8] = "*1\n:1\n0\n".as_bytes();
    /// Snapshot busy (other error)
    pub const R_SNAPSHOT_BUSY: &[u8] = "*1\n!17\nerr-snapshot-busy\n".as_bytes();
    /// Snapshot disabled (other error)
    pub const R_SNAPSHOT_DISABLED: &[u8] = "*1\n!21\nerr-snapshot-disabled\n".as_bytes();
    /// Snapshot has illegal name (other error)
    pub const R_SNAPSHOT_ILLEGAL_NAME: &[u8] = "*1\n!25\nerr-invalid-snapshot-name\n".as_bytes();
    /// Access after termination signal (other error)
    pub const R_ERR_ACCESS_AFTER_TERMSIG: &[u8] = "*1\n!24\nerr-access-after-termsig\n".as_bytes();
    /// Response code: 7; wrongtype
    pub const R_WRONGTYPE_ERR: &[u8] = "*1\n!1\n7".as_bytes();
}
