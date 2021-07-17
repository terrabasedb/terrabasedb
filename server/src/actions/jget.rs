/*
 * Created on Mon Aug 31 2020
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
#![allow(dead_code)]

//! #`JGET` queries
//! Functions for handling `JGET` queries

use crate::dbnet::connection::prelude::*;
use crate::queryengine::ActionIter;

/// Run a `JGET` query
/// This returns a JSON key/value pair of keys and values
/// We need to write something like
/// ```json
/// &1\n
/// $15\n
/// {"key":"value"}\n
/// ```
///
pub async fn jget<T, Strm>(
    _handle: &crate::coredb::CoreDB,
    con: &mut T,
    act: ActionIter,
) -> std::io::Result<()>
where
    T: ProtocolConnectionExt<Strm>,
    Strm: AsyncReadExt + AsyncWriteExt + Unpin + Send + Sync,
{
    err_if_len_is!(act, con, not 1);
    todo!()
}

mod json {
    use crate::util::Unwrappable;
    use bytes::Bytes;
    pub struct BuiltJSON(Vec<u8>);
    pub struct JSONBlob(Vec<u8>);
    impl JSONBlob {
        pub fn new(size: usize) -> Self {
            let mut jblob = Vec::with_capacity(1 + size);
            jblob.push(b'{');
            JSONBlob(jblob)
        }
        pub fn insert(&mut self, key: &str, value: Option<&Bytes>) {
            self.0.push(b'"');
            self.0.extend(key.as_bytes());
            self.0.extend(b"\":");
            if let Some(value) = value {
                self.0.push(b'"');
                self.0.extend(value);
                self.0.push(b'"');
            } else {
                self.0.extend(b"null");
            }
            self.0.push(b',');
        }
        pub fn finish(mut self) -> BuiltJSON {
            *unsafe {
                // UNSAFE(@ohsayan): There will always be a value corresponding to last_mut
                self.0.last_mut().unsafe_unwrap()
            } = b'}';
            BuiltJSON(self.0)
        }
    }
    #[test]
    fn test_buildjson() {
        let mut jblob = JSONBlob::new(128);
        jblob.insert(&"key".to_owned(), Some(&Bytes::from("value".as_bytes())));
        jblob.insert(&"key2".to_owned(), None);
        assert_eq!(
            "{\"key\":\"value\",\"key2\":null}",
            String::from_utf8_lossy(&jblob.finish().0)
        );
    }
}
