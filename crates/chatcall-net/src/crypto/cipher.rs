use chacha20poly1305::{
    ChaCha20Poly1305, Key, Nonce,
    aead::{Aead, KeyInit},
};
use std::sync::atomic::{AtomicU64, Ordering};

/// Error type for cipher operations
#[derive(Debug, thiserror::Error)]
pub enum CipherError {
    #[error("Encryption failed")]
    EncryptionFailed,

    #[error("Decryption failed — data may be tampered or key mismatch")]
    DecryptionFailed,

    #[error("Invalid nonce in ciphertext")]
    InvalidNonce,

    #[error("Ciphertext too short (minimum {min} bytes, got {got})")]
    CiphertextTooShort { min: usize, got: usize },
}

/// ChaCha20-Poly1305 AEAD cipher for encrypting voice and chat data.
///
/// Provides authenticated encryption — each ciphertext includes a 16-byte
/// authentication tag that detects tampering. Uses a monotonic nonce counter
/// to ensure nonce uniqueness (critical for security).
///
/// Output format:
/// ```text
/// [Nonce (12 bytes)] [Ciphertext (N bytes)] [Auth Tag (16 bytes)]
/// ```
///
/// The nonce is prepended to the ciphertext so the receiver can extract
/// it for decryption. The auth tag is appended by ChaCha20-Poly1305
/// automatically.
pub struct SessionCipher {
    cipher: ChaCha20Poly1305,
    /// Monotonically increasing nonce counter (thread-safe)
    nonce_counter: AtomicU64,
}

impl SessionCipher {
    /// Nonce size for ChaCha20-Poly1305
    pub const NONCE_SIZE: usize = 12;
    /// Auth tag size
    pub const TAG_SIZE: usize = 16;
    /// Total overhead added to plaintext: nonce + tag
    pub const OVERHEAD: usize = Self::NONCE_SIZE + Self::TAG_SIZE;

    /// Create a new cipher from a 32-byte key (derived from key exchange)
    pub fn new(key_bytes: &[u8; 32]) -> Self {
        let key = Key::from_slice(key_bytes);
        let cipher = ChaCha20Poly1305::new(key);
        Self {
            cipher,
            nonce_counter: AtomicU64::new(0),
        }
    }

    /// Encrypt plaintext. Returns nonce + ciphertext + tag.
    ///
    /// Each call uses a unique nonce derived from an atomic counter,
    /// ensuring no nonce is ever reused with the same key.
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, CipherError> {
        let counter = self.nonce_counter.fetch_add(1, Ordering::Relaxed);
        let nonce_bytes = Self::counter_to_nonce(counter);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self.cipher.encrypt(nonce, plaintext)
            .map_err(|_| CipherError::EncryptionFailed)?;

        // Prepend nonce to ciphertext
        let mut output = Vec::with_capacity(Self::NONCE_SIZE + ciphertext.len());
        output.extend_from_slice(&nonce_bytes);
        output.extend_from_slice(&ciphertext);
        Ok(output)
    }

    /// Decrypt ciphertext. Input must be: nonce (12B) + ciphertext + tag (16B).
    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, CipherError> {
        if data.len() < Self::OVERHEAD {
            return Err(CipherError::CiphertextTooShort {
                min: Self::OVERHEAD,
                got: data.len(),
            });
        }

        let nonce = Nonce::from_slice(&data[..Self::NONCE_SIZE]);
        let ciphertext = &data[Self::NONCE_SIZE..];

        self.cipher.decrypt(nonce, ciphertext)
            .map_err(|_| CipherError::DecryptionFailed)
    }

    /// Convert a u64 counter to a 12-byte nonce.
    /// First 4 bytes are zero, last 8 bytes are the counter in little-endian.
    fn counter_to_nonce(counter: u64) -> [u8; 12] {
        let mut nonce = [0u8; 12];
        nonce[4..12].copy_from_slice(&counter.to_le_bytes());
        nonce
    }

    /// Get the current nonce counter value (for diagnostics)
    pub fn nonce_count(&self) -> u64 {
        self.nonce_counter.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        for (i, byte) in key.iter_mut().enumerate() {
            *byte = i as u8;
        }
        key
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let cipher = SessionCipher::new(&test_key());
        let plaintext = b"Hello, ChatCall!";
        let encrypted = cipher.encrypt(plaintext).unwrap();
        let decrypted = cipher.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_unique_nonces() {
        let cipher = SessionCipher::new(&test_key());
        let e1 = cipher.encrypt(b"test1").unwrap();
        let e2 = cipher.encrypt(b"test1").unwrap();
        // Same plaintext should produce different ciphertext (different nonces)
        assert_ne!(e1, e2);
        // Nonces should be different
        assert_ne!(&e1[..12], &e2[..12]);
    }

    #[test]
    fn test_tampered_data_fails() {
        let cipher = SessionCipher::new(&test_key());
        let mut encrypted = cipher.encrypt(b"secret data").unwrap();
        // Tamper with a byte in the ciphertext (after the nonce)
        encrypted[15] ^= 0xFF;
        assert!(cipher.decrypt(&encrypted).is_err());
    }

    #[test]
    fn test_wrong_key_fails() {
        let key1 = test_key();
        let mut key2 = test_key();
        key2[0] = 0xFF;

        let cipher1 = SessionCipher::new(&key1);
        let cipher2 = SessionCipher::new(&key2);

        let encrypted = cipher1.encrypt(b"hello").unwrap();
        assert!(cipher2.decrypt(&encrypted).is_err());
    }

    #[test]
    fn test_empty_plaintext() {
        let cipher = SessionCipher::new(&test_key());
        let encrypted = cipher.encrypt(b"").unwrap();
        assert_eq!(encrypted.len(), SessionCipher::OVERHEAD);
        let decrypted = cipher.decrypt(&encrypted).unwrap();
        assert!(decrypted.is_empty());
    }

    #[test]
    fn test_large_payload() {
        let cipher = SessionCipher::new(&test_key());
        let plaintext = vec![0xAB; 4096]; // 4KB payload
        let encrypted = cipher.encrypt(&plaintext).unwrap();
        let decrypted = cipher.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_ciphertext_too_short() {
        let cipher = SessionCipher::new(&test_key());
        let short_data = [0u8; 10]; // less than OVERHEAD
        assert!(matches!(
            cipher.decrypt(&short_data),
            Err(CipherError::CiphertextTooShort { .. })
        ));
    }
}
