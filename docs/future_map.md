# Krusty-rs Future Features Roadmap

This document tracks advanced and long-term features for Krusty-rs that are not required for the initial base system. These items will be prioritized after the core system is functional (i.e., the printer can move, queue jobs, and execute basic G-code reliably).

## Deferred Advanced Features

- **Advanced Motion Planning**
  - Vibration cancellation (motion/planner/snap_crackle.rs)
  - Higher-order controller (motion/planner/snap_crackle.rs)
  - Optimizer (motion/planner/snap_crackle.rs)
  - Multi-axis optimization
  - Real-time feedback and adaptive planning (AI/ML for tuning)
  - Hardware-accelerated step generation and multi-threading
  - Constraint-based global optimizer (e.g., SQP, Newton-Raphson)
  - Pathological/edge-case handling for all advanced profiles

- **Input Shaping**
  - Per-axis input shaper assignment and configuration
  - Advanced shaper types (ZVD, Sine, EI, etc.)
  - Simulation and parameter sweep harnesses
  - Hardware validation and tuning tools

- **Web API**
  - Streaming endpoints for real-time monitoring
  - Advanced logging and analytics endpoints
  - OpenAPI/Swagger spec and auto-generated docs

- **Hardware Abstraction**
  - Support for additional peripherals (fans, sensors, power, etc.)
  - Modular plugin system for hardware extensions
  - Multi-MCU and distributed control

- **Host OS Abstraction**
  - Dynamic module/plugin system
  - Event bus and extensibility hooks
  - Platform abstraction for Windows, Linux, embedded

- **Testing & Simulation**
  - Automated result collection and reporting
  - Hardware-in-the-loop and real-time simulation
  - Advanced test scenarios (disturbances, faults, etc.)

- **Other**
  - Machine learning-driven 3D printing
  - Closed-loop control with real sensor feedback
  - Documentation and best practices for all advanced features

---

*This document will be updated as the project evolves and as advanced features become a priority.*
