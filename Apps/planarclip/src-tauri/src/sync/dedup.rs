use std::collections::HashSet;

/// Deduplication ring buffer to prevent sync loops.
pub struct DedupStore {
    seen: HashSet<[u8; 32]>,
    max_size: usize,
}

impl DedupStore {
    pub fn new(max_size: usize) -> Self {
        Self {
            seen: HashSet::new(),
            max_size,
        }
    }

    pub fn has_seen(&self, hash: &[u8; 32]) -> bool {
        self.seen.contains(hash)
    }

    pub fn mark_seen(&mut self, hash: [u8; 32]) {
        if self.seen.len() >= self.max_size {
            self.seen.clear(); // simple eviction: clear all when full
        }
        self.seen.insert(hash);
    }
}
