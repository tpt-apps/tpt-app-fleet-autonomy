# Fleet (Fleet Autonomy Suite) — TODO

Source: `spec.txt` (section refs in parentheses), `GUMROAD.md`. Tick boxes as you go: `- [x]`.

## Decisions locked in

- Desktop shell: pure `tpt-appfront` native, no Tauri (§10).
- Layout: single Cargo workspace, crates under `crates/` (proposed: `fleet-core`, `fleet-store`, `fleet-sim`, `fleet-skills`, `fleet-adapters`, `fleet-caps`, `fleet-perception`, `fleet-package`, `fleet-license`, `fleet-app`).
- Foundation crates (`tpt-swarm`, `tpt-perception`, `tpt-control`, `tpt-keystone-db`, `tpt-appfront`, `tpt-anima`, `tpt-math`, `tpt-primitives`, `tpt-crypto`) come from git, pinned by `rev`: `github.com/TPT-Solutions/<crate>`. Anything missing or broken is wrapped behind a trait in `fleet-core` and logged in `docs/ARCHITECTURE.md`.
- Simulation: built-in lightweight sim plus a `SimBackend` trait so Gazebo or others can plug in later (§10).
- ROS2: none in v1. The hardware-adapter trait is designed so a ROS2 adapter can be added later (§10).
- Federated learning: minimal in v1. Skill/model version sync through the registry, plus a simple demo (§10, `GUMROAD.md` "basic shared learning").
- Real-hardware adapters in v1: generic TCP/UDP adapter and MAVLink (drones). Everything else is simulated.
- Package format: zip-style container, extension `.fleet`: manifest + robots + skills + worlds + Keystone snapshot; ed25519-signed; optional ChaCha20-Poly1305 with Argon2id passphrase KDF (§6, §10). Mirrors `tpt-app-digital-twin`.
- Licence key: offline ed25519-signed key file, verified locally. Open-source build fully functional without it; official binaries show a banner only.
- Platforms: Windows, macOS, Linux (§4). Dual licence MIT OR Apache-2.0; no Apache-only deps (§5).
- No telemetry, no accounts, no phone-home. LAN-only by default (§6).
- Worlds: 3D viewport. Supports both ground robots and drones.

## Open decisions (default shown; change if wrong)

- [ ] Exact foundation-crate URLs and commits — default: `github.com/TPT-Solutions/<crate>`, pin to master on the day of audit
- [ ] Do `tpt-swarm` and `tpt-anima` exist and compile? — decided in Phase 0 audit
- [ ] Rust edition / MSRV — default: edition 2021, latest stable
- [ ] GitHub org for this repo — default: `tpt-apps` (as in Twin)
- [ ] Rendering backend for the 3D viewport — depends on `tpt-appfront` (Twin spike: egui paint callbacks, `glow` vs `wgpu`)
- [ ] Dependency licence checker — default: `cargo-deny`
- [ ] Wire protocol for the generic TCP/UDP adapter — default: length-prefixed CBOR or JSON messages, defined in `docs/ADAPTER_PROTOCOL.md`
- [ ] Capability model shape — default: explicit allow-list per robot/agent (e.g. `move`, `actuate:gripper`, `sensor:read:lidar`, `deploy`), enforced at the adapter boundary
- [ ] Sim fidelity — default: kinematic ground robots + simple quadrotor dynamics, point-cloud/lidar sensor models
- [ ] Gumroad price, licence-key issuing flow — default: $429 as in `GUMROAD.md`, keys generated offline by a separate tool
- [ ] `tpt-swarm` is Apache-2.0 only (spec §5 needs dual licence) — default: wrap it behind a trait in `fleet-core` and log it in `docs/ARCHITECTURE.md`; alternatively ask the owner for a dual licence
- [ ] Copyright holder in `LICENSE-MIT` — default: "The TPT Fleet Autonomy Suite contributors"; replace with the legal entity name if wanted

## Phase 0 — Workspace & groundwork

- [x] Cargo workspace root, `rust-toolchain.toml`, `.gitignore`
- [x] `LICENSE-MIT`, `LICENSE-APACHE`
- [x] Stub crates (build, clippy and test pass)
- [x] CI workflow for Windows, macOS, Linux
- [x] `git init` (local repo, nothing committed)
- [ ] First commit (after your go-ahead)
- [ ] Create the GitHub remote (after your go-ahead; confirm org)
- [~] Clone and audit the foundation crates: `tpt-swarm`, `tpt-perception`, `tpt-control`, `tpt-keystone-db`, `tpt-appfront`, `tpt-anima`, `tpt-math`, `tpt-primitives`, `tpt-crypto` (existence, licence, build on Windows/macOS/Linux) — existence and licence done, Windows build done (3 failing), macOS/Linux pending. See `docs/ARCHITECTURE.md`.
- [~] Record their public APIs and gaps in `docs/ARCHITECTURE.md` — audit status and blockers recorded; public APIs not yet recorded.
- [~] Pin git dependencies by `rev` in `[workspace.dependencies]` — `tpt-math-numeric` pinned at `a6c659d`; `tpt-perception` and `tpt-swarm` not yet pinned (they do not build yet, see `docs/ARCHITECTURE.md`)
- [x] `deny.toml` and `cargo deny check licenses` passes
- [ ] Confirm foundation crates build with default features off (no cloud, S3, OTLP, etc.)
- [x] README with build instructions
- [ ] Spike: 3D viewport renderer inside `tpt-appfront`

## Phase 1 — Fleet document & storage (`fleet-core`, `fleet-store`)

