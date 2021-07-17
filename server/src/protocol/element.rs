/*
 * Created on Tue May 11 2021
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

use bytes::Bytes;

#[non_exhaustive]
#[derive(Debug, PartialEq)]
/// # Data Types
///
/// This enum represents the data types supported by the Skyhash Protocol
pub enum Element {
    /// Arrays can be nested! Their `<tsymbol>` is `&`
    Array(Vec<Element>),
    /// A String value; `<tsymbol>` is `+`
    String(String),
    /// An unsigned integer value; `<tsymbol>` is `:`
    UnsignedInt(u64),
    /// A non-recursive String array; tsymbol: `_`
    FlatArray(Vec<String>),
    /// Swap the KS (ASCII `1A` (SUB HEADER))
    SwapKSHeader(Bytes),
    /// Swap the NS (ASCII `0x1B` (ESC HEADER))
    SwapNSHeader(Bytes),
}
