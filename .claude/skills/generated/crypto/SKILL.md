---
name: crypto
description: "Skill for the Crypto area of planarclip. 7 symbols across 3 files."
---

# Crypto

7 symbols | 3 files | Cohesion: 83%

## When to Use

- Working with code in `Apps/`
- Understanding how fingerprint, connect, generate work
- Modifying crypto-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `Apps/planarclip/src-tauri/src/crypto/keys.rs` | fingerprint, generate, public_bytes |
| `Apps/planarclip/src-tauri/src/lib.rs` | get_pairing_code, pair, load_or_create_key_pair |
| `Apps/planarclip/src-tauri/src/network/webrtc.rs` | connect |

## Entry Points

Start here when exploring this area:

- **`fingerprint`** (Function) — `Apps/planarclip/src-tauri/src/crypto/keys.rs:19`
- **`connect`** (Function) — `Apps/planarclip/src-tauri/src/network/webrtc.rs:58`
- **`generate`** (Function) — `Apps/planarclip/src-tauri/src/crypto/keys.rs:9`
- **`public_bytes`** (Function) — `Apps/planarclip/src-tauri/src/crypto/keys.rs:15`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `fingerprint` | Function | `Apps/planarclip/src-tauri/src/crypto/keys.rs` | 19 |
| `connect` | Function | `Apps/planarclip/src-tauri/src/network/webrtc.rs` | 58 |
| `generate` | Function | `Apps/planarclip/src-tauri/src/crypto/keys.rs` | 9 |
| `public_bytes` | Function | `Apps/planarclip/src-tauri/src/crypto/keys.rs` | 15 |
| `get_pairing_code` | Function | `Apps/planarclip/src-tauri/src/lib.rs` | 43 |
| `pair` | Function | `Apps/planarclip/src-tauri/src/lib.rs` | 60 |
| `load_or_create_key_pair` | Function | `Apps/planarclip/src-tauri/src/lib.rs` | 100 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Pair → Dirs_next` | cross_community | 4 |
| `Run → KeyPair` | cross_community | 3 |
| `Run → Generate` | cross_community | 3 |
| `Run → KeyPairData` | cross_community | 3 |
| `Run → Public_bytes` | cross_community | 3 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Storage | 1 calls |

## How to Explore

1. `gitnexus_context({name: "fingerprint"})` — see callers and callees
2. `gitnexus_query({query: "crypto"})` — find related execution flows
3. Read key files listed above for implementation details
