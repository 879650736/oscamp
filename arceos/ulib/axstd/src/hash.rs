use axhal::misc::random;
use alloc::vec;
use crate::sync::{Mutex, MutexGuard};
use core::hash::{Hash, Hasher};
use core::{mem, str};
use alloc::{string::String, vec::Vec};
use hashbrown::HashMap as InnerMap;

pub struct HashMap<K, V> {
    inner: InnerMap<K, V>,
}

impl<K: Hash + Eq, V> HashMap<K, V> {
    pub fn new() -> Self {
        Self {
            inner: InnerMap::new(),
        }
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.inner.insert(key, value)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.inner.iter()
    }
}
