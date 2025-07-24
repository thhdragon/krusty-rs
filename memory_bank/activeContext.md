# Active Context


## Current Focus
With motion queueing, execution, and G-code macro/streaming parsing now fully implemented and tested (see TASK001, TASK003), the focus has shifted to:
- Print job management (queue, pause, resume, cancel) in `print_job.rs` (TASK002)
- Web API endpoints for pause, resume, cancel, status, and authentication (TASK004)
- Modularizing hardware interfaces and integrating with the main workflow (TASK005)
- Host OS abstraction: serial protocol, time sync, event system (TASK006)
- Refactoring error handling to use robust enums and context (TASK007)
- Increasing unit/integration test coverage and automating result collection (TASK008)


## Recent Changes
- Motion queueing and execution (TASK001) fully implemented, tested, and documented (see `docs/motion_queue_api.md`)
- G-code macro/streaming parsing and macro expansion (TASK003) fully refactored to use async streaming, recursive macro expansion, and robust error handling; all tests pass
- Advanced motion planning (G⁴, Bézier blending, input shaping) implemented and validated with unit tests
- Integration of adaptive optimizer and feedback-driven parameter tuning
- Refactored test structure; all major subsystems have dedicated test files
- Improved error handling in some modules (custom enums, `thiserror`)


## Next Steps
- Integrate print job state with G-code streaming and error recovery (TASK002)
- Add web API endpoints for pause, resume, cancel, status, and authentication (TASK004)
- Modularize hardware interfaces and integrate with main workflow (TASK005)
- Implement serial protocol and host OS abstraction (TASK006)
- Refactor error handling to use robust enums and propagate with context (TASK007)
- Increase unit/integration test coverage, especially for error and edge cases (TASK008)

## Active Decisions & Considerations
- Advanced features (vibration cancellation, higher-order controller, optimizer, multi-axis optimization, hardware-accelerated step generation) are deferred to `future_map.md`
- Focus is on extensibility, modularity, and robust diagnostics before expanding feature set
- Motion queueing and execution is now considered stable and complete; future changes will be tracked as new tasks if needed
