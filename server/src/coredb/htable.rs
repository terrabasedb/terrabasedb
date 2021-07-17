/*
 * Created on Sun May 09 2021
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

use crate::coredb::array::Array;
use bytes::Bytes;
use libsky::TResult;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::fmt;
use std::hash::Hash;
use std::iter::FromIterator;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug)]
/// A thread-safe in-memory hashtable
///
/// This wraps around a [`Coremap`] object in an [`Arc`] to make it shareable across threads. Clones
/// are cheap because it just increments the atomic reference counter
pub struct HTable<K: Eq + Hash, V>
where
    K: Eq + Hash,
{
    inner: Arc<Coremap<K, V>>,
    _marker_key: PhantomData<K>,
    _marker_value: PhantomData<V>,
}

impl<K: Eq + Hash + Clone + Serialize, V: Clone + Serialize> HTable<K, V> {
    /// Initialize a new HTable instance from an existing [`Coremap`]
    pub fn from_raw(inner: Coremap<K, V>) -> Self {
        Self {
            inner: Arc::new(inner),
            _marker_key: PhantomData,
            _marker_value: PhantomData,
        }
    }
}

impl<K: Eq + Hash, V> HTable<K, V> {
    /// Create a new, empty in-memory table
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Coremap::new()),
            _marker_key: PhantomData,
            _marker_value: PhantomData,
        }
    }
}

impl<K, V> Clone for HTable<K, V>
where
    K: Eq + Hash,
{
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            _marker_key: self._marker_key,
            _marker_value: self._marker_value,
        }
    }
}

use dashmap::iter::Iter;
pub use dashmap::lock::RwLock as MapRWL;
pub use dashmap::lock::RwLockReadGuard as MapRWLGuard;
pub use dashmap::mapref::entry::Entry as MapEntry;
pub use dashmap::mapref::one::Ref as MapSingleReference;
use dashmap::mapref::one::Ref;
use dashmap::DashMap;
pub use dashmap::SharedValue;
pub type HashTable<K, V> = DashMap<K, V>;

#[derive(Debug)]
/// The Coremap contains the actual key/value pairs along with additional fields for data safety
/// and protection
pub struct Coremap<K, V>
where
    K: Eq + Hash,
{
    pub(crate) inner: HashTable<K, V>,
}

impl<K: Eq + Hash, V> Deref for HTable<K, V> {
    type Target = Coremap<K, V>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<K: Eq + Hash, V> Coremap<K, V> {
    /// Create an empty coremap
    pub fn new() -> Self {
        Coremap {
            inner: HashTable::new(),
        }
    }
    pub fn with_capacity(cap: usize) -> Self {
        Coremap {
            inner: HashTable::with_capacity(cap),
        }
    }
}

impl<K, V> Coremap<K, V>
where
    K: Eq + Hash,
{
    /// Returns the total number of key value pairs
    pub fn len(&self) -> usize {
        self.inner.len()
    }
    /// Returns the removed value for key, it it existed
    pub fn remove<Q>(&self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.remove(key)
    }
    /// Returns true if an existent key was removed
    pub fn true_if_removed<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.remove(key).is_some()
    }
    /// Check if a table contains a key
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.contains_key(key)
    }
    /// Clears the inner table!
    pub fn clear(&self) {
        self.inner.clear()
    }
    /// Return a non-consuming iterator
    pub fn iter(&self) -> Iter<'_, K, V> {
        self.inner.iter()
    }
    /// Get a reference to the value of a key, if it exists
    pub fn get<Q>(&self, key: &Q) -> Option<Ref<'_, K, V>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.get(key)
    }
    /// Returns true if the non-existent key was assigned to a value
    pub fn true_if_insert(&self, k: K, v: V) -> bool {
        if let MapEntry::Vacant(ve) = self.inner.entry(k) {
            ve.insert(v);
            true
        } else {
            false
        }
    }
    pub fn true_remove_if(&self, key: &K, exec: impl FnOnce(&K, &V) -> bool) -> bool {
        self.inner.remove_if(key, exec).is_some()
    }
    /// Update or insert
    pub fn upsert(&self, k: K, v: V) {
        let _ = self.inner.insert(k, v);
    }
    /// Returns true if the value was updated
    pub fn true_if_update(&self, k: K, v: V) -> bool {
        if let MapEntry::Occupied(mut oe) = self.inner.entry(k) {
            oe.insert(v);
            true
        } else {
            false
        }
    }
}

impl<K, V> Coremap<K, V>
where
    K: Eq + Hash + Serialize,
    V: Serialize,
{
    /// Serialize the hashtable into a `Vec<u8>` that can be saved to a file
    pub fn serialize(&self) -> TResult<Vec<u8>> {
        bincode::serialize(&self.inner).map_err(|e| e.into())
    }
}

impl Coremap<Data, Data> {
    /// Returns atleast `count` number of keys from the hashtable
    pub fn get_keys(&self, count: usize) -> Vec<Bytes> {
        let mut v = Vec::with_capacity(count);
        self.iter()
            .take(count)
            .map(|kv| kv.key().get_blob().clone())
            .for_each(|key| v.push(key));
        v
    }
    /// Returns a `Coremap<Data, Data>` from the provided file (as a `Vec<u8>`)
    pub fn deserialize(src: Vec<u8>) -> TResult<Self> {
        let h: HashTable<Data, Data> = bincode::deserialize(&src)?;
        Ok(Self { inner: h })
    }
}
impl<const M: usize, const N: usize> Coremap<Array<u8, M>, Array<u8, N>> {
    #[cfg(test)]
    pub fn deserialize_array(bytes: Vec<u8>) -> TResult<Self> {
        let h: HashTable<Array<u8, M>, Array<u8, N>> = bincode::deserialize(&bytes)?;
        Ok(Self { inner: h })
    }
}
impl<K: Eq + Hash, V> IntoIterator for Coremap<K, V> {
    type Item = (K, V);
    type IntoIter = dashmap::iter::OwningIter<K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl Deref for Data {
    type Target = [u8];
    fn deref(&self) -> &<Self>::Target {
        &self.blob
    }
}

impl Borrow<[u8]> for Data {
    fn borrow(&self) -> &[u8] {
        self.blob.borrow()
    }
}

impl AsRef<[u8]> for Data {
    fn as_ref(&self) -> &[u8] {
        &self.blob
    }
}

impl<K, V> FromIterator<(K, V)> for HTable<K, V>
where
    K: Eq + Hash,
{
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = (K, V)>,
    {
        Self {
            inner: Arc::new(Coremap {
                inner: DashMap::from_iter(iter),
            }),
            _marker_value: PhantomData,
            _marker_key: PhantomData,
        }
    }
}

impl<K, V> FromIterator<(K, V)> for Coremap<K, V>
where
    K: Eq + Hash,
{
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = (K, V)>,
    {
        Coremap {
            inner: DashMap::from_iter(iter),
        }
    }
}

/// A wrapper for `Bytes`
#[derive(Debug, PartialEq, Clone, Hash)]
pub struct Data {
    /// The blob of data
    blob: Bytes,
}

impl Data {
    /// Create a new blob from a string
    pub fn from_string(val: String) -> Self {
        Data {
            blob: Bytes::from(val.into_bytes()),
        }
    }
    /// Create a new blob from an existing `Bytes` instance
    pub const fn from_blob(blob: Bytes) -> Self {
        Data { blob }
    }
    /// Get the inner blob (raw `Bytes`)
    pub const fn get_blob(&self) -> &Bytes {
        &self.blob
    }
    pub fn into_inner(self) -> Bytes {
        self.blob
    }
    #[allow(clippy::needless_lifetimes)]
    pub fn copy_from_slice<'a>(slice: &'a [u8]) -> Self {
        Self {
            blob: Bytes::copy_from_slice(slice),
        }
    }
}

impl Eq for Data {}

impl<T> From<T> for Data
where
    T: Into<Bytes>,
{
    fn from(dat: T) -> Self {
        Self { blob: dat.into() }
    }
}

use serde::ser::{SerializeSeq, Serializer};

impl Serialize for Data {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.blob.len()))?;
        for e in self.blob.iter() {
            seq.serialize_element(e)?;
        }
        seq.end()
    }
}

use serde::de::{Deserializer, SeqAccess, Visitor};

struct DataVisitor;
impl<'de> Visitor<'de> for DataVisitor {
    type Value = Data;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("Expecting a coredb::htable::Data object")
    }
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut bytes = Vec::new();
        while let Some(unsigned_8bit_int) = seq.next_element()? {
            bytes.push(unsigned_8bit_int);
        }
        Ok(Data::from_blob(Bytes::from(bytes)))
    }
}

impl<'de> Deserialize<'de> for Data {
    fn deserialize<D>(deserializer: D) -> Result<Data, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(DataVisitor)
    }
}

#[test]
fn test_de() {
    let x = HTable::new();
    x.upsert(
        Data::from("sayan"),
        Data::from_string("is writing open-source code".to_owned()),
    );
    let ser = x.serialize().unwrap();
    let de = Coremap::<Data, Data>::deserialize(ser).unwrap();
    assert!(de.contains_key(&Data::from("sayan")));
    assert!(de.len() == x.len());
    let hmap: Coremap<Data, Data> = Coremap::new();
    hmap.upsert(Data::from("sayan"), Data::from("writes code"));
    assert!(hmap.get("sayan".as_bytes()).is_some());
}
