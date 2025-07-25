# Project Brief

This is the foundation document for the Memory Bank. It defines the core requirements and goals for the Krusty-rs project.

## Project Name

Krusty-rs: Modular Async 3D Printer Host & Motion Control System

## Core Requirements
- Modular, async Rust-based 3D printer host and motion control system
- Extensible architecture for configuration, motion planning, hardware abstraction, G-code processing, and web API
- High performance and real-time guarantees for motion and hardware control
- Robust error handling, diagnostics, and test coverage
- Secure, extensible web API for printer control and monitoring

## Goals
- Build a reliable, extensible, and high-performance 3D printer host in Rust
- Support advanced motion planning (G⁴, Bézier blending, input shaping, adaptive optimization)
- Provide a modern web API for printer control, monitoring, and integration
- Enable simulation, parameter tuning, and validation for motion and hardware
- Achieve robust error handling and diagnostics throughout the stack

## Scope
- Core system: motion planning, hardware abstraction, G-code parsing/execution, web API
- Advanced features (vibration cancellation, higher-order controller, optimizer, multi-axis optimization, hardware-accelerated step generation) are tracked in `future_map.md` and not in the base scope
- Focus on extensibility, modularity, and testability

## Source of Truth
This document is the source of truth for project scope and direction.
