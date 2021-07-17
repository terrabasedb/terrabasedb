/*
 * Created on Sat Jul 17 2021
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

macro_rules! little_endian {
    ($block:block) => {
        #[cfg(target_endian = "little")]
        {
            $block
        }
    };
}

macro_rules! big_endian {
    ($block:block) => {
        #[cfg(target_endian = "big")]
        {
            $block
        }
    };
}

macro_rules! not_64_bit {
    ($block:block) => {
        #[cfg(not(target_pointer_width = "64"))]
        {
            $block
        }
    };
}

macro_rules! is_64_bit {
    ($block:block) => {
        #[cfg(target_pointer_width = "64")]
        {
            $block
        }
    };
}

#[cfg(target_endian = "big")]
macro_rules! to_64bit_little_endian {
    ($e:expr) => {
        ($e as u64).swap_bytes()
    };
}

#[cfg(target_endian = "little")]
macro_rules! to_64bit_little_endian {
    ($e:expr) => {
        ($e as u64)
    };
}

macro_rules! try_dir_ignore_existing {
    ($dir:expr) => {{
        match std::fs::create_dir_all($dir) {
            Ok(_) => Ok(()),
            Err(e) => match e.kind() {
                std::io::ErrorKind::AlreadyExists => Ok(()),
                _ => Err(e),
            },
        }
    }};
    ($($dir:expr),*) => {
        $(try_dir_ignore_existing!($dir)?;)*
    }
}

#[macro_export]
macro_rules! concat_path {
    ($($s:expr),+) => {{ {
        let mut path = std::path::PathBuf::with_capacity($(($s).len()+)*0);
        $(path.push($s);)*
        path
    }}};
}

#[macro_export]
macro_rules! concat_str {
    ($($s:expr),+) => {{ {
        let mut st = std::string::String::with_capacity($(($s).len()+)*0);
        $(st.push_str($s);)*
        st
    }}};
}

#[macro_export]
macro_rules! bad_data {
    () => {
        std::io::Error::from(std::io::ErrorKind::InvalidData)
    };
}
