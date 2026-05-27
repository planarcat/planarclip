use rand::rngs::OsRng;
use x25519_dalek::{PublicKey, StaticSecret};

pub struct KeyPair {
    pub secret: StaticSecret,
    pub public: PublicKey,
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
        let hash = blake3::hash(self.public.as_bytes());
        hex::encode(&hash.as_bytes()[..8])
    }
}

/// Derive a shared secret from our secret key and peer's public key
pub fn derive_shared(secret: &StaticSecret, peer_public: &PublicKey) -> [u8; 32] {
    let shared = secret.diffie_hellman(peer_public);
    *shared.as_bytes()
}

/// Generate a 6-digit pairing code from a shared secret
pub fn pairing_code(shared_secret: &[u8; 32]) -> String {
    let hash = blake3::hash(shared_secret);
    let code = u64::from_le_bytes(hash.as_bytes()[..8].try_into().unwrap()) % 1_000_000;
    format!("{:06}", code)
}
