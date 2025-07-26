# Krusty Modular System Overview (2025)

## System Architecture

Krusty is architected as a set of modular Rust crates, each with a clear responsibility and API boundary. This enables robust code reuse, simulation-driven development, and maintainability.

### Crate Structure

- **krusty_host**
  - Main host application: printer orchestration, hardware management, web API, and integration with real hardware.
  - Owns the runtime, state machine, and all OS-level logic.
  - Imports shared logic from `krusty_shared`.
  - Exposes public API types for integration and testing.

- **krusty_mcu**
  - Firmware/embedded code for the microcontroller (MCU).
  - Designed for `embassy-framework` `no_std` and embedded targets (future: STM32, RP2040, AVR, etc.).
  - Will eventually use feature flags for target selection.
  - Integrates only the subset of `krusty_shared` that is `no_std` compatible.

- **krusty_simulator**
  - Full-featured simulation harness for virtual hardware, motion, and event-driven testing.
  - Uses a fake/emulated MCU for simulation (not the real MCU code).
  - Leverages all shared logic from `krusty_shared` and mimics the host's orchestration.
  - Provides CLI tools for batch testing, parameter sweeps, and CI.

- **krusty_shared**
  - Pure library crate: all logic, types, traits, and utilities shared by host, simulator, and (where possible) MCU.
  - Contains: G-code parsing, hardware traits, board config, motion/trajectory planning, event queue, API models, auth traits, and utility types.
  - No application, firmware, or simulation logic—only reusable, hardware-agnostic code.

### Inter-Crate API Boundaries

- All cross-cutting logic (G-code, motion, hardware traits, event queue, API models, auth, etc.) lives in `krusty_shared`.
- Host and simulator import from `krusty_shared` and add their own orchestration, state, and I/O.
- MCU imports only the subset of `krusty_shared` that is `no_std` compatible (future work).
- Simulator uses host logic for orchestration, but swaps in virtual hardware

### Extension Points

- **Hardware Abstraction:** All hardware interfaces are trait-based and live in `krusty_shared::hardware_traits`.
- **Motion Planning:** Pluggable planners, kinematics, and input shapers are defined in `krusty_shared` and selected/configured by the host/simulator.
- **Web/API:** API models and auth traits are shared; host provides the actual web server.
- **Simulation:** Event queue and simulation clock are shared; simulator provides CLI and harness logic.

---

# Migration Plan: Sharing Logic Between krusty_host and krusty_simulator

## Modular Architecture Overview

The Krusty project is organized into four clearly defined crates:

- **krusty_host**: Contains all host OS logic, including printer orchestration, hardware management, web API, and integration with real hardware. This is the main application for running a 3D printer.
- **krusty_mcu**: Contains the code and build system to generate firmware binaries for the microcontroller (MCU) that runs on the printer hardware. This crate is focused on embedded/firmware logic and uses `embassy` and `no_std` Rust, which is not compatible with the simulator.
- **krusty_simulator**: Contains all simulation logic, including virtual hardware, simulated motion, and test harnesses. The simulator uses a fake/emulated MCU implemented in a crate called `krusty_vcu` (Virtual Control Unit) inside the simulator, because the real MCU code is not compatible with the simulation environment. The simulator also leverages the logic of krusty_host to simulate the printer, enabling development and testing without real hardware.
- **krusty_shared**: A pure library crate containing all logic, types, traits, and utilities that must be shared between krusty_host and krusty_simulator. No application or firmware logic should live here—only reusable, hardware-agnostic code.

This modular structure ensures a single source of truth for core logic, maximizes code reuse, and enables robust CI and simulation workflows.

## Rationale

Centralizing shared logic in `krusty_shared`:
- Reduces code duplication
- Ensures consistent behavior between host and simulator
- Simplifies maintenance and future feature development
- Enables simulator-first and CI-driven development workflows
- Keeps application, firmware, and simulation logic cleanly separated