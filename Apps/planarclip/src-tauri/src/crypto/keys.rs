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
