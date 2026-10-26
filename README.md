# TPT Fleet Autonomy Suite

An offline / local-network desktop environment for designing, simulating, deploying and
managing fleets of robots and drones. See `spec.txt` for the full design and `TODO.md` for
the phased plan.

Licensed under MIT OR Apache-2.0 (see `LICENSE-MIT` and `LICENSE-APACHE`).

## Requirements

- Rust stable (pinned in `rust-toolchain.toml`, installed automatically by `rustup`)
- Windows, macOS or Linux
- Optional: `cargo-deny` for the licence check (`cargo install cargo-deny`)

## Build

```sh
cargo build --workspace
```

## Test and lint

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check licenses
```

## Layout

| Crate | Purpose |
|-------|---------|
| `fleet-core` | Fleet Project document model |
| `fleet-store` | Keystone-backed storage |
| `fleet-sim` | Simulation (`SimBackend` trait + built-in backend) |
| `fleet-skills` | Skill registry and deployment |
| `fleet-adapters` | Robot adapters (TCP/UDP, MAVLink) |
| `fleet-caps` | Capability-based permissions |
| `fleet-perception` | Sensor views and fusion |
| `fleet-package` | `.fleet` container, signing, encryption |
| `fleet-license` | Offline licence key verification |
| `fleet-app` | Desktop application |

All crates are currently stubs; see `TODO.md` for what each phase adds.
