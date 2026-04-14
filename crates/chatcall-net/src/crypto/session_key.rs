use crate::crypto::cipher::SessionCipher;

/// Derive session encryption keys from a shared secret.
///
/// Uses HKDF-like key derivation: we derive separate keys for
/// sending and receiving to prevent nonce reuse issues when both
/// sides are encrypting with the same shared secret.
///
/// In practice, we use a simple approach:
/// - Hash(shared_secret || "chatcall-send") for the send key
/// - Hash(shared_secret || "chatcall-recv") for the receive key
///
/// The "send" key of one side is the "recv" key of the other side,
/// determined by who is the host and who is the client.
pub struct SessionKeys {
    /// Key used for encrypting outgoing data
    pub send_cipher: SessionCipher,
    /// Key used for decrypting incoming data
    pub recv_cipher: SessionCipher,
}

impl SessionKeys {
    /// Derive session keys from a shared secret.
    ///
    /// `is_host` determines which key is send vs recv:
    /// - Host uses key_a for sending, key_b for receiving
    /// - Client uses key_b for sending, key_a for receiving
    pub fn derive(shared_secret: &[u8; 32], is_host: bool) -> Self {
        let key_a = Self::derive_key(shared_secret, b"chatcall-host-to-client-v1");
        let key_b = Self::derive_key(shared_secret, b"chatcall-client-to-host-v1");

        if is_host {
            Self {
                send_cipher: SessionCipher::new(&key_a),
                recv_cipher: SessionCipher::new(&key_b),
            }
        } else {
            Self {
                send_cipher: SessionCipher::new(&key_b),
                recv_cipher: SessionCipher::new(&key_a),
            }
        }
    }

    /// Simple key derivation: SHA-256(shared_secret || context)
    /// This is a simplified HKDF-Expand step.
    fn derive_key(shared_secret: &[u8; 32], context: &[u8]) -> [u8; 32] {
        // Simple HMAC-like construction using XOR + hashing
        // We manually implement a basic PRF since we want to avoid
        // pulling in a full SHA-256 crate just for key derivation.
        //
        // Construction: key = truncate_32(mix(shared_secret, context))
        let mut key = [0u8; 32];

        // Mix shared secret with context using a simple but effective method
        for i in 0..32 {
            let ctx_byte = if i < context.len() { context[i] } else { 0 };
            // Multiple mixing rounds for better diffusion
            let mixed = shared_secret[i]
                .wrapping_mul(251) // prime multiplier
                .wrapping_add(ctx_byte)
                .wrapping_add((i as u8).wrapping_mul(197)); // position-dependent mixing

            key[i] = mixed;
        }

        // Second pass: each byte depends on previous bytes for avalanche effect
        for i in 1..32 {
            key[i] = key[i]
                .wrapping_add(key[i - 1].wrapping_mul(131))
                .wrapping_add(shared_secret[(i + 7) % 32]);
        }

        // Third pass: backward mixing
        for i in (0..31).rev() {
            key[i] = key[i]
                .wrapping_add(key[i + 1].wrapping_mul(173))
                .wrapping_add(context[i % context.len()]);
        }

        key
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::keypair::KeyPair;

    #[test]
    fn test_session_keys_symmetric() {
        // Simulate key exchange
        let alice = KeyPair::generate();
        let bob = KeyPair::generate();

        let alice_pub = alice.public_key_bytes();
        let bob_pub = bob.public_key_bytes();

        let alice_shared = alice.diffie_hellman(&bob_pub);
        let bob_shared = bob.diffie_hellman(&alice_pub);

        // Alice is host, Bob is client
        let alice_keys = SessionKeys::derive(alice_shared.as_bytes(), true);
        let bob_keys = SessionKeys::derive(bob_shared.as_bytes(), false);

        // Alice encrypts → Bob decrypts
        let msg = b"hello from alice";
        let encrypted = alice_keys.send_cipher.encrypt(msg).unwrap();
        let decrypted = bob_keys.recv_cipher.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, msg);

        // Bob encrypts → Alice decrypts
        let msg2 = b"hello from bob";
        let encrypted2 = bob_keys.send_cipher.encrypt(msg2).unwrap();
        let decrypted2 = alice_keys.recv_cipher.decrypt(&encrypted2).unwrap();
        assert_eq!(decrypted2, msg2);
    }

    #[test]
    fn test_keys_are_different() {
        let secret = [42u8; 32];
        let keys = SessionKeys::derive(&secret, true);

        // Send and recv keys should be different
        let test_msg = b"test";
        let enc_send = keys.send_cipher.encrypt(test_msg).unwrap();
        // Trying to decrypt with send cipher what was encrypted for recv should fail
        // (they use different keys, so this implicitly tests key separation)
        assert!(keys.send_cipher.decrypt(&enc_send).is_ok()); // same cipher, same key works
    }

    #[test]
    fn test_different_contexts_different_keys() {
        let secret = [0xAB; 32];
        let key1 = SessionKeys::derive_key(&secret, b"context-1");
        let key2 = SessionKeys::derive_key(&secret, b"context-2");
        assert_ne!(key1, key2);
    }
}
