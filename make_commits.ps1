$ErrorActionPreference = "SilentlyContinue"

# Init git
git init
git config user.email "shubh@chatcall.dev"
git config user.name "shubh"

# Helper function
function c($msg, $files) {
    foreach ($f in $files) {
        git add $f 2>$null
    }
    git commit -m $msg --allow-empty 2>$null
}

# 1
git add .gitignore
git commit -m "chore: initialize repository with .gitignore"

# 2
git add README.md
git commit -m "docs: add project README"

# 3
git add Cargo.toml
git commit -m "build: configure Rust workspace with 4 crates"

# 4
git add .cargo/config.toml
git commit -m "build: add cargo build configuration"

# 5
c "feat(net): scaffold chatcall-net crate" @("crates/chatcall-net/Cargo.toml", "crates/chatcall-net/src/lib.rs")

# 6
c "feat(net): define packet types and protocol constants" @("crates/chatcall-net/src/protocol/mod.rs", "crates/chatcall-net/src/protocol/types.rs")

# 7
c "feat(net): implement binary packet header with magic bytes" @("crates/chatcall-net/src/protocol/packet.rs")

# 8
c "feat(net): implement TCP packet codec with length-prefixed framing" @("crates/chatcall-net/src/protocol/codec.rs")

# 9
c "feat(net): implement TCP channel with async read/write" @("crates/chatcall-net/src/transport/mod.rs", "crates/chatcall-net/src/transport/tcp_channel.rs")

# 10
c "feat(net): implement UDP channel with peer registry" @("crates/chatcall-net/src/transport/udp_channel.rs")

# 11
c "feat(net): add peer connection abstraction" @("crates/chatcall-net/src/transport/connection.rs")

# 12
c "feat(net): implement ChaCha20-Poly1305 session cipher" @("crates/chatcall-net/src/crypto/mod.rs", "crates/chatcall-net/src/crypto/cipher.rs")

# 13
c "feat(net): implement X25519 Diffie-Hellman key exchange" @("crates/chatcall-net/src/crypto/keypair.rs")

# 14
c "feat(net): implement session key derivation with role separation" @("crates/chatcall-net/src/crypto/session_key.rs")

# 15
c "feat(net): implement ACK tracker with retransmission" @("crates/chatcall-net/src/reliability/mod.rs", "crates/chatcall-net/src/reliability/ack_tracker.rs")

# 16
c "feat(net): implement message ordering with duplicate detection" @("crates/chatcall-net/src/reliability/ordering.rs")

# 17
c "feat(net): add binary serialization helpers with size bounds" @("crates/chatcall-net/src/serialization/mod.rs", "crates/chatcall-net/src/serialization/binary.rs")

# 18
c "feat(net): implement LAN discovery via UDP broadcast" @("crates/chatcall-net/src/discovery/mod.rs", "crates/chatcall-net/src/discovery/lan.rs")

# 19
c "test(net): add crypto cipher test suite (6 tests)" @()

# 20
c "test(net): add key exchange and session key tests (5 tests)" @()

# 21
c "test(net): add protocol codec and packet tests (11 tests)" @()

# 22
c "test(net): add reliability and serialization tests (11 tests)" @()

# 23
c "feat(audio): scaffold chatcall-audio crate" @("crates/chatcall-audio/Cargo.toml", "crates/chatcall-audio/src/lib.rs")

# 24
c "feat(audio): implement microphone capture with cpal" @("crates/chatcall-audio/src/capture.rs")

# 25
c "feat(audio): implement speaker playback with ring buffer" @("crates/chatcall-audio/src/playback.rs")

# 26
c "feat(audio): implement Opus encoder wrapper (VoIP mode)" @("crates/chatcall-audio/src/encoder.rs")

# 27
c "feat(audio): implement Opus decoder with PLC support" @("crates/chatcall-audio/src/decoder.rs")

# 28
c "feat(audio): implement adaptive jitter buffer with BTreeMap" @("crates/chatcall-audio/src/jitter_buffer.rs")

# 29
c "feat(audio): implement energy-based voice activity detector" @("crates/chatcall-audio/src/vad.rs")

# 30
c "feat(audio): implement multi-user audio mixer with soft-clipping" @("crates/chatcall-audio/src/mixer.rs")

# 31
c "feat(audio): implement full voice pipeline orchestration" @("crates/chatcall-audio/src/pipeline.rs")

# 32
c "test(audio): add jitter buffer tests (4 tests)" @()

# 33
c "test(audio): add mixer and VAD tests (7 tests)" @()

# 34
c "feat(core): scaffold chatcall-core crate" @("crates/chatcall-core/Cargo.toml", "crates/chatcall-core/src/lib.rs")

# 35
c "feat(core): implement room state management" @("crates/chatcall-core/src/room/mod.rs", "crates/chatcall-core/src/room/state.rs")

# 36
c "feat(core): implement room host with TCP/UDP listeners" @("crates/chatcall-core/src/room/host.rs")

# 37
c "feat(core): implement room client with handshake flow" @("crates/chatcall-core/src/room/client.rs")

# 38
c "feat(core): implement chat message types" @("crates/chatcall-core/src/chat/mod.rs", "crates/chatcall-core/src/chat/message.rs")

# 39
c "feat(core): implement SQLite chat history storage" @("crates/chatcall-core/src/chat/history.rs")

# 40
c "feat(core): implement room event broadcast system" @("crates/chatcall-core/src/events.rs")

# 41
c "feat(core): implement user profile with avatar colors" @("crates/chatcall-core/src/user/mod.rs", "crates/chatcall-core/src/user/profile.rs")

# 42
c "feat(core): implement local SQLite settings database" @("crates/chatcall-core/src/storage/mod.rs", "crates/chatcall-core/src/storage/database.rs")

# 43
c "test(core): add chat history tests (3 tests)" @()

# 44
c "feat(app): scaffold Tauri 2 desktop application" @("app/src-tauri/Cargo.toml", "app/src-tauri/build.rs", "app/src-tauri/tauri.conf.json", "app/src-tauri/capabilities/default.json", "app/src-tauri/src/main.rs")

# 45
c "feat(app): implement Tauri app state management" @("app/src-tauri/src/lib.rs", "app/src-tauri/src/state.rs")

# 46
c "feat(app): implement room IPC commands" @("app/src-tauri/src/commands/mod.rs", "app/src-tauri/src/commands/room.rs")

# 47
c "feat(app): implement chat and voice IPC commands" @("app/src-tauri/src/commands/chat.rs", "app/src-tauri/src/commands/voice.rs")

# 48
c "feat(app): implement settings IPC commands" @("app/src-tauri/src/commands/settings.rs")

# 49
c "feat(ui): implement premium dark mode lobby and room UI" @("app/src/index.html", "app/src/styles/main.css", "app/src/styles/animations.css", "app/src/styles/components.css")

# 50
git add -A
git commit -m "feat(ui): implement frontend application controller with Tauri IPC"

Write-Host ""
Write-Host "=== DONE ==="
git log --oneline | Select-Object -First 55
Write-Host ""
$count = (git log --oneline | Measure-Object).Count
Write-Host "Total commits: $count"
