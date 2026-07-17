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


// ---- inline unit tests ----
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_store_never_matches() {
        let store = DedupStore::new(8);
        assert!(!store.has_seen(&[1u8; 32]));
    }

    #[test]
    fn mark_then_check_returns_true_for_same_hash() {
        let mut store = DedupStore::new(8);
        let hash = [7u8; 32];
        store.mark_seen(hash);
        assert!(store.has_seen(&hash));
    }

    #[test]
    fn different_hashes_are_distinct() {
        let mut store = DedupStore::new(8);
        let a = [1u8; 32];
        let b = [2u8; 32];
        store.mark_seen(a);
        assert!(store.has_seen(&a));
        assert!(!store.has_seen(&b));
    }

    #[test]
    fn full_store_evicts_all_previous_entries() {
        // simple eviction: clear all when full（当前实现语义）
        let mut store = DedupStore::new(2);
        let a = [1u8; 32];
        let b = [2u8; 32];
        let c = [3u8; 32];
        store.mark_seen(a);
        store.mark_seen(b);
        assert!(store.has_seen(&a));
        assert!(store.has_seen(&b));

        // 第三个触发 clear，再插入 c
        store.mark_seen(c);
        assert!(store.has_seen(&c));
        assert!(!store.has_seen(&a));
        assert!(!store.has_seen(&b));
    }
}
