# TASK003 - G-code Macro/Streaming Parsing and Macro Expansion

**Status:** In Progress  
**Added:** 2025-07-24  
**Updated:** 2025-07-24

## Original Request
Implement robust G-code macro/streaming parsing and macro expansion. Work is focused in `gcode/macros.rs` and `gcode/parser.rs`. The system should support async/streaming parsing, macro expansion, error recovery, and integration with print job and motion systems.

## Thought Process
- Macro and streaming parsing are essential for flexible, programmable G-code workflows.
- The parser must support async/streaming input, macro expansion, and robust error recovery.
- Integration with print job management and the motion queue is required for seamless job execution.
- The design should be modular, testable, and extensible for future macro features.
- Thread safety and async coordination are important for real-time and web API integration.

## Implementation Plan
- [x] Review current state of `gcode/macros.rs` and `gcode/parser.rs`
- [x] Design macro expansion and streaming parsing architecture (async, error recovery)
- [ ] Implement macro expansion logic and streaming parser
- [ ] Integrate with print job management and motion queue
- [ ] Add error handling and state transitions
- [ ] Write unit and integration tests for macro/streaming parsing
- [ ] Document the API and usage patterns

## Progress Tracking

**Overall Status:** In Progress - 35%

### Subtasks
| ID  | Description                                      | Status       | Updated     | Notes |
|-----|--------------------------------------------------|--------------|-------------|-------|
| 3.1 | Review current code and document findings         | Complete     | 2025-07-24  | MacroProcessor and GCodeParser support macros, async parsing, and expansion. Integration points exist. |
| 3.2 | Design macro/streaming parsing architecture       | Complete     | 2025-07-24  | See progress log for detailed design notes. |
| 3.3 | Implement macro expansion and streaming parser    | In Progress  | 2025-07-24  | See progress log for incremental plan. |
| 3.4 | Integrate with print job and motion queue         | Not Started  |             | |
| 3.5 | Add error handling and state transitions          | Not Started  |             | |
| 3.6 | Write tests                                      | Not Started  |             | |
| 3.7 | Document API and usage                           | Not Started  |             | |

## Progress Log
### 2025-07-24
- Task file created and initialized with plan and subtasks.
- Began review of `gcode/macros.rs` and `gcode/parser.rs` for current macro and streaming parsing logic.
- Completed review: MacroProcessor manages macro storage and expansion; GCodeParser supports macros, comments, checksums, and async parsing. Integration points exist for dispatching expanded commands. Next: design improvements for async trait usage, error recovery, and streaming integration.

### 2025-07-24
**Macro/Streaming Parsing Architecture Design:**
- **Async Trait Usage:**
  - Use `async_trait` to define async parsing interfaces (e.g., `async fn parse_next(&mut self) -> Result<Option<GCodeCommand>, ParseError>`).
  - Allow parser to consume from any async source (file, network, channel).
- **Streaming Parsing:**
  - Implement parser as a `futures::Stream` of `Result<GCodeCommand, ParseError>`.
  - Support backpressure and cancellation via stream combinators.
- **Macro Expansion Flow:**
  - MacroProcessor expands macros as commands are parsed.
  - Expanded commands are injected into the stream before continuing with the source.
  - Support nested macro expansion and recursion limits.
- **Error Recovery:**
  - Use a robust `ParseError` enum with context (line, macro, source).
  - On error, log and attempt to resync at the next valid command boundary.
  - Allow error propagation to print job manager for user feedback.
- **Integration Points:**
  - Parser feeds expanded commands to print job manager via async channel or callback.
  - Print job manager coordinates with motion queue for execution.
- **Thread Safety & Coordination:**
  - Use `Arc<Mutex<...>>` or `tokio::sync::Mutex` for shared state if needed.
  - Prefer message passing (channels) for async coordination.
- **Extensibility:**
  - Design for future macro features (parameters, conditionals, includes).

Next: begin implementation of macro expansion and streaming parser (subtask 3.3).

### 2025-07-24
**Implementation Plan for Macro Expansion and Streaming Parser:**
1. Refactor `GCodeParser` to implement `futures::Stream` for async streaming parsing.
2. Ensure macro expansion is async and integrates with the stream (buffer expanded commands, yield before continuing source).
3. Add support for backpressure and cancellation (stream combinators).
4. Test with a mock macro source and print job integration.

Subtask 3.3 is now In Progress.
