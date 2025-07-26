# krusty-rs: Shared Code Migration Plan

## Purpose
This document outlines the plan to migrate reusable logic from `krusty_host` to `krusty_shared` so that both `krusty_host` and `krusty_simulator` can leverage common code, reduce duplication, and improve maintainability.

---

## Rationale
- **Reduce code duplication** between host and simulator
- **Ensure consistency** in motion planning, input shaping, and job management
- **Centralize shared data models** for easier serialization and communication
- **Align with Rust best practices** for modularity and code reuse

---

## Migration Checklist

- [x] **Motion Planning Logic**
    - [x] Move `motion/trajectory.rs` to `krusty_shared` (complete)
    - [x] Move `motion/s_curve.rs` to `krusty_shared` (complete, file deleted from host)
    - [x] Move `motion/planner/snap_crackle.rs` to `krusty_shared` (complete, file deleted from host)
    - [x] Update imports in both `krusty_host` and `krusty_simulator` (for trajectory, s_curve, snap_crackle)
    - [x] Ensure all dependencies (types, traits) are also available in `krusty_shared` (for trajectory, s_curve, snap_crackle)

- [x] **Input Shaping**
    - [x] Move concrete input shaper structs (e.g., `ZVDShaper`, `SineWaveShaper`, `PerAxisInputShapers`) from `motion/shaper.rs` to `krusty_shared` (complete, file deleted from host)
    - [x] Ensure trait definitions are unified and not duplicated (all logic now in `krusty_shared::shaper`)
    - [x] Update all usages in both host and simulator (all imports use `krusty_shared::shaper`)

- [x] **Print Job State/Enums**
    - [x] Move `JobState`, `PrintJobError`, and related data structures from `print_job.rs` to `krusty_shared` (complete; simulator does not use these types)
    - [x] Refactor code to use shared types (host only)

- [x] **Common Data Models**
    - [x] Identify and move shared data models from `web/models.rs` (e.g., printer status, job status)
    - [x] Update serialization/deserialization logic as needed
    - [x] Refactor both host and simulator to use shared models (simulator: N/A)

- [ ] **Testing & Validation**
    - [ ] Run `cargo check` and `cargo test` in both `krusty_host` and `krusty_simulator`
    - [ ] Fix any breakages or import issues
    - [ ] Ensure all moved code is covered by tests

- [ ] **Documentation**
    - [ ] Update module-level docs and README as needed
    - [ ] Document any API changes or migration notes

---

## Dependency Mapping
For each file/module to be moved, explicitly list dependencies (types, traits, modules) that must also be available in `krusty_shared`. Update this section as you discover new dependencies during migration.

- **motion/trajectory.rs**: Depends on std::collections::VecDeque, thiserror::Error, tracing (all present in krusty_shared; duplicate MotionType and related types removed from host)
- **motion/s_curve.rs**: Depends on thiserror::Error, krusty_shared::event_queue::{SimEventQueue, SimClock, SimEvent, SimEventType}, krusty_shared::StepCommand, std::sync::{Arc, Mutex}, std::time::Duration (all present in krusty_shared)
- **motion/planner/snap_crackle.rs**: Depends on crate::trajectory::MotionType, std::f64, std::cmp, std::collections (all present in krusty_shared)
- **motion/shaper.rs**: Depends on std::f64, std::vec::Vec (all present in krusty_shared; all logic unified in krusty_shared::shaper)
- **print_job.rs** (JobState, PrintJobError): Now in `krusty_shared::print_job` (depends on thiserror, std::sync, serde, etc; host uses shared types, simulator N/A)
- **web/models.rs**: Now in `krusty_shared::api_models` (depends on serde::{Serialize, Deserialize})

---

## API Compatibility Notes
- Track any breaking changes to public APIs as code is moved/refactored.
- Document required refactors in dependent crates (host, simulator, tests).
- Note any changes to serialization formats or trait bounds.
    - The `MotionType` enum and related trajectory types are now only defined in `krusty_shared::trajectory`. All code must use the shared version. Duplicates in host have been removed.
    - All API models for web endpoints are now defined in `krusty_shared::api_models` and re-exported in host.
    - Print job state/enums are now unified in `krusty_shared::print_job` and used by host only.

---

## Testing Strategy
- After each migration step, run `cargo check` and `cargo test` in both `krusty_host` and `krusty_simulator`.
- Add or expand unit/integration tests for all moved code.
- Ensure all edge cases and error paths are tested.
- Use CI or local scripts to verify cross-crate compatibility.

---

## Rollback Plan
- Commit after each successful migration step.
- If a major issue is discovered, revert to the previous commit.
- Document any issues and lessons learned for future migrations.

---

## Timeline & Ownership (Optional)
- Assign steps to team members if working collaboratively.
- Set target dates for each migration phase.
- Track progress in this document or a project board.

---

_Last updated: July 26, 2025_
