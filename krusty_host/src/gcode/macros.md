# G-code Macro System Refactor Plan

## 1. Problems in the Old Implementation
- Overly complex, monolithic code with unclear responsibilities
- Inconsistent async usage and locking scope (potential for deadlocks or performance issues)
- Poor separation of macro parsing, expansion, and execution
- Error handling is ad-hoc and not ergonomic
- Cycle detection logic is duplicated and not robust
- No clear trait boundaries for macro expansion (hard to test or extend)
- Lacks documentation and idiomatic Rust patterns

## 2. Goals for the Refactor
- Clean, modular, and idiomatic Rust code
- Clear separation of macro storage, parsing, expansion, and execution
- Fully async and thread-safe (using `Arc`, `RwLock`, minimal lock scope)
- Robust error handling with custom error types
- Pluggable trait for macro expansion (enables testing and future extensions)
- Cycle detection that is reliable and easy to reason about
- Well-documented API and internal logic
- Easy to test and maintain

## 3. Proposed API and Structure

### MacroProcessor
- Stores macros as `HashMap<String, Vec<String>>` inside `Arc<RwLock<...>>`
- Public async methods:
    - `define_macro(name: &str, commands: Vec<String>) -> Result<()>`
    - `delete_macro(name: &str) -> Result<()>`
    - `list_macros() -> Vec<String>`
    - `expand_macro(name: &str, call_stack: &[String]) -> Result<Vec<String>>` (internal, for recursion/cycle detection)
    - `parse_and_expand_async_owned(command: &str) -> Vec<Result<OwnedGCodeCommand, GCodeError>>`
- Implements a `MacroExpander` trait for integration with the parser

### MacroExpander Trait
- Async trait for macro expansion:
    - `async fn expand(&self, name: &str, args: &str) -> Option<Vec<OwnedGCodeCommand>>`
- Enables plugging in different macro sources (for tests, config, etc.)

### Error Handling
- Custom error enum for macro errors (not just boxed errors)
- All public methods return `Result<T, MacroError>`
- Cycle detection errors are explicit and descriptive

### Async and Thread Safety
- All macro operations are async and use minimal lock scope
- No blocking or long-held locks
- All state is behind `Arc<RwLock<...>>` for safe sharing

### Testing and Extensibility
- MacroProcessor is easily mockable for tests
- MacroExpander trait allows for alternate macro sources
- All logic is covered by unit tests (define, expand, cycle, error cases)

## 4. Implementation Steps
 [x] Create new `macros.rs` with the above structure
 [x] Move macro storage and expansion logic into separate methods
 [x] Implement the `MacroExpander` trait for MacroProcessor (stubbed, logic in progress)
 [x] Refactor error handling to use a custom error type (`MacroError`)
 [x] Add comprehensive doc comments and usage examples (in progress)
 [ ] Write unit tests for all public API methods
 [ ] Migrate integration points in GCodeProcessor and tests


# Macro System Refactor: Clean Slate Summary

## Key Lessons Learned

- Async Rust requires careful lifetime management: avoid `.await` inside loops that borrow data, and never pass references to temporaries into async code.
- Use `Arc<String>` or collect macro lines before parsing to ensure correct lifetimes and avoid borrow checker issues.
- All macro storage and expansion logic should be in small, well-documented, testable methods.
- Cycle detection must be explicit and ergonomic, using a call stack and returning a clear error if recursion is detected.
- Locks must be held for minimal scope and never across `.await` points.
- Traits should be used for macro expansion to allow for easy testing and future extensibility.
- Avoid global mutable state and code duplication, especially in error handling and expansion logic.

## Fresh Start Approach

The previous implementation was cleared. The new `macros.rs` is a minimal, idiomatic, and well-documented foundation. All legacy code and complexity have been removed. The next steps are:

1. Incrementally implement macro storage, expansion, and trait logic, following the clean architecture outlined in the plan.
2. Add comprehensive unit tests for all public API methods and edge cases.
3. Migrate integration points in `GCodeProcessor` and related modules to use the new API.
4. Remove `macros.old.rs` after migration and testing are complete.

---

This document now serves as a concise, up-to-date reference for the macro system refactor. All future work should follow the lessons and approach outlined above.

This plan will guide the full rewrite of `macros.rs` for clarity, safety, and maintainability. See the Implementation Steps for the next actions.
