//! VoxCode — Serverless IP encoding for shareable room codes.
//!
//! Encodes a 32-bit IPv4 address through three obfuscation layers
//! to produce a 7-character alphanumeric code. Both host and joiner
//! have the same app, so the secret constants are shared implicitly.
//!
//! ## Algorithm (Encode)
//!   1. Parse IP → u32 (big-endian)
//!   2. XOR with KEY_A
//!   3. Byte-shuffle: [0,1,2,3] → [2,0,3,1]
//!   4. XOR with KEY_B
//!   5. Encode as 7-char base-36 using a custom shuffled alphabet
//!
//! ## Algorithm (Decode) — exact reverse of above
use std::net::Ipv4Addr;

// ── Secret constants baked into the application ──────────────
// Both host and joiner have the same binary, so these are always in sync.
const KEY_A: u32 = 0xA3F2_B781;
const KEY_B: u32 = 0x4C1A_7F33;

/// Custom base-36 alphabet: all 26 uppercase letters + 10 digits (0-9), shuffled.
/// Shuffled so codes don't look sequential or IP-like.
const ALPHABET: &[u8; 36] = b"7Z3K9WMRB4VDYX8JNTFC2EPQGA1HULS60OI5";

/// All VoxCodes are exactly 7 characters.
pub const CODE_LEN: usize = 7;

// ── Error type ────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum CodeError {
    #[error("Invalid IP address: {0}")]
    InvalidIp(#[from] std::net::AddrParseError),

    #[error("Code must be exactly {CODE_LEN} characters, got {got}")]
    InvalidLength { got: usize },

    #[error("Invalid character in code: '{0}'")]
    InvalidChar(char),
}

// ── Public API ────────────────────────────────────────────────

/// Encode an IPv4 address string into a 7-character VoxCode.
///
/// ```
/// let code = chatcall_net::room_code::encode_ip("8.8.8.8").unwrap();
/// assert_eq!(code.len(), 7);
/// ```
pub fn encode_ip(ip_str: &str) -> Result<String, CodeError> {
    let ip: Ipv4Addr = ip_str.trim().parse()?;
    let n = u32::from(ip);

    // Layer 1: XOR with key A
    let n = n ^ KEY_A;

    // Layer 2: Byte shuffle [0,1,2,3] → [2,0,3,1]
    let b = n.to_be_bytes();
    let n = u32::from_be_bytes([b[2], b[0], b[3], b[1]]);

    // Layer 3: XOR with key B
    let n = n ^ KEY_B;

    Ok(to_base36(n))
}

/// Decode a 7-character VoxCode back to an IPv4 address string.
///
/// ```
/// let code = chatcall_net::room_code::encode_ip("8.8.8.8").unwrap();
/// let ip = chatcall_net::room_code::decode_ip(&code).unwrap();
/// assert_eq!(ip, "8.8.8.8");
/// ```
pub fn decode_ip(code: &str) -> Result<String, CodeError> {
    let code = code.trim().to_uppercase();
    let n = from_base36(&code)?;

    // Reverse layer 3
    let n = n ^ KEY_B;

    // Reverse layer 2:
    // Original: b[2]→[0], b[0]→[1], b[3]→[2], b[1]→[3]
    // Reverse:  [0]→b[1], [1]→b[3], [2]→b[0], [3]→b[2]
    let b = n.to_be_bytes();
    let n = u32::from_be_bytes([b[1], b[3], b[0], b[2]]);

    // Reverse layer 1
    let n = n ^ KEY_A;

    Ok(Ipv4Addr::from(n).to_string())
}

// ── Internal helpers ──────────────────────────────────────────

fn to_base36(mut n: u32) -> String {
    let mut digits = [0u8; CODE_LEN];
    for i in (0..CODE_LEN).rev() {
        digits[i] = ALPHABET[(n % 36) as usize];
        n /= 36;
    }
    String::from_utf8(digits.to_vec()).expect("all chars are ASCII")
}

fn from_base36(s: &str) -> Result<u32, CodeError> {
    let len = s.chars().count();
    if len != CODE_LEN {
        return Err(CodeError::InvalidLength { got: len });
    }
    let mut n: u32 = 0;
    for c in s.chars() {
        let pos = ALPHABET
            .iter()
            .position(|&x| x == c as u8)
            .ok_or(CodeError::InvalidChar(c))?;
        n = n.wrapping_mul(36).wrapping_add(pos as u32);
    }
    Ok(n)
}

// ── Tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_common_ips() {
        let ips = [
            "1.1.1.1",
            "8.8.8.8",
            "8.8.4.4",
            "1.2.3.4",
            "203.0.113.42",
            "192.168.1.1",
            "10.0.0.1",
            "255.255.255.255",
            "0.0.0.0",
        ];
        for ip in &ips {
            let code = encode_ip(ip).unwrap_or_else(|e| panic!("encode({ip}): {e}"));
            assert_eq!(code.len(), CODE_LEN, "wrong length for {ip}: {code}");
            let decoded = decode_ip(&code).unwrap_or_else(|e| panic!("decode({code}): {e}"));
            assert_eq!(decoded, *ip, "roundtrip failed: {ip} → {code} → {decoded}");
        }
    }

    #[test]
    fn test_codes_look_different() {
        let codes: Vec<_> = ["8.8.8.8", "8.8.4.4", "1.1.1.1", "9.9.9.9"]
            .iter()
            .map(|ip| encode_ip(ip).unwrap())
            .collect();

        // All codes must be unique
        for i in 0..codes.len() {
            for j in (i + 1)..codes.len() {
                assert_ne!(codes[i], codes[j]);
            }
        }
    }

    #[test]
    fn test_codes_not_plaintext_ip() {
        // Codes should not look like IPs — no dots, no decimals
        let code = encode_ip("8.8.8.8").unwrap();
        assert!(!code.contains('.'));
        assert_eq!(code.len(), CODE_LEN);
    }

    #[test]
    fn test_case_insensitive_decode() {
        let code = encode_ip("1.2.3.4").unwrap();
        let lower = code.to_lowercase();
        let decoded = decode_ip(&lower).unwrap();
        assert_eq!(decoded, "1.2.3.4");
    }

    #[test]
    fn test_invalid_code_length() {
        assert!(matches!(decode_ip("TOOSHORT"), Err(CodeError::InvalidLength { .. })));
        assert!(matches!(decode_ip("TOOLONGCODE"), Err(CodeError::InvalidLength { .. })));
    }

    #[test]
    fn test_invalid_code_char() {
        // '!' is not in our alphabet
        assert!(matches!(decode_ip("!!!!!!"), Err(_)));
    }
}
