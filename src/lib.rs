use std::{marker::PhantomData, ptr::NonNull};

const MAX_LEVEL: usize = 32;
const P: f64 = 0.5;

struct Data<K, V> {
    key: K,
    value: V,
}

impl<K: Clone, V: Clone> Clone for Data<K, V> {
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            value: self.value.clone(),
        }
    }
}

impl<K: Copy, V: Copy> Copy for Data<K, V> {}

impl<K, V> Into<(K, V)> for Data<K, V> {
    fn into(self) -> (K, V) {
        (self.key, self.value)
    }
}

impl<K, V> Into<Data<K, V>> for (K, V) {
    fn into(self) -> Data<K, V> {
        Data {
            key: self.0,
            value: self.1,
        }
    }
}

struct Node<K, V> {
    data: Option<Data<K, V>>,
    forward: Vec<Option<NonNull<Node<K, V>>>>,
}

impl<K, V> Node<K, V> {
    fn new(data: Option<Data<K, V>>, level: usize) -> Self {
        Self {
            data,
            forward: vec![None; level + 1],
        }
    }

    fn head() -> Self {
        Self::new(None, MAX_LEVEL - 1)
    }

    #[allow(unused)]
    fn level(&self) -> usize {
        self.forward.len() - 1
    }
}

fn random_level() -> usize {
    let mut level = 0;
    let mut x = P;
    let f = 1.0 - rand::random::<f64>();

    while x > f && level + 1 < MAX_LEVEL {
        level += 1;
        x *= P;
    }

    level
}

pub struct SkipList<K, V> {
    len: usize,
    head: NonNull<Node<K, V>>,
}

impl<K, V> SkipList<K, V>
where
    K: Ord,
{
    #[allow(unused)]
    pub fn new() -> Self {
        Self {
            len: 0,
            head: NonNull::new(Box::into_raw(Box::new(Node::head())))
                .expect("Failed to allocate node memory."),
        }
    }

    #[allow(unused)]
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let mut update_nodes: [NonNull<Node<K, V>>; MAX_LEVEL] = [self.head; MAX_LEVEL];

        let mut cur_node = self.head;
        for i in (0..MAX_LEVEL).rev() {
            while let Some(&next_node) = unsafe { cur_node.as_ref().forward[i].as_ref() } {
                if unsafe {
                    next_node
                        .as_ref()
                        .data
                        .as_ref()
                        .is_some_and(|d| d.key < key)
                } {
                    cur_node = next_node;
                } else {
                    break;
                }
            }

            update_nodes[i] = cur_node;
        }

        if cfg!(debug_assertions) {
            assert!(update_nodes
                .iter()
                .all(|ptr| { ptr.as_ptr() != std::ptr::null_mut() }));
        }

        if let Some(next_node) = unsafe { cur_node.as_mut().forward[0].as_mut() } {
            if let Some(data) = unsafe { next_node.as_mut().data.as_mut() } {
                if data.key == key {
                    let value = std::mem::replace(&mut data.value, value);
                    return Some(value);
                }
            }
        }

        let new_level = random_level();

        if cfg!(debug_assertions) {
            assert!(new_level <= MAX_LEVEL);
        }

        let mut new_node = NonNull::new(Box::into_raw(Box::new(Node::new(
            Some(Data { key, value }),
            new_level,
        ))))
        .expect("Failed to allocate node memory on insert.");

        for i in 0..=new_level {
            let mut node = update_nodes[i];
            unsafe {
                new_node.as_mut().forward[i] = node.as_ref().forward[i];
                node.as_mut().forward[i] = Some(new_node);
            }
        }

        self.len += 1;

        None
    }

    #[allow(unused)]
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let mut update_nodes: [NonNull<Node<K, V>>; MAX_LEVEL] = [self.head; MAX_LEVEL];

        let mut cur_node = self.head;
        for i in (0..MAX_LEVEL).rev() {
            while let Some(next_node) = unsafe { cur_node.as_ref().forward[i] } {
                if unsafe {
                    next_node
                        .as_ref()
                        .data
                        .as_ref()
                        .is_some_and(|d| d.key < *key)
                } {
                    cur_node = next_node;
                } else {
                    break;
                }
            }

            update_nodes[i] = cur_node;
        }

        if cfg!(debug_assertions) {
            assert!(update_nodes
                .iter()
                .all(|ptr| { ptr.as_ptr() != std::ptr::null_mut() }));
        }

        let del_node = unsafe {
            if cur_node.as_ref().forward[0]
                .as_ref()
                .is_some_and(|next_node| {
                    next_node
                        .as_ref()
                        .data
                        .as_ref()
                        .is_some_and(|d| d.key == *key)
                })
            {
                cur_node.as_mut().forward[0].unwrap()
            } else {
                return None;
            }
        };

        for i in 0..MAX_LEVEL {
            let mut node = update_nodes[i];
            if unsafe { node.as_ref().forward[i].is_none_or(|ptr| ptr != del_node) } {
                break;
            }

            unsafe {
                node.as_mut().forward[i] = del_node.as_ref().forward[i];
            }
        }

        let node = unsafe { Box::from_raw(del_node.as_ptr()) };
        self.len -= 1;
        Some(node.data.unwrap().value)
    }

    #[allow(unused)]
    pub fn get(&self, key: &K) -> Option<&V> {
        let mut cur_node = self.head;
        for i in (0..MAX_LEVEL).rev() {
            while let Some(next_node) = unsafe { cur_node.as_ref().forward[i] } {
                if unsafe {
                    next_node
                        .as_ref()
                        .data
                        .as_ref()
                        .is_some_and(|d| d.key < *key)
                } {
                    cur_node = next_node;
                } else {
                    break;
                }
            }
        }

        if let Some(next_node) = unsafe { cur_node.as_ref().forward[0] } {
            if let Some(data) = unsafe { next_node.as_ref().data.as_ref() } {
                if data.key == *key {
                    return Some(&data.value);
                }
            }
        }

        None
    }

    #[allow(unused)]
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let mut cur_node = self.head;
        for i in (0..MAX_LEVEL).rev() {
            while let Some(next_node) = unsafe { cur_node.as_ref().forward[i] } {
                if unsafe {
                    next_node
                        .as_ref()
                        .data
                        .as_ref()
                        .is_some_and(|d| d.key < *key)
                } {
                    cur_node = next_node;
                } else {
                    break;
                }
            }
        }

        if let Some(mut next_node) = unsafe { cur_node.as_mut().forward[0] } {
            if let Some(data) = unsafe { next_node.as_mut().data.as_mut() } {
                if data.key == *key {
                    return Some(&mut data.value);
                }
            }
        }

        None
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn clear(&mut self) {
        while !self.is_empty() {
            self.pop_front();
        }
    }
}

