---
name: storage
description: "Skill for the Storage area of planarclip. 5 symbols across 2 files."
---

# Storage

5 symbols | 2 files | Cohesion: 83%

## When to Use

- Working with code in `Apps/`
- Understanding how run, config_path, load_config work
- Modifying storage-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `Apps/planarclip/src-tauri/src/storage/json.rs` | config_path, dirs_next, load_config, save_config |
| `Apps/planarclip/src-tauri/src/lib.rs` | run |

## Entry Points

Start here when exploring this area:

- **`run`** (Function) — `Apps/planarclip/src-tauri/src/lib.rs:122`
- **`config_path`** (Function) — `Apps/planarclip/src-tauri/src/storage/json.rs:22`
- **`load_config`** (Function) — `Apps/planarclip/src-tauri/src/storage/json.rs:43`
- **`save_config`** (Function) — `Apps/planarclip/src-tauri/src/storage/json.rs:55`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `run` | Function | `Apps/planarclip/src-tauri/src/lib.rs` | 122 |
| `config_path` | Function | `Apps/planarclip/src-tauri/src/storage/json.rs` | 22 |
| `load_config` | Function | `Apps/planarclip/src-tauri/src/storage/json.rs` | 43 |
| `save_config` | Function | `Apps/planarclip/src-tauri/src/storage/json.rs` | 55 |
| `dirs_next` | Function | `Apps/planarclip/src-tauri/src/storage/json.rs` | 28 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Run → Dirs_next` | intra_community | 4 |
| `Pair → Dirs_next` | cross_community | 4 |
| `Run → KeyPair` | cross_community | 3 |
| `Run → Generate` | cross_community | 3 |
| `Run → KeyPairData` | cross_community | 3 |
| `Run → Public_bytes` | cross_community | 3 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Crypto | 1 calls |

## How to Explore

1. `gitnexus_context({name: "run"})` — see callers and callees
2. `gitnexus_query({query: "storage"})` — find related execution flows
3. Read key files listed above for implementation details
