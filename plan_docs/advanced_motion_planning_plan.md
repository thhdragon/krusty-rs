# Advanced Motion Planning & Input Shaping: Implementation Plan

## Status & Next Steps
- **Status:** All core features (G⁴ profile, Bézier blending, modular input shaper system) are implemented, integrated, and validated with unit tests. Trait and type issues resolved; all tests pass.
- **Next Steps:**
  - Extend G⁴ and Bézier logic for full analytical solutions if desired
  - Integrate shaper/blending config into user-facing config or API
  - Continue real/simulated scenario validation

---

## Overview
This document outlines a step-by-step plan for implementing advanced motion planning and input shaping in Krusty-rs, inspired by Prunt3D’s G⁴ (31-phase) motion profile and Bézier-based corner blending. It includes actionable tasks, research links, design notes, and a todo checklist for tracking progress.

---

## Research & Inspiration
- **Prunt3D Features:** https://prunt3d.com/docs/features/
- **G⁴ Motion Profiles:** https://prunt3d.com/docs/features/#g-motion-profiles
- **Advanced Corner Blending:** https://prunt3d.com/docs/features/#advanced-corner-blending
- **Prunt3D GitHub Motion Planner:** https://github.com/Prunt3D/prunt/blob/master/src/prunt-motion_planner.adb

---

## Design Goals
- Support independent limits for velocity, acceleration, jerk, snap, and crackle (G⁴ profile)
- Integrate degree-15 Bézier curve corner blending for smooth transitions
- Modular, trait-based input shaper system (per-axis, extensible)
- Analytical/iterative solvers for all derivatives at time t
- Unit tests for all new features
- Inline and external documentation

---

## Implementation Steps
```markdown
- [x] Research Prunt3D’s G⁴ profile and Bézier blending algorithms in detail
- [x] Design Rust data structures for G⁴ profile phases and Bézier blending
- [x] Implement G⁴ profile solver and evaluation functions in `motion/planner/snap_crackle.rs`
- [x] Implement Bézier-based corner blending in `motion/planner/snap_crackle.rs`
- [x] Refactor/extend `motion/shaper.rs` to support modular, trait-based input shapers
- [x] Integrate per-axis input shaper assignment in the planner
- [x] Add configuration options for shapers and blending (future: config file/API)
- [x] Write unit tests for G⁴ profile, Bézier blending, and input shapers
- [x] Document all new types, algorithms, and configuration options
- [x] Validate with real and simulated motion scenarios
```

---

## Design Notes
- **G⁴ Profile:** 31-phase, supports independent limits for all derivatives. Uses analytical or iterative solver for phase durations and evaluation. Data structures and logic implemented in `motion/planner/snap_crackle.rs`.
- **Bézier Blending:** Degree-15 curve, configurable max deviation, ensures bounded jerk/snap/crackle. Implemented in `motion/planner/snap_crackle.rs`.
- **Input Shaper:** Trait-based, supports ZVD, Sine Wave, and future shapers. Assignable per axis. Modular system in `motion/shaper.rs` and integrated in planner.
- **Testing:** Unit tests in `motion/planner/snap_crackle_tests.rs` cover G⁴, Bézier, and shaper logic. All tests pass.
- **Documentation:** Inline Rust doc comments and updates to `project_map.md`.

---

## Testing & Validation
- All new features are covered by unit tests in `motion/planner/snap_crackle_tests.rs`.
- Tests include edge cases and integration scenarios for G⁴, Bézier, and input shapers.
- All tests pass, confirming correctness and robustness.

---

## Rust Data Structure Design: G⁴ Profile & Bézier Blending
### G⁴ (31-Phase) Motion Profile
```rust
/// Represents the phase durations for a single G⁴ (31-phase) motion segment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct G4ProfilePhases {
    /// Durations for each of the 31 phases (seconds)
    pub phases: [f64; 31],
}

/// Kinematic limits for a G⁴ profile.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct G4KinematicLimits {
    pub max_velocity: f64,    // mm/s
    pub max_accel: f64,       // mm/s^2
    pub max_jerk: f64,        // mm/s^3
    pub max_snap: f64,        // mm/s^4
    pub max_crackle: f64,     // mm/s^5
}

/// Complete G⁴ motion profile for a segment.
#[derive(Debug, Clone, PartialEq)]
pub struct G4MotionProfile {
    pub phases: G4ProfilePhases,
    pub limits: G4KinematicLimits,
    pub start_velocity: f64,
    pub end_velocity: f64,
    pub distance: f64,
}
```

### Bézier-Based Corner Blending
```rust
/// Represents a degree-15 Bézier curve for corner blending.
#[derive(Debug, Clone, PartialEq)]
pub struct BezierBlender {
    /// Control points for the Bézier curve (in 2D or 3D as needed)
    pub control_points: Vec<[f64; 2]>,
    /// Maximum deviation from the desired path (tunable parameter)
    pub max_deviation: f64,
}
```
