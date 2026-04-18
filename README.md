<div align="center">

# 🔊 Lake

### Zero-Server Voice & Chat — Built From Scratch in Rust

[![Rust](https://img.shields.io/badge/Rust-100%25-orange?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue?style=for-the-badge)](LICENSE)
[![Tests](https://img.shields.io/badge/Tests-47%2F47_Passing-brightgreen?style=for-the-badge)](/)]
[![Platform](https://img.shields.io/badge/Platform-Windows-0078D6?style=for-the-badge&logo=windows&logoColor=white)](/)]

**A fully custom peer-to-peer voice & text communication app.  
No WebRTC. No WebSockets. No cloud. No bullshit.**

[Features](#-features) · [Getting Started](#-getting-started) · [Running the App](#-running-the-app) · [Building for Production](#-building-for-production) · [Architecture](#-architecture)

</div>

---

## ✨ Features

🎙️ **Real-time Voice Communication** — Sub-50ms latency with Opus codec  
🔐 **Military-grade Encryption** — X25519 key exchange + ChaCha20-Poly1305 per session  
💬 **Encrypted Text Chat** — Messages encrypted in transit, stored locally in SQLite  
🏠 **Serverless Architecture** — One user hosts, others join. No cloud. No accounts.  
🔑 **Room Codes** — Share a 7-character code instead of an IP address  
🎛️ **Adaptive Audio** — Jitter buffer, voice activity detection, packet loss concealment  
🖥️ **Native Desktop App** — Tauri 2 shell, ~15MB, instant startup  
🎨 **Premium Dark UI** — Glassmorphism, gradient accents, micro-animations  

---

## 🚀 Getting Started

### Prerequisites

Install these before anything else:

| Tool | Version | How to get |
|------|---------|-----------|
| **Rust** | 1.70+ | https://rustup.rs — select the MSVC toolchain |
| **CMake** | 3.5+ | https://cmake.org/download or `winget install Kitware.CMake` |
| **MSVC Build Tools** | Latest | Install "Desktop development with C++" via Visual Studio Installer |

> Verify your setup:
> ```powershell
> rustup --version
> cargo --version
> cmake --version
> ```

---

## 💻 Running the App

### Option 1 — Dev Mode (fastest, for testing)

Opens the app window directly. Recompiles only changed code. Use this during development.

```powershell
# Clone the repo
git clone https://github.com/BIGSUS24/Voxlink.git
cd Voxlink

# Set required environment variables (run these every session, or add to your PowerShell profile)
$env:CARGO_TARGET_DIR = "C:\cargo-target\chatcall"
$env:CMAKE_POLICY_VERSION_MINIMUM = "3.5"

# Run the app
cargo run -p chatcall-app
```

The window opens automatically. First run takes 5–15 minutes to compile all dependencies. **Subsequent runs take ~10 seconds** (incremental).

---

### Option 2 — Standalone Executable (no installer)

Builds a single `.exe` you can run anywhere without Cargo.

```powershell
$env:CARGO_TARGET_DIR = "C:\cargo-target\chatcall"
$env:CMAKE_POLICY_VERSION_MINIMUM = "3.5"

cargo build -p chatcall-app --release
```

Your executable will be at:
```
C:\cargo-target\chatcall\release\chatcall-app.exe
```

Double-click it or run it from anywhere. No install required.

---

### Option 3 — Full Installer (MSI / Setup.exe)

Builds a proper Windows installer that adds Lake to your Start Menu and Programs list.

> ⚠️ Requires `cargo-tauri` CLI:
> ```powershell
> cargo install tauri-cli --version "^2"
> ```

```powershell
$env:CARGO_TARGET_DIR = "C:\cargo-target\chatcall"
$env:CMAKE_POLICY_VERSION_MINIMUM = "3.5"

# From the app/src-tauri directory
cd app/src-tauri
cargo tauri build
```

Installers will be at:
```
C:\cargo-target\chatcall\release\bundle\msi\Lake_0.1.0_x64_en-US.msi
C:\cargo-target\chatcall\release\bundle\nsis\Lake_0.1.0_x64-setup.exe
```

Run either installer to install Lake as a normal Windows app.

---

## 🎮 How to Use

### Hosting a Room
1. Open Lake → enter your name
2. Click **Host a Room**
3. App detects your public IP and displays a **7-character Room Code** (e.g. `B4K9WMR`)
4. Copy it and share it with your friend
5. Click **Leave / Close Room** (top-left) when done — you can re-host immediately

### Joining a Room
1. Open Lake → enter your name  
2. Paste the **Room Code** in the "Room Code" field
3. Click **Join**
4. Done — you're connected directly P2P

### WAN Setup (connecting over the internet)
Lake is serverless — it connects directly between machines. For WAN connections, the **host** must forward these ports on their router:

| Port | Protocol | Purpose |
|------|----------|---------|
| **7770** | TCP | Room control & chat |
| **7771** | UDP | Voice data |

Forward both to the host machine's local IP. The joiner needs no port forwarding.

---

## 🧪 Running Tests

```powershell
$env:CARGO_TARGET_DIR = "C:\cargo-target\chatcall"
$env:CMAKE_POLICY_VERSION_MINIMUM = "3.5"

cargo test --workspace --exclude chatcall-app
```

Expected output:
```
chatcall-net    ✅ 33 tests — protocol, crypto, codec, reliability, room_code
chatcall-audio  ✅ 11 tests — jitter buffer, mixer, voice activity detection
chatcall-core   ✅  3 tests — chat history, SQLite storage
────────────────────────────────────────────────
Total           ✅ 47 tests — ALL PASSING
```

---

## 🏗️ Architecture

Lake is a **Rust workspace** with 4 specialized crates:

```
lake/
├── crates/
│   ├── chatcall-net/       # 🌐 Custom networking protocol & encryption
│   ├── chatcall-audio/     # 🎵 Audio capture, codec, mixing pipeline
│   └── chatcall-core/      # 🧠 Room management, chat, storage
└── app/
    ├── src-tauri/          # ⚡ Tauri 2 desktop backend (Rust)
    └── src/                # 🎨 Frontend (HTML/CSS/JS)
```

### Protocol Stack

```
┌─────────────────────────────────────────────────┐
│                 Application Layer                │
│         Room Management · Chat · Events          │
├─────────────────────────────────────────────────┤
│               Encryption Layer                   │
│    X25519 Key Exchange · ChaCha20-Poly1305       │
├─────────────────────────────────────────────────┤
│              Reliability Layer                   │
│       ACK Tracking · Message Ordering            │
├─────────────────────────────────────────────────┤
│              Transport Layer                     │
│     TCP (Control/Chat) · UDP (Voice Data)        │
├─────────────────────────────────────────────────┤
│              Protocol Layer                      │
│   Custom Binary Format · 8-byte Packet Header    │
└─────────────────────────────────────────────────┘
```

### Voice Pipeline

```
Mic → [20ms frames] → VAD → Opus Encode (32kbps) → Encrypt → UDP ──→ Host
                                                                       │
Speaker ← Mixer ← Opus Decode/PLC ← Jitter Buffer ← Decrypt ← UDP ←──┘
```

### Room Code Algorithm (VoxCode)

When you host, Lake encodes your public IP into a 7-character shareable code using a 3-layer cipher baked into the binary:

```
IP address  →  XOR(KeyA)  →  ByteShuffle  →  XOR(KeyB)  →  Base-36 alphabet  →  "B4K9WMR"
```

Both the host and joiner have the same app, so the decoding keys are always available locally — **no server needed**.

---

## 🛡️ Security

- **No data leaves your network** — everything is peer-to-peer
- **Forward secrecy** — ephemeral X25519 keys per session
- **Authenticated encryption** — ChaCha20-Poly1305 AEAD (same as WireGuard)
- **No telemetry, no analytics, no tracking**
- **All data stored locally** — SQLite on your machine only

---

## 🗺️ Roadmap

- [x] Custom binary protocol with 12 packet types
- [x] X25519 + ChaCha20-Poly1305 encryption
- [x] TCP reliable channel + UDP voice channel
- [x] LAN room discovery
- [x] Opus voice codec with FEC
- [x] Adaptive jitter buffer + PLC
- [x] Voice activity detection
- [x] Multi-user audio mixing
- [x] Room hosting and joining
- [x] Encrypted chat with ACK tracking
- [x] SQLite local chat history
- [x] Tauri 2 desktop app with premium UI
- [x] Windows MSI + NSIS installers
- [x] Serverless Room Codes (7-char VoxCode)
- [ ] NAT traversal (hole punching)
- [ ] Screen sharing
- [ ] File transfer (encrypted P2P)
- [ ] macOS and Linux builds

---

## 📁 Project Structure

```
├── Cargo.toml                          # Workspace config
├── crates/
│   ├── chatcall-net/                   # Networking library
│   │   └── src/
│   │       ├── protocol/               # Binary packet format + codec
│   │       ├── transport/              # TCP + UDP channels
│   │       ├── crypto/                 # X25519 + ChaCha20-Poly1305
│   │       ├── reliability/            # ACK tracking + ordering
│   │       ├── discovery/              # LAN broadcast discovery
│   │       ├── room_code.rs            # VoxCode encode/decode
│   │       └── serialization/          # Binary encode/decode helpers
│   │
│   ├── chatcall-audio/                 # Audio pipeline
│   │   └── src/
│   │       ├── capture.rs              # Mic input (cpal)
│   │       ├── playback.rs             # Speaker output (cpal)
│   │       ├── encoder.rs              # Opus encoder
│   │       ├── decoder.rs              # Opus decoder + PLC
│   │       ├── jitter_buffer.rs        # Adaptive packet reordering
│   │       ├── vad.rs                  # Voice activity detection
│   │       ├── mixer.rs                # Multi-user audio mixer
│   │       └── pipeline.rs             # Full orchestration
│   │
│   └── chatcall-core/                  # Application logic
│       └── src/
│           ├── room/                   # Host + Client + State
│           ├── chat/                   # Messages + SQLite history
│           ├── events.rs               # Broadcast event system
│           ├── user/                   # Profile management
│           └── storage/                # Local database
│
└── app/                                # Desktop application
    ├── src-tauri/                      # Rust backend (Tauri 2)
    │   └── src/
    │       ├── commands/               # IPC command handlers
    │       ├── state.rs                # Managed application state
    │       └── lib.rs                  # App entrypoint
    └── src/                            # Frontend
        ├── index.html                  # Single page layout
        ├── styles/                     # CSS design system
        └── js/app.js                   # Application controller
```

---

## 📜 License

MIT — see [LICENSE](LICENSE) for details.

---

<div align="center">

**Built with 🦀 Rust from scratch**

If you found this useful, drop a ⭐

</div>
