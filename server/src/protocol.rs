/*
 * Created on Sat Jul 18 2020
 *
 * This file is a part of the source code for the Terrabase database
 * Copyright (c) 2020, Sayan Nandan <ohsayan at outlook dot com>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 *
*/

use corelib::terrapipe::{extract_idents, get_sizes, ActionType};
use corelib::terrapipe::{RespBytes, RespCodes, DEF_QMETALINE_BUFSIZE};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

/// The query dataframe
#[derive(Debug)]
pub struct QueryDataframe {
    /// The data part
    pub data: Vec<String>,
    /// The query action type
    pub actiontype: ActionType,
}

#[derive(Debug, PartialEq)]
pub struct PreQMF {
    action_type: ActionType,
    content_size: usize,
    metaline_size: usize,
}

impl PreQMF {
    pub fn from_buffer(buf: String) -> Result<Self, RespCodes> {
        let buf: Vec<&str> = buf.split('!').collect();
        if let (Some(atype), Some(csize), Some(metaline_size)) =
            (buf.get(0), buf.get(1), buf.get(2))
        {
            if let Some(atype) = atype.chars().next() {
                let atype = match atype {
                    '*' => ActionType::Simple,
                    '$' => ActionType::Pipeline,
                    _ => return Err(RespCodes::InvalidMetaframe),
                };
                let csize = csize.trim().trim_matches(char::from(0));
                let metaline_size = metaline_size.trim().trim_matches(char::from(0));
                if let (Ok(csize), Ok(metaline_size)) =
                    (csize.parse::<usize>(), metaline_size.parse::<usize>())
                {
                    return Ok(PreQMF {
                        action_type: atype,
                        content_size: csize,
                        metaline_size,
                    });
                }
            }
        }
        Err(RespCodes::InvalidMetaframe)
    }
}

#[cfg(test)]
#[test]
fn test_preqmf() {
    let read_what = "*!12!4".to_owned();
    let preqmf = PreQMF::from_buffer(read_what).unwrap();
    let pqmf_should_be = PreQMF {
        action_type: ActionType::Simple,
        content_size: 12,
        metaline_size: 4,
    };
    assert_eq!(pqmf_should_be, preqmf);
    let a_pipe = "$!12!4".to_owned();
    let preqmf = PreQMF::from_buffer(a_pipe).unwrap();
    let pqmf_should_be = PreQMF {
        action_type: ActionType::Pipeline,
        content_size: 12,
        metaline_size: 4,
    };
    assert_eq!(preqmf, pqmf_should_be);
}

pub struct Connection {
    stream: TcpStream,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        Connection { stream }
    }
    pub async fn read_query(&mut self) -> Result<QueryDataframe, RespCodes> {
        let mut bufreader = BufReader::new(&mut self.stream);
        let mut metaline_buf = String::with_capacity(DEF_QMETALINE_BUFSIZE);
        bufreader.read_line(&mut metaline_buf).await.unwrap();
        let pqmf = PreQMF::from_buffer(metaline_buf)?;
        let (mut metalayout_buf, mut dataframe_buf) = (
            String::with_capacity(pqmf.metaline_size),
            vec![0; pqmf.content_size],
        );
        bufreader.read_line(&mut metalayout_buf).await.unwrap();
        let ss = get_sizes(metalayout_buf)?;
        bufreader.read(&mut dataframe_buf).await.unwrap();
        let qdf = QueryDataframe {
            data: extract_idents(dataframe_buf, ss),
            actiontype: pqmf.action_type,
        };
        Ok(qdf)
    }
    pub async fn write_response(&mut self, resp: Vec<u8>) {
        if let Err(_) = self.stream.write_all(&resp).await {
            eprintln!(
                "Error while writing to stream: {:?}",
                self.stream.peer_addr()
            );
            return;
        }
        if let Err(_) = self.stream.flush().await {
            eprintln!(
                "Error while flushing data to stream: {:?}",
                self.stream.peer_addr()
            );
            return;
        }
    }
    pub async fn close_conn_with_error(&mut self, bytes: impl RespBytes) {
        self.write_response(bytes.into_response()).await
    }
}
