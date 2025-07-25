# System Patterns

## Architecture Overview
Modular, async Rust-based architecture. Major modules: configuration, motion planning, hardware abstraction, G-code processing, web API. Each module is logically separated for extensibility and testability. Async runtime (tokio/async-std) powers all I/O and coordination.

## Key Technical Decisions
- Use of Rust for safety, performance, and async support
- Modular separation of motion, hardware, G-code, and web API
- Advanced motion planning (G⁴, Bézier blending, input shaping, adaptive optimization)
- Dedicated test structure for all major subsystems
- Deferred advanced features to future_map.md for clarity and maintainability

## Design Patterns in Use
- Trait-based extensibility for motion planners, input shapers, and hardware interfaces
- Channel/message passing for async coordination (printer commands, status)
- Error enums with `thiserror` for robust error handling
- Config-driven design (TOML/JSON)

## Component Relationships
- `main.rs` initializes config, sets up async runtime, and launches web/motion subsystems
- `motion` module manages planning, queueing, and execution; integrates with hardware abstraction
- `hardware` module abstracts temperature, steppers, and peripherals; used by motion and printer modules
- `gcode` module handles macro expansion, parsing, and execution; integrates with print job and motion system
- `web` module exposes API endpoints and async channels for printer control and monitoring
- All modules are tested in isolation and via integration tests in `tests/`
