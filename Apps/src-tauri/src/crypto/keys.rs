use rand::rngs::OsRng;
use x25519_dalek::{PublicKey, StaticSecret};

#[derive(Clone)]
pub struct KeyPair {
    pub secret: StaticSecret,
    pub public: PublicKey,
}

/// Builds the stable peer id shared by mDNS discovery, handshakes, and trusted-device storage.
pub fn peer_id_from_public_key(public_key: &[u8]) -> String {
    let hash = blake3::hash(public_key);
    hex::encode(&hash.as_bytes()[..8])
}

impl KeyPair {
    pub fn generate() -> Self {
        let secret = StaticSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);
        Self { secret, public }
    }

    pub fn public_bytes(&self) -> [u8; 32] {
        *self.public.as_bytes()
    }

    pub fn fingerprint(&self) -> String {
        peer_id_from_public_key(self.public.as_bytes())
    }
}


// ---- inline unit tests ----
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peer_id_is_deterministic_for_same_public_key() {
        let bytes = [42u8; 32];
        // 关键契约：相同公钥必须映射到相同 peer_id
        assert_eq!(peer_id_from_public_key(&bytes), peer_id_from_public_key(&bytes));
    }

    #[test]
    fn peer_id_differs_for_different_public_keys() {
        assert_ne!(
            peer_id_from_public_key(&[1u8; 32]),
            peer_id_from_public_key(&[2u8; 32])
        );
    }

    #[test]
    fn peer_id_is_16_char_lowercase_hex() {
        // blake3 前 8 字节 → 16 位小写 hex
        let id = peer_id_from_public_key(&[0u8; 32]);
        assert_eq!(id.len(), 16);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }

    #[test]
    fn generated_keypair_public_and_fingerprint_are_consistent() {
        let kp = KeyPair::generate();
        // fingerprint 必须由 public_bytes 派生而来
        assert_eq!(kp.fingerprint(), peer_id_from_public_key(&kp.public_bytes()));
        assert_eq!(kp.public_bytes().len(), 32);
    }

    #[test]
    fn independent_keypairs_have_different_fingerprints() {
        let a = KeyPair::generate();
        let b = KeyPair::generate();
        assert_ne!(a.fingerprint(), b.fingerprint());
    }
}
