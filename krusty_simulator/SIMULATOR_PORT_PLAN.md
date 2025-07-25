# Krusty Simulator: Prunt Feature Parity & Physics Port Plan (Expanded & Deep-Dived)

## Overview
This plan details the steps to fully clone prunt/prunt_simulators advanced simulation physics, event systems, hardware abstraction, and output features in Rust, achieving full parity, extensibility, and maintainability.

**Important:** The end goal is to evolve from a simulator to robust, real-world production firmware. All functions, modules, and hooks must be designed and validated for actual hardware deployment, not just simulation. Prunt/prunt_simulator and Klipper are advanced references for feature and safety parity, but this project is its own implementationdo not assume direct equivalence or copy-paste. Always build with production readiness, extensibility, and safety in mind.

---

## Verification & Wiring Discipline

**Never assume a feature is finished or wired up.**
- Always double-check that every function, module, and integration point is actually connected, tested, and working as intended.
- Validate that all hooks, callbacks, and event handlers are registered and exercised in real scenarios.
- Use cargo's unused warnings and integration tests to identify dead code, unconnected features, and missing wiring.
- Document all assumptions and verify them with real tests or code reviews.
- Treat every subsystem as potentially incomplete until proven otherwise.
- **Do not use `_` prefixes just to silence warnings.** All unused code, variables, and functions should be reviewed and either removed or properly integrated. Intentionally hiding unused code with `_` is discouraged for production firmwarereview and address the root cause instead.
- Regularly search for `_`-prefixed identifiers and ensure nothing critical is being hidden or left incomplete.

---

## Iterative Deep Dive: Incomplete & Unwired Features

Below is a living table of features, functions, and modules that are incomplete, not fully wired, or not validated in integration. This list should be updated continuously as the codebase evolves. Use cargo's unused warnings, integration test coverage, and manual code review to populate and maintain this table.

| Area/Module         | Feature/Function         | Status         | Notes/Next Steps |
|---------------------|-------------------------|----------------|------------------|
| motion/             | S-curve profiles        | Incomplete     | Needs implementation and integration |
| motion/             | Jerk (crackle) support  | Incomplete     | Not wired to stepper timing         |
| hardware/           | Heater simulation       | Partial        | Lacks error injection, not validated|
| gcode/              | Macro expansion         | Incomplete     | Not exercised in tests              |
| simulator/          | Event queue             | Partial        | Not all modules schedule events     |
| output/             | JSONL output            | Incomplete     | Not hooked to event system          |
| integration/        | Plugin system           | Not started    | Design and initial hooks missing    |
| ...                 | ...                     | ...            | ...                                |

- Use `cargo build` and review all unused warnings to find dead/unwired code.
- Run integration and unit tests to identify untested or unexercised features.
- Manually review module interfaces for missing connections or incomplete wiring.
- Update this table regularly as features are completed or new gaps are found.

---

## Subsystem Breakdown & Feature Mapping

### 1. Motion Planning & Physics
- Trapezoidal/S-curve velocity profiles
- Acceleration, deceleration, jerk (crackle)
- Junction deviation, blending, lookahead
- Stepper timing, interpolation, missed steps
- Realistic inertia, max speed, torque
- Time-based simulation (real/simulated clock)
- Interrupt-driven stepper simulation
- Edge case simulation: stall, overcurrent, missed steps, step loss recovery
- Board-specific timing constraints
- Multi-axis coordination and kinematics
- Dynamic feedrate adjustment
- Physics-based error injection (simulate faults)

### 2. Hardware Modeling
- Heater/thermistor thermal simulation (power, feedback, error)
- Fan PWM, tachometer, state transitions
- Switches, input events, error states
- Board abstraction: pin mapping, hardware config
- Power supply, voltage drop, current limits
- Thermal runaway, recovery, and safety logic
- Sensor simulation: endstops, probes, filament sensors
- Simulated hardware faults and recovery

### 3. GCode Parsing & Execution
- Full GCode command set (arc, dwell, homing, macros, custom commands)
- Robust parameter/context parsing (relative/absolute, offsets, feedrates)
- Conditional, loop, and macro execution
- Streaming/buffering of GCode input
- Macro expansion and context management
- Error handling, reporting, and recovery (resume after failure)
- GCode event hooks and extensibility

### 4. Event-Driven Simulation & Output
- State transitions, event logs, error traces
- Real-time event queueing and prioritization
- Configurable output granularity (per step, ms, event)
- Output to CLI, CSV, JSONL, and event logs
- Integration with plotting/visualization tools
- Customizable output hooks for external analysis
- Simulation clock and event scheduling
- Event injection for testing edge cases

### 5. Integration & Extensibility
- Modular architecture for easy feature addition
- Board and hardware abstraction layers
- Plugin system for custom hardware/simulation modules
- Configurable simulation parameters (clock speed, step size, error injection)
- API for external control and monitoring
- Documentation and code comments for maintainability

---

