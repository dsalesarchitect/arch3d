# Security Policy — Arch3D

## Zero Telemetry & Zero Data Tracking Guarantee

Arch3D is a clean-room, local-first, open-source BIM application. It contains **zero analytics, zero telemetry, zero ads, and zero hidden user data collection**.

### Authorized Network & Local Connections Only
The only network or local IPC endpoints permitted in the Arch3D codebase are:
1. **GitHub Releases Auto-Updater (User-Initiated Only)**: `https://api.github.com/repos/dsalesarchitect/arch3d/releases/latest` and official release downloads from `https://github.com/dsalesarchitect/arch3d/` or `https://objects.githubusercontent.com/` over TLS 1.2+.
2. **Blender Live Sync (Localhost Only)**: `127.0.0.1:9876` (never leaves your computer).
3. **External Links (User-Clicked Only)**: Official links to `https://github.com/dsalesarchitect/arch3d`, `https://dsalesarchitect.github.io/arch3d/`, `https://www.youtube.com/@OpenSourceArchitecture`, and `https://mutracer-dev.meimu-studio.com/`.

Any pull request introducing unauthorized network calls, analytics, trackers, `build.rs` scripts, obfuscated payloads, or executable binaries will be **automatically blocked** by CI and rejected.

---

## Reporting a Vulnerability

If you discover a security vulnerability in Arch3D or a suspicious dependency:
- **Do NOT open a public GitHub issue** with exploit details.
- Email the maintainer directly at **dsalesarchitect@gmail.com** or use GitHub's **Private Vulnerability Reporting** ("Security" tab -> "Report a vulnerability").
- Please include steps to reproduce and affected file paths or crate versions.

---

## Contributor Security Requirements

All Pull Requests must pass:
1. **Code Owner Approval** (`@dsalesarchitect`).
2. **Arch3D Security Gate (`security-audit.yml`)**: Automated scan for unauthorized network trackers, shell commands, `build.rs` scripts, Trojan Source Unicode characters, new `unsafe` blocks, and binary drops.
3. **RustSec & Supply-Chain Audit**: `cargo-audit` and `cargo-deny` (`deny.toml`).
4. **Full Workspace Test Suite**: `cargo test --workspace`.
