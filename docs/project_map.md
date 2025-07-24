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
    - **trajectory.rs**: Trajectory generation and interpolation.
  - **web/**
    - **api.rs**: Axum-based HTTP API routes and handlers.
    - **mod.rs**: Web module root; declares submodules.
    - **models.rs**: API request/response types.
    - **printer_channel.rs**: Async channel for printer commands and status.
- **tests/**
  - **motion_controller.rs**: Integration tests for the motion controller (moved from `src/motion/test.rs`).
  - **snap_crackle.rs**: Unit tests for G⁴ profile, Bézier blending, and input shaper logic (moved from `src/motion/planner/snap_crackle_tests.rs`).
  - **adaptive_optimizer.rs**: Tests for adaptive optimizer (moved from `src/motion/planner/adaptive.rs`).
  - **integration.rs**: Integration tests for the complete system (moved from `src/integration_test.rs`).
  - **gcode_parser.rs**: Tests for the advanced G-code parser prototype (moved from `src/gcode/parser_tests.rs`).
  - **gcode_mod.rs**: Tests for the G-code module (moved from `src/gcode/mod.rs`).
  - **gcode_macros.rs**: Tests for G-code macros (moved from `src/gcode/macros.rs`).
  - **file_manager.rs**: Tests for file manager (moved from `src/file_manager.rs`).
  - **config.rs**: Tests for config (moved from `src/config.rs`).
  - **mod.rs**: Test module root (can be used for shared test utilities).

---

## Key Modules and Relationships

- **Configuration**: `config.rs` defines all configuration structs. Used by `main.rs`, `motion`, `printer`, and hardware modules.
- **Motion System**: `motion/mod.rs` is the core. `MotionPlanner` manages a queue of `MotionSegment`s, applies kinematics, junction deviation, and re-plans for optimal speed/acceleration. `controller.rs` provides a higher-level interface for queueing moves and managing state. The adaptive optimizer in `planner/adaptive.rs` dynamically tunes planner parameters based on real-time feedback.
- **Hardware Abstraction**: `hardware/mod.rs` and submodules abstract temperature, steppers, and other peripherals. Used by motion and printer modules.
- **G-code Processing**: `gcode/` handles macro expansion and (potentially) parsing and execution.
- **Web API**: `web/api.rs` exposes status and G-code execution endpoints via Axum. Uses async channels to communicate with the printer core.
- **Testing**: All unit and integration tests are now located in the `tests/` directory. Each major subsystem has its own test file, and all legacy test modules have been removed from `src/`.

---

## TODO SECTION

**Status (July 2025):**
- Advanced motion planning (G⁴, Bézier blending, input shaping) is implemented and validated with unit tests, but some advanced features (vibration cancellation, higher-order controller, optimizer) remain stubbed in `snap_crackle.rs`.
- Print job management is mostly stubbed; queueing, pausing, resuming, and cancellation are not implemented.
- G-code macro/streaming parsing and error recovery are incomplete; integration with print job and motion system is partial.
- Web API is minimal; advanced endpoints (pause, resume, cancel, logs, streaming) and authentication are missing.
- Hardware abstraction is limited to temperature/stepper; other peripherals are not abstracted.
- Host OS abstraction is stubbed; no real serial protocol, time sync, or event system.
- Error handling is basic in many modules; more robust enums and diagnostics are needed.

**Completed Steps:**
- Studied Prunt3D’s G⁴ motion profile and Bézier-based corner blending for inspiration; implemented and refined advanced motion planning in `motion/shaper.rs` and `motion/planner/snap_crackle.rs`.
- Supported independent limits for higher-order derivatives and integrated smooth corner blending into the main planning pipeline.
- Reviewed and adapted Prunt3D’s open-source code for implementation details.

**Next Steps:**

```markdown
## Updated TODOs and Action Items (July 2025)

## Updated TODOs and Action Items (July 2025)

**Focus: Build the Base System**

- [ ] Motion system:
  - [ ] Implement basic move queueing and execution in `motion/controller.rs` and `motion/planner/mod.rs`.
  - [ ] Ensure the printer can move reliably with simple G-code (G0/G1).
  - [ ] Integrate motion system with hardware abstraction (stepper, temperature).
  - [ ] Add basic error handling and diagnostics for motion failures.
- [ ] Print job management:
  - [ ] Implement job queueing, pausing, resuming, and cancellation in `print_job.rs`.
  - [ ] Integrate print job state with G-code streaming and error recovery.
- [ ] G-code macro/streaming parsing:
  - [ ] Complete async/streaming parsing and macro expansion in `gcode/macros.rs` and `gcode/parser.rs`.
  - [ ] Implement robust error recovery (skip to next line on error, span info).
  - [ ] Integrate macro expansion with print job and motion system.
- [ ] Web API:
  - [ ] Add endpoints for pause, resume, cancel, and basic status.
  - [ ] Implement minimal authentication (API key or JWT).
- [ ] Hardware abstraction:
  - [ ] Modularize hardware interfaces for stepper and temperature.
  - [ ] Integrate and test with main workflow.
- [ ] Host OS abstraction:
  - [ ] Implement serial protocol (frame parsing, CRC, async I/O).
  - [ ] Integrate with motion, hardware, and web modules.
- [ ] Error handling:
  - [ ] Refactor to use robust error enums (`thiserror`), propagate with `?`, add context.
  - [ ] Ensure all errors include span/location info for diagnostics and web API.
- [ ] Testing:
  - [ ] Increase unit/integration test coverage for all modules, especially error and edge cases.
  - [ ] Validate with simulated and real hardware; automate result collection and reporting.

---

**Note:**
Advanced features such as vibration cancellation, higher-order controller, optimizer, advanced input shaping, multi-axis optimization, and hardware-accelerated step generation have been moved to `future_map.md` for tracking after the base system is functional.
```

**Technical Notes:**
- The G⁴ (31-phase) solver uses a root-based constraint limiting approach, inspired by Prunt3D's open-source planner. Each kinematic constraint (velocity, acceleration, jerk, snap, crackle) is applied using the appropriate root (e.g., a_max^(1/2), j_max^(1/3), s_max^(1/4), c_max^(1/5)), and the minimum is used for safe phase duration calculation. This ensures that no constraint is violated during the motion segment. REFERENCE KRUSTY-RS/SIM/PRUNT/SRC/
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

**Status (July 2025, Deep Dive):**
- Advanced motion planning (G⁴, Bézier blending, input shaping, adaptive optimizer) is implemented and validated with unit tests. Stubs remain for vibration cancellation, higher-order controller, and optimizer in `snap_crackle.rs`.
- Print job management (`print_job.rs`) has a `PrintJobManager` with stubs for queue, pause, resume, and cancel, but logic is minimal and not fully integrated.
- G-code macro/streaming parsing and error recovery are advanced (async, macro expansion, error types, trait-based extensibility), but integration with print job and motion system is partial and error recovery is not robust.
- Web API exposes basic endpoints via Axum, but advanced endpoints (pause, resume, cancel, logs, streaming) and authentication are missing.
- Hardware abstraction now includes fans and generic sensors, but only temperature and stepper logic are fully implemented and tested; other peripherals are present but not fully integrated.
- Host OS abstraction (`host_os.rs`) is architected for extensibility (serial protocol, time sync, event bus, multi-MCU), but all features are stubbed and not active.
- Error handling is improved in some modules (custom enums, `thiserror`), but not all errors are robustly typed or contextualized; some modules still use `Box<dyn Error>`.
- Simulation harness and benchmark tests support parameter sweeps and scenario analysis for shaper/blending, but are not integrated into CI.
- Config system supports per-axis shaper/blending, validation, and error enums, but live reload and schema documentation are incomplete.
        - Configurable printer and planner parameters via TOML/JSON (see Prunt3D `prunt_sim.toml` for reference)
        - Injection of disturbances (oscillating substrate, random noise, feedback delay)
      - Automate result collection and reporting; document all scenarios, parameters, and results for reproducibility
      - Reference: [Prunt3D Simulator](https://github.com/Prunt3D/prunt_simulator), [ScienceDirect: Closed-loop controlled conformal 3D printing](https://www.sciencedirect.com/science/article/pii/S1526612523000282)
  - [ ] Tune adaptation heuristics and thresholds for optimal print quality and stability
    - **Best Practices and Plan:**
      - Begin with conservative adaptation and learning rates in `AdaptiveConfig` (see `planner/adaptive.rs`).
      - Monitor quantitative metrics (path deviation, vibration amplitude, print quality, error rates) during tuning.
      - Gradually increase adaptation aggressiveness while checking for instability or overshoot.
      - Document tuned parameters and rationale for each hardware setup in the codebase and project docs.
      - Reference: [Klipper Input Shaping Guide](https://www.klipper3d.org/Resonance_Compensation.html), [Prunt3D Features](https://prunt3d.com/docs/features/), [Machine learning-driven 3D printing: A review](https://www.sciencedirect.com/science/article/pii/S2352940724002518)
    - **Best Practices and Usage:**
      - To enable adaptive motion planning, set the motion controller mode to `Adaptive` in your config (see `planner/adaptive.rs`).
      - Expose and document all key parameters in your config file (TOML):
        - `adaptation_rate`: How quickly the optimizer responds to feedback (start low, increase as needed).
        - `learning_rate`: Controls the magnitude of parameter updates (start with a small value).
        - `thresholds`: Limits for triggering adaptation (e.g., vibration, error, or deviation thresholds).
      - Example TOML config:
        [motion.adaptive]
        enabled = true
        adaptation_rate = 0.05
        buffer_size = 10
        vibration_threshold = 0.2
        error_threshold = 0.05
        ```
        ```rust
        let adaptive_config = AdaptiveConfig {
            enabled: true,
            learning_rate: 0.01,
            buffer_size: 10,
            vibration_threshold: 0.2,
        };
        let mut controller = MotionController::new();
        controller.set_adaptive_config(adaptive_config);
        ```
      - Always validate config changes in simulation before applying to hardware.
      - Document the rationale for chosen parameters and update as you tune for your hardware.
      - Reference: [Prunt3D Features](https://prunt3d.com/docs/features/), [Klipper Input Shaping Guide](https://www.klipper3d.org/Resonance_Compensation.html), [ScienceDirect: Closed-loop controlled conformal 3D printing](https://www.sciencedirect.com/science/article/pii/S1526612523000282)
  - [ ] Add more integration tests and edge case coverage
    - **Best Practices and Plan:**
      - Expand integration tests to cover the full motion pipeline: config parsing, planner/controller interaction, feedback loops, and hardware abstraction.
      - Add edge case tests for:
        - Zero, negative, and extreme values for all motion parameters (acceleration, jerk, snap, crackle, etc.).
        - Sudden or missing feedback (sensor spikes, lost/delayed feedback, simulated hardware faults).
        - Pathological or malformed G-code (out-of-bounds moves, rapid reversals, invalid commands).
        - Hardware faults (stepper/temperature sensor failure, communication errors).
        - Realistic disturbance scenarios (oscillating/moving substrate, vibration, etc.).
      - Use both simulation and hardware-in-the-loop tests where possible.
      - Automate test result collection and reporting (CSV, logs, plots).
      - **Test Scenarios Implemented (July 2025):**
        - Integration tests in `tests/integration.rs` now cover:
          - Zero, negative, and extreme values in `max_acceleration` and `max_jerk` arrays (planner does not panic, returns error or ok as appropriate).
          - Simulated feedback faults at the controller level (robust to missing or extreme feedback, controller remains functional).
        - All tests pass as of July 2025. See `tests/integration.rs` for details and future expansion.
      - **Next:**
        - Expand tests for malformed/pathological G-code and hardware faults.
        - Add automation for test result collection and reporting (CSV/log/plot output).
        - Continue to document all new scenarios and results here for reproducibility.
        - Reference: [Prunt3D Features](https://prunt3d.com/docs/features/), [Klipper Input Shaping Guide](https://www.klipper3d.org/Resonance_Compensation.html), [ScienceDirect: Closed-loop controlled conformal 3D printing](https://www.sciencedirect.com/science/article/pii/S1526612523000282)

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


## Best Practices and Recommendations (2025)

- **Async Rust:**
  - Use structured concurrency; tie task lifetimes to parent scopes, avoid detached tasks.
  - Minimize async locks; prefer channels or lock-free patterns for coordination.
  - Use `tokio` or `async-std` for async runtimes; Embassy for embedded.
  - Avoid `.await` in critical sections or while holding locks.
  - Use `tracing` for async-aware logging and diagnostics.

- **Error Handling:**
  - Use `Result<T, E>` for recoverable errors, `Option<T>` for optional values.
  - Use `thiserror` for custom error enums, `anyhow` for application-level errors.
  - Avoid panics except for unrecoverable bugs; never panic on user input or I/O.
  - Propagate errors with `?`, add context with `.context()`.
  - Ensure all errors include span/location info for diagnostics and web API.

- **3D Printer Host & Motion Planning:**
  - Modular, hardware-agnostic design; support for hybrid/multi-material and large-scale printing.
  - Real-time feedback and adaptive planning (AI/ML for tuning).
  - Advanced motion planners: lookahead, jerk/snap/crackle limiting, input shaping.
  - Simulation harnesses for parameter tuning and validation.
  - Secure, extensible web APIs (authentication, streaming, OpenAPI).

- **Testing:**
  - Write isolated, deterministic tests for error and edge cases.
  - Automate result collection and reporting; validate with simulated and real hardware.

- **Documentation:**
  - Keep this document and inline code comments up to date after each major feature or refactor.
  - Document rationale for parameter choices and best practices for tuning.

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
