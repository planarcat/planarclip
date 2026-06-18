use rand::rngs::OsRng;
use x25519_dalek::{PublicKey, StaticSecret};

#[derive(Clone)]
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
