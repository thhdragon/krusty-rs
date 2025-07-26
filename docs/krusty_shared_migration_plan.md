# Migration Plan: Sharing Logic Between krusty_host and krusty_simulator

## Overview

This document identifies functions, modules, and logic in `krusty_host` that should be migrated to `krusty_shared` to enable code reuse and maintain a single source of truth for core 3D printer logic, simulation, and hardware abstraction.

---

## Candidate Modules/Functions for Migration

### 1. G-code Parsing and Utilities
- **Current Location:** `krusty_host::gcode::*`
- **Rationale:** Both host and simulator need to parse, interpret, and process G-code lines, including macro expansion and context management.
- **Action:** Move core G-code parsing logic, macro processor, and context utilities to `krusty_shared::gcode_utils`.  
- **Status:** ✅ Completed

### 2. Hardware Abstractions & Board Config
- **Current Location:** `krusty_host::hardware::board_config`, `hardware::HardwareManager`
- **Rationale:** Board configuration, pin mapping, and hardware abstraction traits are required for both real and simulated hardware.
- **Action:** Move board abstraction traits, config structs, and pin mapping logic to `krusty_shared::hardware`.  
- **Status:** ✅ Completed

### 3. Motion Planning Traits & Types
- **Current Location:** `krusty_host::motion::*`
- **Rationale:** Motion planning, kinematics, and trajectory generation are needed for both real execution and simulation.
- **Action:** Move traits, error types, and core motion planning logic to `krusty_shared::trajectory` and `krusty_shared::motion`.  
- **Status:** ✅ Completed

### 4. Event Queue & Simulation Clock
- **Current Location:** Already in `krusty_shared::event_queue`
- **Rationale:** Both host and simulator use the event queue and simulation clock for event-driven execution.
- **Action:** Ensure all event types and queue logic are fully migrated and remove any duplicate code in `krusty_simulator`.  
- **Status:** ✅ Centralized in `krusty_shared::event_queue`

### 5. API Models
- **Current Location:** Already in `krusty_shared::api_models`
- **Rationale:** Shared API request/response types for web and simulator endpoints.
- **Action:** Continue to re-export and extend as needed.  
- **Status:** ✅ Up to date

### 6. Temperature, Heater, Fan, and Switch State
- **Current Location:** `krusty_host::hardware::*`, `krusty_simulator::simulator::*`
- **Rationale:** Both host and simulator need to model and update hardware state (heaters, fans, switches).
- **Action:** Move state structs and update logic to `krusty_shared::hardware`.  
- **Status:** ⏳ Next actionable step

### 7. Auth Traits
- **Current Location:** `krusty_host::web::auth`
- **Rationale:** Auth backends may be used in both host and simulator (for web API, test harnesses).
- **Action:** Move `AuthBackend` trait and basic implementations to `krusty_shared`.  
- **Status:** ⏳ Pending

### 8. Utility Types and Traits
- **Current Location:** Scattered in `krusty_host` and `krusty_simulator`
- **Rationale:** Traits like `TimeInterface`, `Kinematics`, and input shaping are already in `krusty_shared`, but ensure all utility traits are centralized.
- **Action:** Audit and migrate any missing utility traits.  
- **Status:** ⏳ Pending

---

## Migration Steps

- [x] Inventory all logic in `krusty_host` that is duplicated or needed in `krusty_simulator`.
- [x] Move G-code parsing, macro, and context logic to `krusty_shared::gcode_utils`.
- [x] Move board config, hardware traits, and state structs to `krusty_shared::hardware`.
- [x] Move motion planning traits, error types, and core logic to `krusty_shared::trajectory` and `krusty_shared::motion`.
- [x] Ensure all event queue and simulation clock logic is only in `krusty_shared::event_queue`.
- [ ] Move `AuthBackend` trait and basic implementations to `krusty_shared`.
- [ ] Audit for any remaining utility types/traits and centralize in `krusty_shared`.
- [ ] Update imports in both `krusty_host` and `krusty_simulator` to use the new shared modules.
- [ ] Remove any now-duplicate code from `krusty_host` and `krusty_simulator`.
- [ ] Test both host and simulator to ensure all shared logic works as intended.

---

## Rationale

Centralizing shared logic in `krusty_shared`:
- Reduces code duplication
- Ensures consistent behavior between host and simulator
- Simplifies maintenance and future feature development
- Enables simulator-first and CI-driven development workflows

---

## Next Steps

- [ ] Review this plan with the team.
- [ ] Begin incremental migration, validating after each step.
- [ ] **Next:** Migrate temperature, heater, fan, and switch state structs and logic to `krusty_shared::hardware`.
- [ ] Update this document as new shared needs are discovered.

---