impl<K: Clone + Ord, V: Clone> Clone for SkipList<K, V> {
    fn clone(&self) -> Self {
        let mut new_list = SkipList::new();
        for (k, v) in self.iter() {
            new_list.insert(k.clone(), v.clone());
        }
        new_list
    }
}

impl<K, V> Drop for SkipList<K, V> {
    fn drop(&mut self) {
        let mut node = self.head;
        unsafe {
            while let Some(next_node) = node.as_ref().forward[0] {
                let _ = Box::from_raw(node.as_ptr());
                node = next_node;
            }

            let _ = Box::from_raw(node.as_ptr());
        }
    }
}

impl<K, V> std::fmt::Display for Data<K, V>
where
    K: std::fmt::Display,
    V: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.key, self.value)
    }
}

impl<K, V> std::fmt::Display for SkipList<K, V>
where
    K: std::fmt::Display,
    V: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;

        let mut node = self.head;
        if let Some(next_node) = unsafe { node.as_ref().forward[0] } {
            if let Some(data) = unsafe { next_node.as_ref().data.as_ref() } {
                write!(f, "{}", data)?;
            }
            node = next_node;
        }

        while let Some(next_node) = unsafe { node.as_ref().forward[0] } {
            if let Some(data) = unsafe { next_node.as_ref().data.as_ref() } {
                write!(f, ", {}", data)?;
            }
            node = next_node;
        }

        write!(f, "}}")
    }
}

impl<K, V> SkipList<K, V> {
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            cursor: unsafe { self.head.as_ref().forward[0] },
            _marker: PhantomData,
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        IterMut {
            cursor: unsafe { self.head.as_ref().forward[0] },
            _marker: PhantomData,
        }
    }
}

pub struct Iter<'a, K: 'a, V: 'a> {
    cursor: Option<NonNull<Node<K, V>>>,
    _marker: PhantomData<&'a Node<K, V>>,
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        match self.cursor {
            Some(node) => unsafe {
                let node = node.as_ptr();
                self.cursor = (*node).forward[0];

                let data = (*node).data.as_ref().unwrap();
                Some((&data.key, &data.value))
            },
            None => None,
        }
    }
}

pub struct IterMut<'a, K: 'a, V: 'a> {
    cursor: Option<NonNull<Node<K, V>>>,
    _marker: PhantomData<&'a mut Node<K, V>>,
}

impl<'a, K, V> Iterator for IterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        match self.cursor {
            Some(node) => unsafe {
                let node = node.as_ptr();
                self.cursor = (*node).forward[0];

                let data = (*node).data.as_mut().unwrap();
                Some((&data.key, &mut data.value))
            },
            None => None,
        }
    }
}

pub struct IntoIter<K, V> {
    inner: SkipList<K, V>,
}