## Rust Module Mapping
- `motion/` : Physics, profiles, blending, timing, interrupts, kinematics
- `hardware/` : Heater, fan, switch, board abstraction, pin mapping, sensors
- `gcode/` : Parser, context, macro engine, streaming, event hooks
- `simulator/` : Event loop, state machine, output, event queue, clock
- `output/` : CLI, CSV, JSONL, event logs, plotting hooks
- `integration/` : Plugin system, board/hardware config, API

---

## Implementation Steps

### Phase 1: Foundation
- [x] Catalog all Ada source features
- [x] Design Rust module structure
- [ ] Set up Rust types for positions, velocities, hardware state
- [ ] Define board abstraction and pin mapping types
- [ ] Establish event queue and simulation clock
- [ ] Document all module interfaces and integration points

### Phase 2: Motion Planning & Physics
- [ ] Implement trapezoidal/S-curve profiles
- [ ] Add acceleration, jerk, blending, junction deviation
- [ ] Simulate stepper timing, missed steps, inertia
- [ ] Integrate time-based simulation and event scheduling
- [ ] Implement interrupt-driven stepper simulation
- [ ] Simulate edge cases (stall, overcurrent, missed steps, step loss recovery)
- [ ] Multi-axis coordination and kinematics
- [ ] Dynamic feedrate adjustment
- [ ] Physics-based error injection

### Phase 3: Hardware Modeling
- [ ] Model heater/thermistor physics, thermal runaway, recovery
- [ ] Model fan PWM/tachometer
- [ ] Model switches, input events, board features
- [ ] Simulate power supply, voltage drop, current limits
- [ ] Sensor simulation (endstops, probes, filament sensors)
- [ ] Simulated hardware faults and recovery

### Phase 4: GCode & Execution
- [ ] Expand parser for full GCode set (arc, dwell, homing, macros, custom)
- [ ] Implement context, macros, conditional/loop logic
- [ ] Add error handling/reporting and recovery
- [ ] Implement GCode streaming and buffering
- [ ] Macro expansion and context management
- [ ] GCode event hooks and extensibility

### Phase 5: Event & Output
- [ ] Implement event-driven simulation and event queue
- [ ] Output state transitions, errors, events
- [ ] Support CLI, CSV, JSONL, event logs
- [ ] Integrate with plotting/visualization tools
- [ ] Add customizable output hooks
- [ ] Event injection for edge case testing

### Phase 6: Integration & Extensibility
- [ ] Implement plugin system for custom modules
- [ ] Add board/hardware config support
- [ ] Make simulation parameters configurable
- [ ] API for external control and monitoring

### Phase 7: Validation & Testing
- [ ] Validate output against prunt results (CSV, JSONL, event logs)
- [ ] Add unit/integration tests for physics and features
- [ ] Document edge cases and limitations
- [ ] Fuzz and stress test event system and hardware simulation

---

## Success Criteria
- All prunt/prunt_simulator features and physics are present
- Output matches prunt for all test cases (CSV, JSONL, event logs)
- Code is modular, maintainable, and documented
- All edge cases and errors are handled and testable
- Extensible for future hardware and simulation features
- API and plugin system documented and usable

---

## Notes
- Reference Ada source for algorithm details and edge case handling
- Use Rust best practices for safety and performance
- Iterate and validate after each phase
- Document integration points, plugin API, and event system

---

## Module Interfaces & Integration Points

### Board Abstraction & Pin Mapping
- `hardware/board_config.rs` provides `BoardConfig` and `BoardTiming` for board-specific features, pin assignments, and timing constraints.
- Used by `HardwareManager` to initialize hardware modules (steppers, heaters, fans, sensors) with correct pin mapping and timing.
- Enables simulation of board-specific quirks and hardware faults.
- Power supply, voltage drop, current limits
- Thermal runaway, recovery, and safety logic
- Sensor simulation: endstops, probes, filament sensors
- Simulated hardware faults and recovery

### Event Queue & Simulation Clock
- `simulator/event_queue.rs` provides `SimEventQueue` and `SimClock` for event-driven simulation and time management.
- All modules (motion, hardware, gcode, output) schedule events via the event queue, which are processed in timestamp order.
- `SimClock` advances simulation time and synchronizes event execution, enabling configurable output granularity and event injection for edge case testing.

### Motion, Hardware, and Simulator Integration
- Motion modules (trajectory, stepper, kinematics) interact with hardware via scheduled events (e.g., step, heater update).
- Hardware modules (temperature, fan, sensor) update state and report events to the queue for output and error handling.
- Simulator module coordinates event processing, state transitions, and output logging, using the event queue and clock as central orchestrators.
- Board config and pin mapping are referenced by all hardware and motion modules for correct simulation behavior.

### Output & Extensibility
- Output module hooks into event queue for CLI, CSV, JSONL, and visualization integration.
- Plugin system and API modules extend event types and board features for future hardware and simulation needs.

---

*Generated by autonomous agent for full prunt simulator parity in Rust. This plan reflects the complete scope of prunt/prunt_simulator, including advanced physics, event systems, hardware abstraction, extensibility, and edge case simulation.*

