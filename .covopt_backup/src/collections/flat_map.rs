use crate::collections::ahash::RandomState;
use core::borrow::Borrow;
use core::hash::{BuildHasher, Hash};
#[derive(Clone, Debug)]
pub enum Bucket<K, V> {
    Empty,
    Deleted,
    Occupied(K, V),
}
#[derive(Clone, Debug)]
#[repr(C, align(64))]
pub struct AHashMap<K, V, const N: usize, S = RandomState> {
    hasher: S,
    table: heapless::Vec<Bucket<K, V>, N>,
    len: usize,
    deleted_count: usize,
}
impl<K, V, const N: usize> Default for AHashMap<K, V, N, RandomState> {
    fn default() -> Self {
        Self::new()
    }
}
impl<K, V, const N: usize> AHashMap<K, V, N, RandomState> {
    pub fn new() -> Self {
        Self {
            hasher: RandomState::new(),
            table: heapless::Vec::new(),
            len: 0,
            deleted_count: 0,
        }
    }
    pub fn with_capacity(capacity: usize) -> Self {
        let _cap = (capacity * 3 / 2).next_power_of_two().max(8);
        let mut table = heapless::Vec::new();
        for _ in 0..N {
            let _ = table.push(Bucket::Empty);
        }
        Self {
            hasher: RandomState::new(),
            table,
            len: 0,
            deleted_count: 0,
        }
    }
}
impl<K, V, const N: usize, S> AHashMap<K, V, N, S> {
    pub fn with_hasher(hasher: S) -> Self {
        Self {
            hasher,
            table: heapless::Vec::new(),
            len: 0,
            deleted_count: 0,
        }
    }
}
impl<K, V, const N: usize, S> AHashMap<K, V, N, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    #[inline(never)]
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if self.table.is_empty() || (self.len + self.deleted_count + 1) * 10 >= self.table.len() * 7
        {
            let _new_cap = ((self.len + 1) * 2).next_power_of_two().max(8);
        }
        self.insert_no_resize(key, value)
    }
    fn insert_no_resize(&mut self, key: K, value: V) -> Option<V> {
        let hash = self.hasher.hash_one(&key);
        let cap = self.table.len();
        let mask = cap - 1;
        let mut idx = hash as usize & mask;
        let mut first_deleted = None;
        loop {
            match &mut self.table[idx] {
                Bucket::Empty => {
                    if let Some(del_idx) = first_deleted {
                        self.table[del_idx] = Bucket::Occupied(key, value);
                        self.deleted_count -= 1;
                        self.len += 1;
                        return None;
                    } else {
                        self.table[idx] = Bucket::Occupied(key, value);
                        self.len += 1;
                        return None;
                    }
                }
                Bucket::Deleted => {
                    if first_deleted.is_none() {
                        first_deleted = Some(idx);
                    }
                }
                Bucket::Occupied(k, v) => {
                    if k == &key {
                        return Some(core::mem::replace(v, value));
                    }
                }
            }
            idx = (idx + 1) & mask;
        }
    }
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if self.table.is_empty() {
            return None;
        }
        let hash = self.hasher.hash_one(key);
        let cap = self.table.len();
        let mask = cap - 1;
        let mut idx = hash as usize & mask;
        loop {
            match &self.table[idx] {
                Bucket::Empty => return None,
                Bucket::Deleted => {}
                Bucket::Occupied(k, v) => {
                    if k.borrow() == key {
                        return Some(v);
                    }
                }
            }
            idx = (idx + 1) & mask;
        }
    }
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if self.table.is_empty() {
            return None;
        }
        let hash = self.hasher.hash_one(key);
        let cap = self.table.len();
        let mask = cap - 1;
        let mut idx = hash as usize & mask;
        let found_idx = loop {
            match &self.table[idx] {
                Bucket::Empty => return None,
                Bucket::Deleted => {}
                Bucket::Occupied(k, _) => {
                    if k.borrow() == key {
                        break idx;
                    }
                }
            }
            idx = (idx + 1) & mask;
        };
        match &mut self.table[found_idx] {
            Bucket::Occupied(_, v) => Some(v),
            _ => unreachable!(),
        }
    }
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if self.table.is_empty() {
            return None;
        }
        let hash = self.hasher.hash_one(key);
        let cap = self.table.len();
        let mask = cap - 1;
        let mut idx = hash as usize & mask;
        let found_idx = loop {
            match &self.table[idx] {
                Bucket::Empty => return None,
                Bucket::Deleted => {}
                Bucket::Occupied(k, _) => {
                    if k.borrow() == key {
                        break idx;
                    }
                }
            }
            idx = (idx + 1) & mask;
        };
        let old = core::mem::replace(&mut self.table[found_idx], Bucket::Deleted);
        self.len -= 1;
        self.deleted_count += 1;
        if let Bucket::Occupied(_, v) = old {
            Some(v)
        } else {
            None
        }
    }
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.get(key).is_some()
    }
    pub fn clear(&mut self) {
        self.table.clear();
        self.len = 0;
        self.deleted_count = 0;
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            iter: self.table.iter(),
        }
    }
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        IterMut {
            iter: self.table.iter_mut(),
        }
    }
    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys { iter: self.iter() }
    }
    pub fn values(&self) -> Values<'_, K, V> {
        Values { iter: self.iter() }
    }
}
#[repr(C, align(64))]
pub struct Iter<'a, K, V> {
    iter: core::slice::Iter<'a, Bucket<K, V>>,
}
impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.iter.next() {
                Some(Bucket::Occupied(k, v)) => return Some((k, v)),
                Some(_) => {}
                None => return None,
            }
        }
    }
}
#[repr(C, align(64))]
pub struct IterMut<'a, K, V> {
    iter: core::slice::IterMut<'a, Bucket<K, V>>,
}
impl<'a, K, V> Iterator for IterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.iter.next() {
                Some(Bucket::Occupied(k, v)) => return Some((k, v)),
                Some(_) => {}
                None => return None,
            }
        }
    }
}
#[repr(C, align(64))]
pub struct Keys<'a, K, V> {
    iter: Iter<'a, K, V>,
}
impl<'a, K, V> Iterator for Keys<'a, K, V> {
    type Item = &'a K;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(k, _)| k)
    }
}
#[repr(C, align(64))]
pub struct Values<'a, K, V> {
    iter: Iter<'a, K, V>,
}
impl<'a, K, V> Iterator for Values<'a, K, V> {
    type Item = &'a V;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(_, v)| v)
    }
}
impl<'a, K, V, const N: usize, S> IntoIterator for &'a AHashMap<K, V, N, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<'a, K, V, const N: usize, S> IntoIterator for &'a mut AHashMap<K, V, N, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}
#[repr(C, align(64))]
pub struct IntoIter<K, V, const N: usize> {
    iter: <heapless::Vec<Bucket<K, V>, N> as IntoIterator>::IntoIter,
}
impl<K, V, const N: usize> Iterator for IntoIter<K, V, N> {
    type Item = (K, V);
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.iter.next() {
                Some(Bucket::Occupied(k, v)) => return Some((k, v)),
                Some(_) => {}
                None => return None,
            }
        }
    }
}
impl<K, V, const N: usize, S> IntoIterator for AHashMap<K, V, N, S> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V, N>;
    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            iter: self.table.into_iter(),
        }
    }
}
impl<K, V, const N: usize> FromIterator<(K, V)> for AHashMap<K, V, N, RandomState>
where
    K: Eq + Hash,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut map = Self::new();
        for (k, v) in iter {
            map.insert(k, v);
        }
        map
    }
}
