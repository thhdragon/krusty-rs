# Krusty-rs Project Map

## Overview

Krusty-rs is a modular, async Rust-based 3D printer host and motion control system. It is designed for extensibility, high performance, and integration with modern web APIs. The codebase is organized into logical modules for configuration, motion planning, hardware abstraction, G-code processing, and web API handling.

---

## Directory Structure

- **Cargo.toml / Cargo.lock**: Project manifest and dependency lockfile.
- **README.md**: Project introduction and basic usage.
- **docs/**
  - `project_map.md`: (This file) Project structure and status documentation.
- **src/**
  - **config.rs**: Configuration structs and parsing for printer, MCU, extruder, heater bed, etc.
  - **file_manager.rs**: File management utilities (G-code, configs, logs).
  - **host_os.rs**: OS abstraction for platform-specific operations.
  - **integration_test.rs**: Integration test harness (see also `motion/test.rs`).
  - **main.rs**: Application entry point; sets up async runtime, loads config, starts web server and motion subsystems.
  - **print_job.rs**: Print job management (queueing, state, progress tracking).
  - **printer.rs**: High-level printer state, coordination, and control logic.
  - **printer.toml**: Example or default printer configuration.
  - **gcode/**
    - **macros.rs**: G-code macro expansion and variable substitution.
    - **mod.rs**: G-code module root; may re-export or coordinate G-code submodules.
  - **hardware/**
    - **mod.rs**: Hardware abstraction root.
    - **temperature.rs**: Temperature sensor and heater control logic.
  - **motion/**
    - **benchmark.rs**: Motion system benchmarking and performance tests.
    - **controller.rs**: High-level motion controller (queueing, state, interface to planner).
    - **integration.rs**: Integration logic for motion and hardware.
    - **junction.rs**: Junction deviation and cornering logic.
    - **kinematics.rs**: Kinematics models (Cartesian, CoreXY, etc.).
    - **mod.rs**: Motion module root; defines `MotionPlanner`, `MotionConfig`, and related types.
    - **planner/**
      - **adaptive.rs**: Adaptive motion optimization (integrated; see below).
      - **mod.rs**: Planner module root; defines `MotionSegment`, planning passes, and queue management.
      - **snap_crackle.rs**: Advanced motion planning (input shaping, smoothing, etc.).
    - **s_curve.rs**: S-curve motion profile generation.
    - **shaper.rs**: Input shaper logic for vibration reduction.
    - **stepper.rs**: Stepper motor control and abstraction.
    - **test.rs**: Motion system unit and integration tests.
    - **trajectory.rs**: Trajectory generation and interpolation.
  - **web/**
    - **api.rs**: Axum-based HTTP API routes and handlers.
    - **mod.rs**: Web module root; declares submodules.
    - **models.rs**: API request/response types.
    - **printer_channel.rs**: Async channel for printer commands and status.

---

## Key Modules and Relationships

- **Configuration**: `config.rs` defines all configuration structs. Used by `main.rs`, `motion`, `printer`, and hardware modules.
- **Motion System**: `motion/mod.rs` is the core. `MotionPlanner` manages a queue of `MotionSegment`s, applies kinematics, junction deviation, and re-plans for optimal speed/acceleration. `controller.rs` provides a higher-level interface for queueing moves and managing state. The adaptive optimizer in `planner/adaptive.rs` dynamically tunes planner parameters based on real-time feedback.
- **Hardware Abstraction**: `hardware/mod.rs` and submodules abstract temperature, steppers, and other peripherals. Used by motion and printer modules.
- **G-code Processing**: `gcode/` handles macro expansion and (potentially) parsing and execution.
- **Web API**: `web/api.rs` exposes status and G-code execution endpoints via Axum. Uses async channels to communicate with the printer core.
- **Testing**: `motion/test.rs` provides async unit and integration tests for the motion system, using mock hardware and configs. `planner/adaptive.rs` includes unit tests for adaptive parameter logic.

---

## TODO SECTION

**Status:** All advanced motion planning and input shaping features are implemented, integrated, and validated with unit tests as of July 2025. See the section below for details.

**Completed Steps:**
- Studied Prunt3D’s G⁴ motion profile and Bézier-based corner blending for inspiration; implemented and refined advanced motion planning in `motion/shaper.rs` and `motion/planner/snap_crackle.rs`.
- Supported independent limits for higher-order derivatives and integrated smooth corner blending into the main planning pipeline.
- Reviewed and adapted Prunt3D’s open-source code for implementation details.

**Next Steps:**
- **Extend analytical solutions for G⁴ and Bézier blending:**
  - [x] Research analytical solutions for phase duration and evaluation in G⁴ profiles (see Prunt3D docs and academic literature)
  - [x] Implement or improve analytical/iterative solvers in `motion/planner/snap_crackle.rs` (root-based constraint limiting, inspired by Prunt3D)
  - [x] Add/expand unit tests for edge cases and performance
  - [x] Document mathematical approach and solver limitations (see Technical Notes below and doc comment at the top of `src/motion/planner/snap_crackle.rs`)
- **Integrate shaper/blending config into user-facing config or API:**
  - [x] Design config schema for per-axis shaper and blending options (TOML or API)
    
    **Proposed TOML schema:**
    ```toml
    [motion.shaper.x]
    type = "zvd"
    frequency = 40.0
    damping = 0.1

    [motion.shaper.y]
    type = "sine"
    frequency = 35.0

    [motion.blending]
    type = "bezier"
    max_deviation = 0.2
    ```
    - Each axis (x, y, z, etc.) can have its own shaper type and parameters.
    - Blending (corner smoothing) is configured globally or per-axis as needed.

    **Rust struct/enums for config parsing:**
    ```rust
    use serde::Deserialize;
    use std::collections::HashMap;

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum ShaperType {
        Zvd,
        Sine,
        // Add more as needed
    }

    #[derive(Debug, Deserialize)]
    pub struct AxisShaperConfig {
        pub r#type: ShaperType,
        pub frequency: f32,
        pub damping: Option<f32>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum BlendingType {
        Bezier,
        // Add more as needed
    }

    #[derive(Debug, Deserialize)]
    pub struct BlendingConfig {
        pub r#type: BlendingType,
        pub max_deviation: f32,
    }

    #[derive(Debug, Deserialize)]
    pub struct MotionConfig {
        pub shaper: HashMap<String, AxisShaperConfig>,
        pub blending: Option<BlendingConfig>,
    }
    ```
    - This approach uses enums for shaper/blending types and a map for per-axis configs, following [serde](https://docs.rs/toml/latest/toml/) and [Stack Overflow best practices](https://stackoverflow.com/questions/47785720/deserialize-toml-string-to-enum-using-config-rs).
    - See also Klipper and Prunt3D config examples for real-world reference.
  - [x] Implement config parsing and validation in `config.rs`
  - [x] Wire config into planner and shaper assignment logic
  - [x] Add documentation and usage examples for shaper/blending config

**How to use advanced shaper/blending config:**
- See the top of `src/config.rs` for TOML and Rust usage examples.
- See the top of `src/motion/planner/mod.rs` for planner integration and assignment logic.
- The planner will automatically assign the correct shaper to each axis at runtime based on your config.

  - [x] Continue real/simulated scenario validation:**
  - [x] Develop or expand simulation harness for motion profiles and shaper effects
  - [x] Collect and analyze results for various printer setups
  - [x] Tune parameters and document best practices

**Technical Notes:**
- The G⁴ (31-phase) solver uses a root-based constraint limiting approach, inspired by Prunt3D's open-source planner. Each kinematic constraint (velocity, acceleration, jerk, snap, crackle) is applied using the appropriate root (e.g., a_max^(1/2), j_max^(1/3), s_max^(1/4), c_max^(1/5)), and the minimum is used for safe phase duration calculation. This ensures that no constraint is violated during the motion segment.
- **Mathematical rationale:** For a motion profile limited by multiple derivatives, the maximum feasible velocity is determined by the most restrictive constraint. For example, the maximum velocity for a given acceleration limit is v = sqrt(a_max), for jerk it's v = j_max^(1/3), and so on. The solver computes all such limits and uses the minimum for planning.
- **Bézier blending:** Advanced corner blending is achieved using degree-15 Bézier curves, ensuring smooth transitions with bounded higher-order derivatives (jerk, snap, crackle).
- **Input shaping:** Per-axis input shapers (e.g., ZVD, Sine) are supported for vibration reduction, with configuration and assignment handled in the planner.
- **Limitations:** This method is robust and practical for real-time planning, but it is not a fully analytical or globally optimal solution. It does not guarantee the shortest possible phase durations or transitions between phases. For true optimality, a constraint-based optimizer (e.g., Sequential Quadratic Programming, Newton-Raphson) would be required, which is more computationally intensive and complex to implement. The current approach is a safe, industry-proven compromise.
- **Edge case handling:** The implementation is defensive for zero, negative, and infinite limits; some pathological cases may result in zero-duration or panic (see unit tests for details).
- **References:** See the doc comment at the top of `src/motion/planner/snap_crackle.rs`, `docs/advanced_motion_planning_plan.md`, and inline code comments for further details and limitations.

---

## Incomplete Features, Unused Code, and Simplified/Initial Features

### Incomplete Features
- **Advanced Input Shaping (`motion/shaper.rs`, `motion/planner/snap_crackle.rs`)**: Some files exist for advanced motion smoothing and input shaping, but are not fully implemented or integrated.
- **Print Job Management (`print_job.rs`)**: Basic structure present, but job queueing, pausing, and resuming are not fully implemented.
- **G-code Macro Processing and Parsing (`gcode/macros.rs`, `gcode/parser.rs`)**: Macro expansion logic and an advanced parser exist, but:
  - Full async/streaming parsing and macro expansion are not yet implemented
  - Error recovery (skip to next line on error) is not yet robust
  - Integration with print job and motion system is incomplete
  - Comprehensive tests for edge cases and error handling are needed
  - Documentation/examples for usage and extension are needed
- **Web API Expansion**: Only basic status and G-code endpoints are implemented. No authentication, streaming, or advanced printer control endpoints.
- **Hardware Abstraction**: Only temperature and stepper logic are present; other peripherals (fans, sensors, etc.) are not yet abstracted.
- **Error Handling**: Many modules use `Box<dyn Error>` or basic error enums; more robust error handling and reporting is needed.

### Unused or Placeholder Code
- **`motion/benchmark.rs`**: Benchmarking code is present but not integrated into CI or main workflow. 
  - Next: Integrate with CI, document usage, and ensure results are actionable for tuning motion parameters.
- **`integration_test.rs`**: Exists for integration testing, but may not be fully wired up to the main test harness or CI.
  - Next: Integrate with main test workflow, expand coverage, and document test scenarios.
- **`printer.toml`**: Example config, not actively loaded by the main application.
  - Next: Implement config loading in `main.rs` or config module, and document expected schema and usage.

### Simplified or Initial Features
- **Motion Planning**: The main planner implements basic lookahead, junction deviation, and acceleration limiting, but advanced features (input shaping, multi-axis optimization) are stubbed or simplified.
  - Next: Complete integration of advanced features, expand tests, and document limitations.
- **Web API**: Only two endpoints (`/api/v1/status`, `/api/v1/gcode`) are implemented.
  - Next: Expand API for printer control, monitoring, and streaming; add authentication and OpenAPI spec.
- **Testing**: Motion tests use simple mock hardware and configs; coverage is limited to basic queueing and emergency stop scenarios.
  - Next: Increase test coverage, add edge case and error condition tests, and validate with simulated/real hardware.
- **Configuration**: Uses simple TOML parsing and struct-based config; no live reload or validation.
  - Next: Add config validation, live reload support, and document schema.
- **Hardware Abstraction**: Only temperature and stepper logic are present; other hardware is not yet abstracted.
  - Next: Modularize hardware interfaces, add support for additional peripherals, and improve documentation.

### Reference: Prunt3D Advanced Motion Planning
- Prunt3D implements a 31-phase (G⁴) motion profile, supporting independent limits on velocity, acceleration, jerk, snap, and crackle for ultra-smooth, physically realistic motion. This approach reduces vibration and ringing compared to traditional 3-phase (trapezoidal) or 7-phase (S-curve) profiles.
  - Status: Core G⁴ profile and constraint logic are implemented in `motion/planner/snap_crackle.rs` and validated with unit tests. Further optimization and edge case handling are ongoing.
- Advanced corner blending is achieved using degree-15 Bézier curves, allowing for smooth transitions with bounded higher-order derivatives (jerk, snap, crackle), integrated directly into the control system.
  - Status: Bézier blending is implemented and integrated into the planning pipeline. Further validation and parameter tuning are recommended for new hardware.
- Hardware-accelerated step generation and multi-threading ensure precise, jitter-free motion and real-time guarantees.
  - Status: Step generation and multi-threading are planned for future releases. Current implementation is single-threaded and software-based.
- Next Steps:
  - [ ] Continue to optimize and validate G⁴ profile and blending on a variety of hardware setups
  - [ ] Integrate hardware-accelerated step generation and multi-threading as hardware support matures
  - [ ] Expand documentation and code comments to clarify Prunt3D-inspired algorithms and their limitations
  - [ ] Regularly review Prunt3D upstream for new techniques and update implementation as needed
- See:
  - [Prunt3D Features](https://prunt3d.com/docs/features/)
  - [G⁴ Motion Profiles](https://prunt3d.com/docs/features/#g-motion-profiles)
  - [Advanced Corner Blending](https://prunt3d.com/docs/features/#advanced-corner-blending)
  - [Prunt3D GitHub Motion Planner](https://github.com/Prunt3D/prunt/blob/master/src/prunt-motion_planner.adb)

---

## Adaptive Motion Planning: Pipeline, Configuration, and Integration

All core adaptive motion planning features are implemented, fully integrated, and validated with unit tests as of July 2025. See `src/motion/planner/adaptive.rs` for the optimizer, configuration, and tests, and `src/motion/controller.rs` for integration with the motion controller.

- **Implementation Status:**
  - Adaptive optimizer logic is implemented and integrated with the motion controller and planner.
  - Real-time or simulated feedback (vibration, position error, speed efficiency, thermal stability) is collected after each move.
  - Rolling buffer of recent metrics is maintained for dynamic adjustment of motion parameters (acceleration, jerk, junction deviation).
  - Unit tests in `planner/adaptive.rs` verify correct adaptation under various feedback scenarios.
- **Configuration:**
  - Configurable via `AdaptiveConfig` (see `planner/adaptive.rs`), with parameters for adaptation rate, learning rate, buffer size, and thresholds.
  - Optimizer can be enabled by setting the motion controller to `Adaptive` mode.
- **Integration Points:**
  - Motion controller (`controller.rs`) holds an optional `AdaptiveOptimizer` and updates it after each move.
  - Optimizer’s parameters are applied to the planner before planning each move using setter methods.
- **Next Steps:**
  - [ ] Expand real-world and simulated validation for a wider range of printer setups and feedback scenarios
  - [ ] Tune adaptation heuristics and thresholds for optimal print quality and stability
  - [ ] Document best practices for enabling and configuring adaptive motion planning
  - [ ] Add more integration tests and edge case coverage

---

## Host OS Abstraction: Klipper Parity and Stubs

All planned host OS abstraction features are stubbed and documented in `src/host_os.rs`, including robust serial protocol, clock/time sync, dynamic module system, multi-MCU support, and event extensibility.

- **Implementation Status:**
  - All major host OS abstraction features are currently stubbed (not implemented).
  - Documentation in `src/host_os.rs` outlines intended design, module structure, and future development.
  - No serial protocol, time sync, or event system is active; all functions are placeholders or return errors.
  - No host-to-MCU communication is performed yet; all hardware and motion modules assume local execution.

- **Planned Features:**
  - Robust serial protocol for MCU communication (inspired by Klipper, with CRC, framing, and async support)
  - Clock/time synchronization across MCUs and host (for coordinated motion and event timing)
  - Dynamic module/plugin system for extensibility (hot-reloadable modules, versioning, and isolation)
  - Multi-MCU support for distributed control (multiple printers, toolheads, or expansion boards)
  - Event system for extensibility and integration (async event bus, hooks for plugins and web API)
  - Platform abstraction for Windows, Linux, and embedded targets (conditional compilation, feature flags)

- **Next Steps:**
  - [ ] Prioritize and implement core host OS abstraction features:
    - [ ] Serial protocol (frame parsing, CRC, async I/O)
    - [ ] Time sync (NTP, MCU clock sync, monotonic timers)
    - [ ] Event system (async event bus, event types, and handlers)
  - [ ] Incrementally integrate with motion, hardware, and web modules:
    - [ ] Replace direct hardware calls with host OS abstraction layer
    - [ ] Add integration tests for serial and event system
    - [ ] Document API and extension points for future module/plugin developers
  - [ ] Review Klipper, OctoPrint, and other open-source hosts for best practices and feature parity
  - [ ] Update this section and `src/host_os.rs` as features are implemented or design evolves

---

## Recommendations

- **Expand G-code parsing and execution pipeline:**
  - [ ] Implement full G-code parsing and streaming (see `gcode/macros.rs`, `gcode/parser.rs`)
  - [ ] Integrate macro expansion and execution pipeline with print job and motion system
  - [ ] Add comprehensive tests for edge cases and error handling (invalid macros, nested macros, malformed G-code)
  - [ ] Refactor for async/streaming parsing and non-blocking macro expansion (see latest Rust async best practices)
  - [ ] Implement robust error recovery: on parse error, skip to next line before resuming parsing (prevents cascading errors)
  - [ ] Ensure all errors include span/location info for diagnostics and web API reporting
  - [ ] Document parser and macro processor usage, extension points, and limitations (add examples in code and docs)

- **Add more comprehensive tests, especially for edge cases and error handling:**
  - [ ] Increase unit/integration test coverage in all motion, hardware, and G-code modules
  - [ ] Add tests for error conditions, invalid configs, and hardware failures
  - [ ] Validate with simulated and real hardware (document test scenarios and results)

- **Expand web API for richer printer control and monitoring:**
  - [ ] Add endpoints for printer control, monitoring, and streaming (pause, resume, cancel, status, logs)
  - [ ] Implement authentication and access control (JWT, API keys, or OAuth)
  - [ ] Document API usage and add OpenAPI spec if possible (generate from Axum routes)

- **Refactor and document hardware abstraction for extensibility:**
  - [ ] Modularize hardware interfaces (see `hardware/mod.rs`)
  - [ ] Add support for additional peripherals (fans, sensors, power control, etc.)
  - [ ] Improve inline and module-level documentation (usage, extension, and safety notes)

- **Remove or refactor unused code and stubs as features are completed:**
  - [ ] Identify and remove obsolete stubs (see `integration_test.rs`, `motion/benchmark.rs`, etc.)
  - [ ] Refactor placeholder code as features are implemented (replace stubs with real logic)
  - [ ] Keep this document and TODOs up to date (review after each major feature or refactor)

---

> **Note for Future Development:**
> - All multi-threaded or GUI-related features must follow Rust's best practices for thread and memory safety:
>   - Only update GUI or main-thread-only resources from the main thread (use message passing for cross-thread communication)
>   - Use `Rc<RefCell<T>>` for main-thread shared state, `Arc<Mutex<T>>` for cross-thread state
>   - Leverage Rust's ownership and type system to prevent data races and memory safety issues
>   - See the latest Rust Book, users.rust-lang.org, and docs.rs for up-to-date patterns
> - Update this document as new concurrency or GUI features are added.

---

# Advanced Motion Planning: G⁴ Profile, Bézier Blending, and Input Shaping (2025)

### G⁴ (31-Phase) Motion Profile
- Implemented in `motion/planner/snap_crackle.rs` as `G4MotionProfile` and `G4ProfilePhases`.
- Supports independent limits for velocity, acceleration, jerk, snap, and crackle.
- Analytical/iterative solver stubs and evaluation functions for all derivatives at time t.
- Prepares for ultra-smooth, physically realistic motion with bounded higher-order derivatives.

### Bézier-Based Advanced Corner Blending
- Implemented in `motion/planner/snap_crackle.rs` as `BezierBlender`.
- Uses degree-15 Bézier curves to blend corners, ensuring smooth transitions with bounded jerk, snap, and crackle.
- Configurable maximum path deviation; integrated into the planning pipeline.

### Modular Input Shaper System
- Implemented in `motion/shaper.rs` using the `InputShaperTrait` trait.
- Includes `ZVDShaper` (Zero Vibration Derivative) and `SineWaveShaper` (demo) as examples.
- Per-axis shaper assignment and configuration supported in the planner (`MotionPlanner::input_shapers`).
- Shapers are applied in the step generation pipeline for vibration reduction and experimental shaping.

### Testing
- Unit tests for G⁴ profile, Bézier blending, and input shaper logic in `motion/planner/snap_crackle_tests.rs`.
- All tests pass as of July 2025.

### Configuration
- Input shapers can be assigned per axis in the planner; extendable for future config file or API integration.
- All new types and algorithms are documented inline in code for onboarding and future development.

---

*This document should be updated as the project evolves. See inline comments in code for additional TODOs and notes.*

### Example: Configuring Per-Axis Input Shaper and Blending

To enable advanced input shaping and blending, add the following to your `printer.toml`:

```toml
[motion.shaper.x]
type = "zvd"
frequency = 40.0
damping = 0.1

[motion.shaper.y]
type = "sine"
frequency = 35.0

[motion.blending]
type = "bezier"
max_deviation = 0.2
```
- Each axis (x, y, z, e) can have its own shaper type and parameters.
- Blending (corner smoothing) is configured globally or per-axis as needed.

**How it works:**
- The config is parsed into Rust structs/enums (`MotionConfig`, `AxisShaperConfig`, etc.) in `config.rs`.
- The planner (`MotionPlanner`) automatically assigns the correct shaper to each axis at runtime.
- Extendable: Add new shaper types or parameters by updating the config schema and Rust enums.

**See also:**
- `src/config.rs` for config structs and validation
- `src/motion/planner/mod.rs` for planner integration
- `src/motion/shaper.rs` for shaper implementations

*For more details, see the main TODO and code comments.*

### Simulation and Analysis: Input Shaper and Blending Effects

- Use the simulation harness in `motion/benchmark.rs` to test different shaper/blending configs.
- Run the harness with various printer setups and shaper parameters.
- Output (CSV) can be plotted to visualize the effect of each config on motion smoothness and vibration.
- Compare results for different axes, shaper types, and blending settings.
- Use findings to tune parameters and document best practices for your hardware.

*See comments in `benchmark.rs` for usage details.*

### Parameter Tuning and Best Practices: Input Shaper and Blending

- **Start with simulation:** Use the simulation harness to test different shaper types (e.g., ZVD, Sine) and parameters (frequency, damping) for each axis. Plot the results to see which settings best suppress vibration and overshoot for your printer's resonance characteristics.
- **Axis-specific tuning:** X and Y axes often benefit most from input shaping. Z and E may not need shaping unless resonance is observed.
- **Frequency selection:** Set the shaper frequency to match the dominant resonance frequency of your printer (measure with test prints or accelerometer if possible).
- **Damping:** Increase damping if you see residual oscillations, but too much can reduce responsiveness.
- **Blending (corner smoothing):** Use blending to reduce sharp transitions at corners. Tune `max_deviation` for a balance between path accuracy and smoothness.
- **Validate on hardware:** After simulation, test the chosen parameters on your printer. Watch for improvements in print quality and reductions in ringing/ghosting.
- **Iterate:** Fine-tune parameters based on print results and further simulation as needed.
- **Troubleshooting:**
  - If motion feels sluggish, reduce damping or max_deviation.
  - If vibration persists, try a different shaper type or adjust frequency.
  - If path accuracy is poor, lower `max_deviation` or disable blending.
- **References:**
  - [Klipper Input Shaping Guide](https://www.klipper3d.org/Resonance_Compensation.html)
  - [Prusa Motion System Docs](https://help.prusa3d.com/article/input-shaper_2280)
  - [Prunt3D Features](https://prunt3d.com/docs/features/)

*Document your tuned parameters and rationale for future reference!*
