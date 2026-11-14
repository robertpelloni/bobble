# OmniCore scripts

**Use the unified launcher.** The five original single-purpose scripts still work, but everything they do is now available through one entry point (originals backed up in `archive/` — nothing was deleted).

| Task | Windows | Linux / macOS |
|---|---|---|
| Build frontend + Rust workspace | `.\scripts\omnicore.ps1 build` | `./scripts/omnicore.sh build` |
| Test (frontend build + `cargo test --workspace`) | `.\scripts\omnicore.ps1 test` | `./scripts/omnicore.sh test` |
| Dev preview (Vite, port 1420) | `.\scripts\omnicore.ps1 dev` | `./scripts/omnicore.sh dev` |
| Run desktop app + system tray | `.\scripts\omnicore.ps1 run` | `./scripts/omnicore.sh run` |
| Package bundles | `.\scripts\omnicore.ps1 bundle all` | `./scripts/omnicore.sh bundle all` |
| Status (version, tools, git) | `.\scripts\omnicore.ps1 status` | `./scripts/omnicore.sh status` |

Legacy equivalents (kept, archived in `archive/`): `build.sh`, `test.sh`, `dev.sh`, `run.sh`, `bundle.sh`.

## Windows build prerequisite (found 2026-11-13)

`rusqlite` with `bundled-sqlcipher` fails with `Missing environment variable OPENSSL_DIR` unless OpenSSL is discoverable. The PowerShell launcher auto-sets `OPENSSL_DIR=C:\Program Files\OpenSSL-Win64` when present. For raw `cargo` commands, set it first:

```powershell
$env:OPENSSL_DIR = 'C:\Program Files\OpenSSL-Win64'
cargo test --workspace
```

Heavy builds run at BelowNormal process priority so the machine stays responsive.
