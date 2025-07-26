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

## Findings: Logic to Move from `krusty_host` to `krusty_shared`

### 1. Motion Planning and Kinematics
- **MotionPlanner**, **MotionSegment**, **PlannerState**, and related logic in `motion/planner/mod.rs`:
  - All core motion planning algorithms, segment structures, and config-driven shaper/blending logic are hardware-agnostic and should be shared.
  - The kinematics trait and instantiation should be unified with the shared crate.
- **Adaptive Motion Planning** in `motion/planner/adaptive.rs`:
  - Types such as `AdaptiveConfig`, `PerformanceData`, `OptimizationParams`, `ResonancePeak`, `AdaptiveOptimizer`, `PerformanceMetrics`, `TrainingSample`, `PerformanceMonitor`, `ErrorPredictor`, `VibrationAnalysis`, and `VibrationAnalyzer` are all reusable, hardware-agnostic logic.
  - The `AdaptiveMotionPlanner` wrapper and its logic should also be shared.
- **MotionController** in `motion/controller.rs`:
  - The core logic for queueing, updating, and managing motion should be abstracted for reuse by both host and simulator.
  - The `MotionMode` enum and feature toggling logic are also candidates for sharing.

### 2. G-code Processing
- **GCodeProcessor** in `gcode/mod.rs`:
  - The logic for parsing, expanding, and dispatching G-code commands is hardware-agnostic and should be shared.
  - The `GCodeExecutor` trait in `gcode/gcode_executor.rs` is a pure interface and should be moved to shared.

### 3. File Management
- **FileManager** in `file_manager.rs`:
  - The logic for reading, writing, and listing files, as well as processing G-code files, is generic and can be shared (with async/await support).

### 4. Hardware Traits and State
- **HardwareManager** in `hardware/mod.rs`:
  - The struct itself is hardware-specific, but the error types (`HardwareError`), state types (`FanState`, `ThermistorState`, `HeaterState`), and command statistics (`CommandStats`) are generic and should be shared.
  - Any trait-based abstractions for hardware should be unified in `krusty_shared::hardware_traits`.

### 5. Statistics and State Types
- **QueueStats** in `motion/mod.rs` and **CommandStats** in `hardware/mod.rs`:
  - These are generic statistics types and should be shared.

### 6. Configuration Types
- Any configuration structures or enums that are referenced by both host and simulator (e.g., shaper types, planner config) should be defined in `krusty_shared::config`.

---

## Findings: Logic to Move from `krusty_host` to `krusty_shared` (2025 Review)

Based on a thorough review of the `krusty_host` codebase and the modularization guidelines, the following logic and types should be migrated to `krusty_shared` to maximize code reuse, ensure simulation/host parity, and maintain a clean separation of concerns:

### 1. Motion Planning and Kinematics

- **QueueStats struct** (`motion/mod.rs`):  
  - Generic statistics for the motion queue.  
  - Should be moved to `krusty_shared::motion` for use by both host and simulator.

- **MotionMode enum** (`motion/controller.rs`):  
  - Represents planner feature toggles (Basic, Adaptive, SnapCrackle).  
  - Should be defined in `krusty_shared::motion` for unified feature selection.

- **MotionController struct and core logic** (`motion/controller.rs`):  
  - The core queueing, updating, and management logic is hardware-agnostic and should be abstracted for reuse.
  - Any host-specific dependencies (e.g., hardware manager) should be injected via traits or interfaces.

### 2. G-code Processing

- **GCodeExecutor trait** (`gcode/mod.rs`):  
  - Pure interface for executing parsed G-code commands.  
  - Should be moved to `krusty_shared::gcode`.

- **GCodeProcessor struct and core logic** (`gcode/mod.rs`):  
  - Parsing, macro expansion, and command dispatch logic is generic and should be shared.
  - Host-specific state (e.g., PrinterState) should be abstracted.

### 3. File Management

- **FileManager struct and logic** (`file_manager.rs`):  
  - Reading, writing, and listing files, as well as G-code file processing, is generic and can be shared.
  - Any OS-specific logic should be isolated behind traits.

### 4. Hardware Traits and State

- **HardwareError, FanState, ThermistorState, HeaterState, CommandStats** (`hardware/mod.rs`):  
  - Error types and state representations are generic and should be defined in `krusty_shared::hardware_traits` or `krusty_shared::hardware`.
  - `CommandStats` struct for hardware command statistics should be shared.

