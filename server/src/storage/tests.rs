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

use super::*;

#[test]
fn test_serialize_deserialize_empty() {
    let cmap = Coremap::new();
    let ser = se::serialize_map(&cmap, 0).unwrap();
    let (de, model_code) = de::deserialize_map(ser).unwrap();
    assert!(de.len() == 0);
    assert_eq!(0, model_code);
}

#[test]
fn test_ser_de_few_elements() {
    let cmap = Coremap::new();
    cmap.upsert("sayan".into(), "writes code".into());
    cmap.upsert("supersayan".into(), "writes super code".into());
    let ser = se::serialize_map(&cmap, 0).unwrap();
    let (de, modelcode) = de::deserialize_map(ser).unwrap();
    assert!(de.len() == cmap.len());
    assert!(de
        .iter()
        .all(|kv| cmap.get(kv.key()).unwrap().eq(kv.value())));
    assert_eq!(modelcode, 0);
}

cfg_test!(
    use libstress::utils::generate_random_string_vector;
    use rand::thread_rng;
    #[test]
    fn roast_the_serializer() {
        const COUNT: usize = 1000_usize;
        const LEN: usize = 8_usize;
        let mut rng = thread_rng();
        let (keys, values) = (
            generate_random_string_vector(COUNT, LEN, &mut rng, true),
            generate_random_string_vector(COUNT, LEN, &mut rng, false),
        );
        let cmap: Coremap<Data, Data> = keys
            .iter()
            .zip(values.iter())
            .map(|(k, v)| (Data::from(k.to_owned()), Data::from(v.to_owned())))
            .collect();
        let ser = se::serialize_map(&cmap, 0).unwrap();
        let (de, modelcode) = de::deserialize_map(ser).unwrap();
        assert!(de
            .iter()
            .all(|kv| cmap.get(kv.key()).unwrap().eq(kv.value())));
        assert!(de.len() == cmap.len());
        assert_eq!(modelcode, 0);
    }

    #[test]
    fn test_ser_de_safety() {
        const COUNT: usize = 1000_usize;
        const LEN: usize = 8_usize;
        let mut rng = thread_rng();
        let (keys, values) = (
            generate_random_string_vector(COUNT, LEN, &mut rng, true),
            generate_random_string_vector(COUNT, LEN, &mut rng, false),
        );
        let cmap: Coremap<Data, Data> = keys
            .iter()
            .zip(values.iter())
            .map(|(k, v)| (Data::from(k.to_owned()), Data::from(v.to_owned())))
            .collect();
        let mut se = se::serialize_map(&cmap, 0).unwrap();
        // random chop
        se.truncate(124);
        // corrupted
        assert!(de::deserialize_map(se).is_none());
    }
    #[test]
    fn test_ser_de_excess_bytes() {
        // this test needs a lot of auxiliary space
        // we can approximate this to be: 100,000 x 30 bytes = 3,000,000 bytes
        // and then we may have a clone overhead + heap allocation by the map
        // so ~9,000,000 bytes or ~9MB
        const COUNT: usize = 1000_usize;
        const LEN: usize = 8_usize;
        let mut rng = thread_rng();
        let (keys, values) = (
            generate_random_string_vector(COUNT, LEN, &mut rng, true),
            generate_random_string_vector(COUNT, LEN, &mut rng, false),
        );
        let cmap: Coremap<Data, Data> = keys
            .iter()
            .zip(values.iter())
            .map(|(k, v)| (Data::from(k.to_owned()), Data::from(v.to_owned())))
            .collect();
        let mut se = se::serialize_map(&cmap, 0).unwrap();
        // random patch
        let patch: Vec<u8> = (0u16..500u16).into_iter().map(|v| (v >> 7) as u8).collect();
        se.extend(patch);
        assert!(de::deserialize_map(se).is_none());
    }
);

#[cfg(target_pointer_width = "32")]
#[test]
#[should_panic]
fn test_runtime_panic_32bit_or_lower() {
    let max = u64::MAX;
    let byte_stream = unsafe { raw_byte_repr(&max).to_owned() };
    let ptr = byte_stream.as_ptr();
    unsafe { de::transmute_len(ptr) };
}

