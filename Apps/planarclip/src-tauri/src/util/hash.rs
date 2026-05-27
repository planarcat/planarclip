pub fn hash_bytes(data: &[u8]) -> [u8; 32] {
    *blake3::hash(data).as_bytes()
}

pub fn hash_str(s: &str) -> [u8; 32] {
    hash_bytes(s.as_bytes())
}
