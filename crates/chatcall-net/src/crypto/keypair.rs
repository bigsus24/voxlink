use x25519_dalek::{EphemeralSecret, PublicKey, SharedSecret};
use rand::rngs::OsRng;

/// X25519 Diffie-Hellman key pair for secure key exchange.
///
/// Each side generates an ephemeral keypair, exchanges public keys,
/// and derives a shared secret that's used to create session encryption keys.
pub struct KeyPair {
    secret: Option<EphemeralSecret>,
    public: PublicKey,
}

impl KeyPair {
    /// Generate a new random X25519 key pair
    pub fn generate() -> Self {
        let secret = EphemeralSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);
        Self {
            secret: Some(secret),
            public,
        }
    }

    /// Get the public key bytes (32 bytes) for sending to the peer
    pub fn public_key_bytes(&self) -> [u8; 32] {
        *self.public.as_bytes()
    }

    /// Get a reference to the public key
    pub fn public_key(&self) -> &PublicKey {
        &self.public
    }

    /// Perform the Diffie-Hellman key exchange with the peer's public key.
    /// Consumes the secret key (it should only be used once).
    ///
    /// Returns the shared secret (32 bytes) that both sides will derive
    /// independently.
    pub fn diffie_hellman(self, peer_public_key: &[u8; 32]) -> SharedSecret {
        let peer_pk = PublicKey::from(*peer_public_key);
        self.secret
            .expect("Secret key already consumed")
            .diffie_hellman(&peer_pk)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_exchange() {
        // Simulate Alice and Bob
        let alice = KeyPair::generate();
        let bob = KeyPair::generate();

        let alice_pub = alice.public_key_bytes();
        let bob_pub = bob.public_key_bytes();

        // Each side performs DH with the other's public key
        let alice_shared = alice.diffie_hellman(&bob_pub);
        let bob_shared = bob.diffie_hellman(&alice_pub);

        // Both should derive the same shared secret
        assert_eq!(alice_shared.as_bytes(), bob_shared.as_bytes());
    }

    #[test]
    fn test_different_keys_different_secrets() {
        let alice = KeyPair::generate();
        let bob = KeyPair::generate();
        let charlie = KeyPair::generate();

        let bob_pub = bob.public_key_bytes();
        let charlie_pub = charlie.public_key_bytes();

        let secret_ab = alice.diffie_hellman(&bob_pub);
        let secret_bc = bob.diffie_hellman(&charlie_pub);

        // Different key pairs should yield different shared secrets
        assert_ne!(secret_ab.as_bytes(), secret_bc.as_bytes());
    }
}