mod interface_tests {
    use super::interface::{create_tree, DIR_KSROOT, DIR_ROOT, DIR_SNAPROOT};
    use crate::concat_path;
    use crate::coredb::memstore::Memstore;
    use std::fs;
    use std::path::PathBuf;
    #[test]
    fn test_tree() {
        create_tree(Memstore::new_default()).unwrap();
        let read_ks: Vec<String> = fs::read_dir(DIR_KSROOT)
            .unwrap()
            .map(|dir| {
                let v = dir.unwrap().file_name();
                v.to_string_lossy().to_string()
            })
            .collect();
        assert_eq!(read_ks, vec!["default".to_owned()]);
        // just read one level of the snaps dir
        let read_snaps: Vec<String> = fs::read_dir(DIR_SNAPROOT)
            .unwrap()
            .map(|dir| {
                let v = dir.unwrap().file_name();
                v.to_string_lossy().to_string()
            })
            .collect();
        assert_eq!(read_snaps, vec!["default".to_owned()]);
        // now read level two: snaps/default
        let read_snaps: Vec<String> = fs::read_dir(concat_path!(DIR_SNAPROOT, "default"))
            .unwrap()
            .map(|dir| {
                let v = dir.unwrap().file_name();
                v.to_string_lossy().to_string()
            })
            .collect();
        assert_veceq!(read_snaps, vec!["_system".to_owned(), "default".to_owned()]);
        assert!(PathBuf::from("data/backups").is_dir());
        // clean up
        fs::remove_dir_all(DIR_ROOT).unwrap();
    }
}

mod preload_tests {
    use super::*;
    use crate::coredb::memstore::Memstore;
    #[test]
    fn test_preload() {
        let memstore = Memstore::new_default();
        let mut v = Vec::new();
        preload::raw_generate_preload(&mut v, &memstore).unwrap();
        let de: Vec<String> = preload::read_preload_raw(v)
            .unwrap()
            .into_iter()
            .map(|each| unsafe { each.as_str().to_owned() })
            .collect();
        assert_veceq!(de, vec!["default".to_owned()]);
    }
}

mod bytemark_set_tests {
    use super::*;
    use crate::coredb::memstore::{Keyspace, ObjectID};
    use crate::coredb::table::Table;
    use std::collections::HashMap;
    #[test]
    fn test_bytemark_for_nonvolatile() {
        let ks = Keyspace::empty_default();
        let mut v = Vec::new();
        se::raw_serialize_partmap(&mut v, &ks).unwrap();
        let ret: HashMap<ObjectID, u8> = de::deserialize_set_ctype_bytemark(&v).unwrap();
        let mut expected = HashMap::new();
        unsafe {
            expected.insert(ObjectID::from_slice("default"), 0);
            expected.insert(ObjectID::from_slice("_system"), 0);
        }
        assert_hmeq!(expected, ret);
    }
    #[test]
    fn test_bytemark_volatility_mixed() {
        let ks = Keyspace::empty();
        unsafe {
            ks.create_table(
                ObjectID::from_slice("cache"),
                Table::kve_from_model_code_and_data(0, true, Coremap::new()).unwrap(),
            );
            ks.create_table(
                ObjectID::from_slice("supersafe"),
                Table::kve_from_model_code_and_data(0, false, Coremap::new()).unwrap(),
            );
        }
        let mut v = Vec::new();
        se::raw_serialize_partmap(&mut v, &ks).unwrap();
        let ret: HashMap<ObjectID, u8> = de::deserialize_set_ctype_bytemark(&v).unwrap();
        let mut expected = HashMap::new();
        unsafe {
            // our cache is volatile
            expected.insert(ObjectID::from_slice("cache"), 1);
            // our supersafe is non volatile
            expected.insert(ObjectID::from_slice("supersafe"), 0);
        }
        assert_hmeq!(expected, ret);
    }
}