impl<K, V> SkipList<K, V> {
    fn pop_front(&mut self) -> Option<(K, V)> {
        let next_node = unsafe { (*self.head.as_ptr()).forward[0] };

        if let Some(next_node) = next_node {
            let head = self.head.as_ptr();
            let del_node = next_node.as_ptr();

            unsafe {
                for level in 0..=((*del_node).level()) {
                    (*head).forward[level] = (*del_node).forward[level];
                }

                let node = Box::from_raw(del_node);
                let data = node.data.unwrap();

                self.len -= 1;

                Some(data.into())
            }
        } else {
            None
        }
    }
}

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.pop_front()
    }
}

impl<K, V> IntoIterator for SkipList<K, V> {
    type Item = (K, V);

    type IntoIter = IntoIter<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter { inner: self }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use itertools::Itertools;
    use rand::random;

    use crate::SkipList;

    #[test]
    fn new() {
        let skip_list: SkipList<i32, i32> = SkipList::new();
        assert_eq!(skip_list.len(), 0);
    }

    #[test]
    fn contains() {
        const TEST_SIZE: usize = 10000000;

        let mut data: HashMap<u64, u64> = HashMap::new();

        while data.len() < TEST_SIZE {
            let k = random();
            if data.contains_key(&k) {
                continue;
            }
            let v = random();

            data.insert(k, v);
        }

        let data = data.into_iter().sorted().collect_vec();

        let mut skip_list = SkipList::new();
        for &(k, v) in data.iter() {
            skip_list.insert(k, v);
        }

        assert_eq!(skip_list.len(), TEST_SIZE);

        let res = skip_list.into_iter().collect_vec();
        assert_eq!(res, data);
    }

    #[test]
    fn remove() {
        const TEST_SIZE: usize = 10000000;

        let mut data: HashMap<u64, u64> = HashMap::new();

        while data.len() < TEST_SIZE {
            let k = random();
            if data.contains_key(&k) {
                continue;
            }
            let v = random();
            data.insert(k, v);
        }

        let data = data.into_iter().sorted().collect_vec();
        let delete_keys = data
            .iter()
            .map(|&(k, _)| k)
            .filter(|&k| k % 2 == 0)
            .collect_vec();

        let mut skip_list = SkipList::new();
        for &(k, v) in data.iter() {
            skip_list.insert(k, v);
        }

        assert_eq!(skip_list.len(), TEST_SIZE);

        for &k in delete_keys.iter() {
            skip_list.remove(&k);
            assert!(skip_list.get(&k).is_none());
        }
    }

    #[test]
    fn clear() {
        const TEST_SIZE: usize = 10000000;

        let mut data: HashMap<u64, u64> = HashMap::new();

        while data.len() < TEST_SIZE {
            let k = random();
            if data.contains_key(&k) {
                continue;
            }
            let v = random();

            data.insert(k, v);
        }

        let data = data.into_iter().sorted().collect_vec();

        let mut skip_list = SkipList::new();
        for &(k, v) in data.iter() {
            skip_list.insert(k, v);
        }

        assert_eq!(skip_list.len(), TEST_SIZE);

        skip_list.clear();
        assert_eq!(skip_list.len(), 0);
        assert_eq!(skip_list.is_empty(), true);
    }

    #[test]
    fn get() {
        const TEST_SIZE: usize = 10000000;

        let mut data: HashMap<u64, u64> = HashMap::new();

        while data.len() < TEST_SIZE {
            let k = random();
            if data.contains_key(&k) {
                continue;
            }
            let v = random();

            data.insert(k, v);
        }

        let data = data.into_iter().sorted().collect_vec();

        let mut skip_list = SkipList::new();
        for &(k, v) in data.iter() {
            skip_list.insert(k, v);
        }

        assert_eq!(skip_list.len(), TEST_SIZE);

        for &(k, v) in data.iter() {
            assert_eq!(skip_list.get(&k), Some(&v));
        }
    }

    #[test]
    fn clone() {
        const TEST_SIZE: usize = 10000000;
        let mut data: HashMap<u64, u64> = HashMap::new();
        while data.len() < TEST_SIZE {
            let k = random();
            if data.contains_key(&k) {
                continue;
            }
            let v = random();
            data.insert(k, v);
        }

        let data = data.into_iter().sorted().collect_vec();

        let mut skip_list = SkipList::new();
        for &(k, v) in data.iter() {
            skip_list.insert(k, v);
        }

        let skip_list2 = skip_list.clone();

        assert_eq!(skip_list.len(), skip_list2.len());

        for (k, v) in skip_list.iter() {
            assert_eq!(skip_list2.get(k), Some(v));
        }

        assert_eq!(
            skip_list.into_iter().collect_vec(),
            skip_list2.into_iter().collect_vec()
        );
    }
}
