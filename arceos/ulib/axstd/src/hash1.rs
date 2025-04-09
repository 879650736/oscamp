use axhal::misc::random;
use alloc::vec;
use crate::sync::{Mutex, MutexGuard};
use core::{hash, mem, str};
use alloc::{string::String, vec::Vec};

struct KeyValue<K, V> {
    key: K,
    value: V,
}

pub struct HashMap<K, V> {
    inner: Mutex<InnerMap<K, V>>,
}

pub struct HashMapIter<K, V> {
    items: Vec<(K, V)>,
    current: usize,
}

impl<K, V> Iterator for HashMapIter<K, V> 
where 
    K: Clone,
    V: Clone,
{
    type Item = (K, V);
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.items.len() {
            let item = &self.items[self.current];
            self.current += 1;
            Some((item.0.clone(), item.1.clone()))
        } else {
            None
        }
    }
}

pub struct InnerMap<K, V> {
    buckets: Vec<Vec<KeyValue<K, V>>>,
    capacity: usize,
    size: usize,
}

impl<K, V> HashMap<K, V> 
where 
    K: Eq + Clone,
    V: Clone,
{
    pub fn new() -> Self {
        let capacity = 16;
        let mut buckets = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buckets.push(Vec::new());
        }
        Self {
            inner: Mutex::new(InnerMap {
                buckets,
                capacity,
                size: 0,
            }),
        }
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let mut inner = self.inner.lock();
        let hash = random() as usize;
        let index = hash % inner.capacity;
        
        // Check if key already exists in the bucket
        for kv in inner.buckets[index].iter_mut() {
            if kv.key == key {
                // Replace existing value
                let old_value = core::mem::replace(&mut kv.value, value);
                return Some(old_value);
            }
        }

        // Insert new key-value pair
        inner.buckets[index].push(KeyValue {
            key: key.clone(),
            value,
        });
        inner.size += 1;
        None
    }

    pub fn iter(&self) -> HashMapIter<K, V> {
        let guard = self.inner.lock();
        let mut items = Vec::new();
        
        for bucket in &guard.buckets {
            for kv in bucket {
                items.push((kv.key.clone(), kv.value.clone()));
            }
        }
        
        HashMapIter {
            items,
            current: 0,
        }
    }
}
