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

---

## Overview

This document identifies functions, modules, and logic in `krusty_host` that should be migrated to `krusty_shared` to enable code reuse and maintain a single source of truth for core 3D printer logic, simulation, and hardware abstraction.

---

## Candidate Modules/Functions for Migration

### 1. G-code Parsing and Utilities
- **Current Location:** `krusty_host::gcode::*`
- **Rationale:** Both host and simulator need to parse, interpret, and process G-code lines, including macro expansion and context management.
- **Action:** Move core G-code parsing logic, macro processor, and context utilities to `krusty_shared::gcode_utils`.  
- **Status:** ✅ Completed

### 2. Hardware Abstractions & Board Config
- **Current Location:** `krusty_host::hardware::board_config`, `hardware::HardwareManager`
- **Rationale:** Board configuration, pin mapping, and hardware abstraction traits are required for both real and simulated hardware.
- **Action:** Move board abstraction traits, config structs, and pin mapping logic to `krusty_shared::hardware`.  
- **Status:** ✅ Completed

### 3. Motion Planning Traits & Types
- **Current Location:** `krusty_host::motion::*`
- **Rationale:** Motion planning, kinematics, and trajectory generation are needed for both real execution and simulation.
- **Action:** Move traits, error types, and core motion planning logic to `krusty_shared::trajectory` and `krusty_shared::motion`.  
- **Status:** ✅ Completed

### 4. Event Queue & Simulation Clock
- **Current Location:** Already in `krusty_shared::event_queue`
- **Rationale:** Both host and simulator use the event queue and simulation clock for event-driven execution.
- **Action:** Ensure all event types and queue logic are fully migrated and remove any duplicate code in `krusty_simulator`.  
- **Status:** ✅ Centralized in `krusty_shared::event_queue`

### 5. API Models
- **Current Location:** Already in `krusty_shared::api_models`
- **Rationale:** Shared API request/response types for web and simulator endpoints.
- **Action:** Continue to re-export and extend as needed.  
- **Status:** ✅ Up to date

### 6. Temperature, Heater, Fan, and Switch State
- **Current Location:** `krusty_host::hardware::*`, `krusty_simulator::simulator::*`
- **Rationale:** Both host and simulator need to model and update hardware state (heaters, fans, switches).
- **Action:** Move state structs and update logic to `krusty_shared::hardware`.  
- **Status:** ✅ Completed

### 7. Auth Traits
- **Current Location:** `krusty_host::web::auth`
- **Rationale:** Auth backends may be used in both host and simulator (for web API, test harnesses).
- **Action:** Move `AuthBackend` trait and basic implementations to `krusty_shared`.  
- **Status:** ✅ Completed (see `krusty_shared::auth_backend`)

### 8. Utility Types and Traits
- **Current Location:** Scattered in `krusty_host` and `krusty_simulator`
- **Rationale:** Traits like `TimeInterface`, `Kinematics`, and input shaping are already in `krusty_shared`, but ensure all utility traits are centralized.
- **Action:** Audit and migrate any missing utility traits.  
- **Status:** ✅ Completed

---

## Migration Steps

- [x] Inventory all logic in `krusty_host` that is duplicated or needed in `krusty_simulator`.
- [x] Move G-code parsing, macro, and context logic to `krusty_shared::gcode_utils`.
- [x] Move board config, hardware traits, and state structs to `krusty_shared::hardware`.
- [x] Move motion planning traits, error types, and core logic to `krusty_shared::trajectory` and `krusty_shared::motion`.
- [x] Ensure all event queue and simulation clock logic is only in `krusty_shared::event_queue`.
- [x] Move `AuthBackend` trait and basic implementations to `krusty_shared`.
- [x] Audit for any remaining utility types/traits and centralize in `krusty_shared`.
- [x] Update imports in both `krusty_host` and `krusty_simulator` to use the new shared modules.
- [x] Remove any now-duplicate code from `krusty_host` and `krusty_simulator`.
- [x] Test both host and simulator to ensure all shared logic works as intended.

**All shared logic is now centralized in `krusty_shared`. The codebase is deduplicated and ready for further modular development.**

---

## Rationale

Centralizing shared logic in `krusty_shared`:
- Reduces code duplication
- Ensures consistent behavior between host and simulator
- Simplifies maintenance and future feature development
- Enables simulator-first and CI-driven development workflows
- Keeps application, firmware, and simulation logic cleanly separated

---

## Next Steps

- [x] Review this plan with the team.
- [x] Complete incremental migration, validating after each step.
- [x] Migrate temperature, heater, fan, and switch state structs and logic to `krusty_shared::hardware`.
- [x] Update this document as new shared needs are discovered.

**Migration complete. All shared logic is now centralized in `krusty_shared`. Future shared features should be added to `krusty_shared` first, and all crates should import from there to maintain a single source of truth.**

---
