//! Debug test: decode the user's actual VoxCode and verify the algorithm.

use chatcall_net::room_code::{encode_ip, decode_ip};

#[test]
fn decode_user_code() {
    // The user's friend's code
    let code = "7ULFJ7Z";
    match decode_ip(code) {
        Ok(ip) => println!("✅ Code '{}' decodes to IP: {}", code, ip),
        Err(e) => println!("❌ Failed to decode '{}': {}", code, e),
    }
}

#[test]
fn test_roundtrip_verify() {
    // Test several IPs and print their codes
    let test_ips = [
        "0.0.0.0",
        "1.1.1.1",
        "8.8.8.8",
        "192.168.1.1",
        "10.0.0.1",
        "103.25.200.50",
        "49.36.100.200",
        "255.255.255.255",
    ];
    
    println!("\n=== IP ↔ VoxCode Roundtrip Table ===");
    for ip in &test_ips {
        match encode_ip(ip) {
            Ok(code) => {
                let decoded = decode_ip(&code).unwrap_or_else(|e| format!("ERROR: {}", e));
                let ok = if decoded == *ip { "✅" } else { "❌" };
                println!("{} {} → {} → {}", ok, ip, code, decoded);
            }
            Err(e) => println!("❌ encode({}) failed: {}", ip, e),
        }
    }
}

#[test]
fn what_does_0_0_0_0_encode_to() {
    // Check what 0.0.0.0 encodes to — if the code is "7ULFJ7Z" this means
    // something went wrong with the public IP fetch
    let code = encode_ip("0.0.0.0").unwrap();
    println!("0.0.0.0 encodes to: {}", code);
    
    let code2 = encode_ip("127.0.0.1").unwrap();
    println!("127.0.0.1 encodes to: {}", code2);
}
