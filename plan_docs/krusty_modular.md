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

---

# Findings: Logic in `krusty_host` Suitable for `krusty_shared`

## 1. G-code Parsing and Macro Expansion
- **Files:** `gcode/parser.rs`, `gcode/macros.rs`, `gcode/mod.rs`
- **Why:** G-code parsing, macro expansion, and command models are hardware-agnostic and required by both host and simulator. These should be unified in `krusty_shared` for consistent parsing and macro behavior.

## 2. Hardware Traits and Board Config
- **Files:** `hardware/hardware_traits.rs`, `hardware/board_config.rs`
- **Why:** Traits and config types for hardware abstraction are already referenced in `krusty_shared`. Ensure all trait definitions and board config types are defined in `krusty_shared`, not duplicated in host.

## 3. Motion Planning, Kinematics, and Junction Logic
- **Files:** `motion/kinematics.rs`, `motion/junction.rs`, `motion/planner/`, `motion/controller.rs`
- **Why:** Core motion planning, kinematics, and junction deviation logic are reusable for both real and simulated hardware. These should be moved to `krusty_shared` as pure logic modules.

## 4. Event Queue and Event Interface
- **Files:** `communication/event_interface.rs`, `communication/event_system.rs`
- **Why:** Event queueing and event trait definitions are cross-cutting concerns. Move trait definitions and pure event types to `krusty_shared`.

## 5. Print Job Management
- **Files:** `print_job.rs`
- **Why:** Print job state, job queueing, and job models are logic that can be reused by both host and simulator. Move core types and logic to `krusty_shared`.

## 6. File Management Utilities
- **Files:** `file_manager.rs`
- **Why:** File info types and pure file utilities (not OS-specific logic) are reusable. Move generic file models and helpers to `krusty_shared`.

## 7. API Models and Auth Traits
- **Files:** `web/models.rs`, `web/auth.rs`
- **Why:** API data models and authentication trait definitions are shared between host and simulator (and potentially web clients). Move these to `krusty_shared`.

## 8. System Info and Utility Types
- **Files:** `system_info.rs`
- **Why:** System info types and utility structs that are not OS-specific should be moved to `krusty_shared`.

---

# Migration Plan: Host → Shared

```markdown
- [ ] 1. Move G-code parsing, macro expansion, and command models to `krusty_shared`
- [ ] 2. Move all hardware trait definitions and board config types to `krusty_shared`
- [ ] 3. Move motion planning, kinematics, and junction logic to `krusty_shared`
- [ ] 4. Move event queue traits and pure event types to `krusty_shared`
- [ ] 5. Move print job state and queue logic to `krusty_shared`
- [ ] 6. Move generic file info types and helpers to `krusty_shared`
- [ ] 7. Move API models and auth traits to `krusty_shared`
- [ ] 8. Move system info and utility types to `krusty_shared`
- [ ] 9. Refactor host and simulator to use shared logic from `krusty_shared`
- [ ] 10. Remove any duplicated or obsolete logic from `krusty_host`
- [ ] 11. Validate with tests and CI for both host and simulator
```

**Note:** Only pure, hardware-agnostic, and reusable logic should be moved. OS-specific, hardware-specific, or application orchestration code must remain in `krusty_host`.

---

This plan ensures a clean separation of concerns, maximizes code reuse, and aligns with the modular architecture described above.