# Krusty-RS - Rust 3D Printer Host OS

A modern, high-performance 3D printer host software written in Rust.

---

## Workspace Architecture (Post-Refactor)

This repository is now a Cargo workspace with the following crates:

- **krusty_host**: Main host logic, G-code processing, motion control, hardware abstraction, and web API.
- **krusty_simulator**: Simulation harness and event queue, depends on both `krusty_host` and `krusty_mcu` for integration testing and simulation.
- **krusty_mcu**: Placeholder for MCU firmware (currently fake/emulated for simulator integration; will support real MCU targets in the future).
- **krusty_shared**: Shared traits, types, and abstractions used by host, simulator, and MCU crates.

### Directory Structure

```
krusty-rs/
│  Cargo.toml (workspace)
│  README.md
│  refactor_plan.md
├─ krusty_host/
│    Cargo.toml
│    src/
├─ krusty_mcu/
│    Cargo.toml
│    src/
├─ krusty_simulator/
│    Cargo.toml
│    src/
├─ krusty_shared/
│    Cargo.toml
│    src/
├─ memory_bank/
├─ docs/
├─ tests/
```

### Crate Responsibilities

- **krusty_host**: Implements all host-side logic, including G-code parsing, motion planning, hardware management, and web API endpoints.
- **krusty_simulator**: Provides a simulation environment for testing host and MCU logic together, using fake or real MCU code.
- **krusty_mcu**: Will eventually provide real firmware for multiple MCU targets; currently contains a fake/emulated MCU for simulation.
- **krusty_shared**: Contains traits and types shared across host, simulator, and MCU crates (e.g., motion traits, authentication traits).

---

## Features

- Async/await architecture for high performance
- TOML configuration files
- G-code processing
- Motion control system
- Hardware communication layer

## Building

```bash
cargo build
```