- [x] Define the Fleet Project: robot definitions, skills, worlds, deployment configs, observed state (§3)
- [x] Robot definition: id, type (ground/drone), sensors, actuators, adapter, capabilities
- [x] Stable IDs (UUID) and serde serialisation; `schema_version` field, refuses newer schemas
- [ ] Keystone-backed store: fleet state, logs, skill versions, experiment data (§3) — not started; a file-backed store is in place. Keystone is server-first with an undocumented library surface, so it needs its own step.
- [x] Open / create / save / load / list / delete a project in a local directory
- [x] Migration hook (passthrough while only v1 exists)
- [~] Unit tests: round-trip, validation, NaN, schema refusal (done for `fleet-core` and the file store); store tests against the real Keystone engine pending
- [x] History: append-only event log per robot/project

## Phase 2 — Capability-based permissions (`fleet-caps`)

- [x] Capability type and per-robot / per-agent grants (§4, §6)
- [~] Enforcement point at the adapter and deployment boundary (deny by default) — `Policy::authorize` is the check; wiring it into adapters and deployment happens in Phases 4 and 6
- [~] Audit log of allowed and denied actions in Keystone — in-memory audit log in place; Keystone persistence pending
- [x] Unit tests: deny by default, grant, revoke, escalation attempts
- [x] Capability review UI data model (`Policy::grants_for`)

## Phase 3 — Simulation (`fleet-sim`)

- [x] `SimBackend` trait (step, spawn, sensors, world state) — spawn, command, step, state, agents, time; sensors are free functions
- [~] Built-in backend: 3D world, ground robot kinematics, simple quadrotor dynamics — unicycle ground model and point-mass drone with gravity, integrated by `tpt-math-numeric::ode::rk4_step`. Obstacles are axis-aligned boxes, not a full 3D world. `tpt-control` not used.
- [x] World/scenario definitions (obstacles, start poses, goals, duration)
- [~] Sensor models: lidar / point cloud, pose, basic camera stub — horizontal lidar and noisy pose done; point cloud and camera stub not done
- [x] Deterministic runs (seeded) and scenario replay
- [~] Store run results in Keystone — saved to `runs/<name>.json` in the project directory (file store); Keystone pending
- [x] Tests: determinism, collision, control convergence

## Phase 4 — Skill registry & deployment (`fleet-skills`, `fleet-adapters`)

- [ ] Skill definition, versioning and registry (via `tpt-swarm`)
- [ ] Declared capabilities per skill; deployment refused if robot lacks them
- [ ] Deploy a skill to a simulated robot
- [ ] `RobotAdapter` trait (connect, send command, stream telemetry, disconnect)
- [ ] Generic TCP/UDP adapter + `docs/ADAPTER_PROTOCOL.md`
- [ ] MAVLink adapter (drones)
- [ ] Reference robot/mock server for adapter tests
- [ ] LAN discovery and binding: loopback by default, explicit opt-in for LAN, authenticated sessions (no default tokens)
- [ ] Minimal federated learning: skill/model parameter version sync + simple demo
- [ ] Light multi-agent negotiation via `tpt-anima` (task allocation between agents)

## Phase 5 — Perception & control views (`fleet-perception`)

- [ ] Sensor visualisation: point cloud, pose, telemetry plots (`tpt-perception`)
- [ ] Basic multi-sensor fusion view and spatial registration
- [ ] State estimation and a basic controller/MPC helper hooked to sim and adapters (`tpt-control`)
- [ ] Tests on recorded and synthetic data

## Phase 6 — Packaging, encryption, licence (`fleet-package`, `fleet-license`)

- [ ] `.fleet` container: manifest + contents + Keystone snapshot
- [ ] ed25519 signing and verification
- [ ] Optional encryption: ChaCha20-Poly1305, Argon2id passphrase KDF
- [ ] Export / import of a complete project, clean round-trip between machines (§9)
- [ ] Reject tampered, truncated or newer-schema packages
- [ ] Licence key file: offline ed25519 verification, banner-only behaviour; key-generation tool (separate, not shipped)
- [ ] Tests: round-trip, tamper, wrong passphrase, cross-platform

## Phase 7 — Desktop app (`fleet-app`)

- [ ] `tpt-appfront` shell: window, navigation, theme
- [ ] Fleet overview dashboard (robot status cards, map / 3D view)
- [ ] Robot and fleet editor
- [ ] Scenario / simulation panel with 3D viewport
- [ ] Skill registry and deployment UI
- [ ] Capability editor
- [ ] Sensor and fusion views
- [ ] Project import / export dialogs
- [ ] Log / history browser
- [ ] Offline / LAN settings
- [ ] Example fleet projects and skills

## Phase 8 — Release & distribution (§7)

- [ ] Verify end-to-end success criteria (§9): define a fleet, simulate, deploy a skill, move the project to another machine, all without cloud
- [ ] Confirm no network calls beyond the user's own LAN (no telemetry)
- [ ] Binaries for Windows, macOS, Linux, signed
- [ ] Installer per platform
- [ ] Quick-start guide (PDF)
- [ ] Licence terms text and README open-source build instructions
- [ ] Gumroad listing from `GUMROAD.md`, zip with binaries + licence key + docs + examples
- [ ] Error-report intake process (§7)

## Later (no commitment, §4)

- [ ] Richer federated learning workflows
- [ ] Advanced perception pipelines
- [ ] Visual skill and behaviour editors
- [ ] Stronger multi-agent negotiation
- [ ] Hardware-in-the-loop improvements
- [ ] Headless fleet server mode
- [ ] ROS2 adapter / URDF and bag import
- [ ] Gazebo `SimBackend`