### 5. Statistics and State Types

- **PrinterState struct** (`host_os.rs`):  
  - Represents the printer's runtime state (position, temperature, progress, etc.).
  - Should be moved to `krusty_shared::api_models` or a new `state` module for use by both host and simulator.

### 6. Configuration Types

- **All configuration structures and enums referenced by both host and simulator** (`config.rs`):  
  - Shaper types, planner config, blending options, etc., should be defined in `krusty_shared::config`.
  - Host should only re-export or extend these as needed.

### 7. Traits and Interfaces

- **Any trait-based abstractions for hardware, motion, or event systems**  
  - Should be unified in `krusty_shared` to ensure both host and simulator can implement and use them.

---

## Findings: Additional Logic to Move from `krusty_host` to `krusty_shared` (2025 Deep Review)

Based on a detailed review of the current `krusty_host` codebase, the following logic and types should be migrated to `krusty_shared` to maximize code reuse, ensure simulation/host parity, and maintain a clean separation of concerns:

### 1. Motion Planning and Kinematics

- **MotionController struct and core logic** (`motion/controller.rs`):  
  - The core queueing, updating, and management logic is hardware-agnostic and should be abstracted for reuse.
  - Any host-specific dependencies (e.g., hardware manager) should be injected via traits or interfaces.
  - The `MotionMode` enum is already re-exported from `krusty_shared`, but ensure all feature toggling logic is unified.

- **MotionSystem struct** (`motion/mod.rs`):  
  - The struct is a thin wrapper, but any generic orchestration logic should be shared if used by both host and simulator.

### 2. G-code Processing

- **GCodeExecutor trait** (`gcode/mod.rs`):  
  - Pure interface for executing parsed G-code commands.  
  - Should be moved to `krusty_shared::gcode`.

- **GCodeProcessor struct and core logic** (`gcode/mod.rs`):  
  - Parsing, macro expansion, and command dispatch logic is generic and should be shared.
  - Host-specific state (e.g., PrinterState) should be abstracted.

### 3. File Management

- **FileManager struct and logic** (`file_manager.rs`):  
  - Reading, writing, and listing files, as well as G-code file processing, is generic and can be shared.
  - Any OS-specific logic should be isolated behind traits.

### 4. Hardware Traits and State

- **HardwareError, FanState, ThermistorState, HeaterState, CommandStats** (`hardware/mod.rs`):  
  - Error types and state representations are generic and should be defined in `krusty_shared::hardware_traits` or `krusty_shared::hardware`.
  - `CommandStats` struct for hardware command statistics should be shared.

### 5. Statistics and State Types

- **PrinterState struct** (`host_os.rs`):  
  - Represents the printer's runtime state (position, temperature, progress, etc.).
  - Should be moved to `krusty_shared::api_models` or a new `state` module for use by both host and simulator.

### 6. Configuration Types

- **All configuration structures and enums referenced by both host and simulator** (`config.rs`):  
  - Shaper types, planner config, blending options, etc., should be defined in `krusty_shared::config`.
  - Host should only re-export or extend these as needed.

### 7. Traits and Interfaces

- **Any trait-based abstractions for hardware, motion, or event systems**  
  - Should be unified in `krusty_shared` to ensure both host and simulator can implement and use them.

---

## Migration Plan: krusty_host → krusty_shared

- [ ] Move `MotionController` and related feature toggling logic to `krusty_shared::motion`.
- [ ] Move `GCodeExecutor` trait and `GCodeProcessor` core logic to `krusty_shared::gcode`.
- [ ] Move `FileManager` and file processing logic to `krusty_shared::file_manager`.
- [ ] Move `HardwareError`, `FanState`, `ThermistorState`, `HeaterState`, and `CommandStats` to `krusty_shared::hardware_traits` or `hardware`.
- [ ] Move `PrinterState` to `krusty_shared::api_models` or a new `state` module.
- [ ] Ensure all shared configuration types are defined in `krusty_shared::config`.
- [ ] Audit for any additional trait-based abstractions and unify them in `krusty_shared`.

---

**Rationale:**  
This migration will ensure all hardware-agnostic logic is shared, maximize code reuse, and maintain a clean separation between host, simulator, and firmware. Host-specific logic will remain in `krusty_host`, while all reusable types, traits, and logic will live in `krusty_shared`.

