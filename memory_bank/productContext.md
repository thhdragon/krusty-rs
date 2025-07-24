# Product Context

## Purpose
Krusty-rs exists to provide a modern, modular, and extensible 3D printer host and motion control system, leveraging Rust’s safety and async capabilities. It aims to enable advanced motion planning, robust hardware abstraction, and seamless integration with web APIs for next-generation 3D printing workflows.

## Problems Solved
- Lack of modern, extensible, and safe 3D printer hosts in Rust
- Difficulty integrating advanced motion planning and input shaping in existing hosts
- Limited support for real-time diagnostics, error handling, and simulation in legacy systems
- Fragmented hardware abstraction and poor testability in traditional firmware/host stacks

## User Experience Goals
- Reliable, high-performance printing with advanced motion quality
- Easy configuration, extensibility, and integration with other tools
- Modern web API for control, monitoring, and automation
- Clear diagnostics, error reporting, and simulation support for tuning and validation

## How It Should Work
Users configure their printer and motion system via TOML/JSON files, interact with the printer through a secure web API, and benefit from advanced motion planning and diagnostics. The system supports simulation and real hardware, with robust error handling and clear feedback throughout the workflow.
