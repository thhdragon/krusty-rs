---
applyTo: '**'
---

**Absolutely no code truncation or placeholder comments are allowed in any code file, under any circumstances. Every file must contain the full, unabridged implementation.**
# Migration & Refactor Rules

**Scope:** Applies to all Rust source files in this repository.

## 1. No Stubs or Truncation
- All code must be fully migrated or refactored.
- Do not use placeholders, stubs, or omit any implementation details.
- Do not use comments such as `// ...existing code...` or `// (Implementation omitted for brevity)`.

## 2. Reference Implementations
- For motion and kinematics, refer to `sim_reference/`.
- Use `sim_reference/prunt/` for advanced motion planning and kinematics.
- See `sim_reference/kalico/` for advanced Klipper fork logic.
- Use `sim_reference/RepRapFirmware/` for optimized reference code.

## 3. Rationale
- This ensures all features and logic are preserved and no functionality is lost during migration or refactor.

## 4. Examples
- **Don't:**
  ```rust
  // ...existing code...
  ```
- **Do:**
  ```rust
  // Full implementation migrated here
  ```

## 5. Checklist Before Submitting
- [ ] No stubs or truncated code present
- [ ] All logic fully migrated
- [ ] Reference implementations consulted as needed