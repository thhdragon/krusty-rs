# Motion Queueing and Execution API

## Motion Queueing API (Controller)

The `MotionController` exposes the following queue control methods:

- `queue_linear_move(target: [f64; 3], feedrate: Option<f64>, extrude: Option<f64>) -> Result<(), Box<dyn std::error::Error>>` (async): Queue a new linear move.
- `pause_queue(&mut self) -> Result<(), MotionError>`: Pause the motion queue.
- `resume_queue(&mut self) -> Result<(), MotionError>`: Resume the motion queue if paused.
- `cancel_queue(&mut self) -> Result<(), MotionError>`: Cancel and clear the motion queue.
- `get_queue_state(&self) -> MotionQueueState`: Query the current queue state (`Idle`, `Running`, `Paused`, `Cancelled`).

## Motion Queueing API (Planner)

The `MotionPlanner` provides the following methods:

- `plan_move(target: [f64; 4], feedrate: f64, motion_type: MotionType) -> Result<(), MotionError>` (async): Queue a new move segment.
- `pause(&mut self) -> Result<(), MotionError>`: Pause the queue (only valid if running).
- `resume(&mut self) -> Result<(), MotionError>`: Resume the queue (only valid if paused).
- `cancel(&mut self) -> Result<(), MotionError>`: Cancel and clear the queue.
- `get_state(&self) -> MotionQueueState`: Get the current queue state.

## Usage Example

```rust
// Create a controller (simplified)
let mut controller = MotionController::new(state, hardware_manager, MotionMode::Basic, &config);

// Queue a move
controller.queue_linear_move([100.0, 100.0, 0.0], Some(300.0), None).await?;

// Pause the queue
controller.pause_queue()?;

// Resume the queue
controller.resume_queue()?;

// Cancel the queue
controller.cancel_queue()?;

// Check queue state
let state = controller.get_queue_state();
println!("Current queue state: {:?}", state);
```

## Error Handling and State Transitions

- All queue control methods return a `Result<(), MotionError>`.
- Invalid state transitions (e.g., pausing when already paused) return an error.
- State transitions:
  - `Running` → `Paused` (pause)
  - `Paused` → `Running` (resume)
  - Any → `Cancelled` (cancel, clears queue)
  - `Idle` is the default state when no moves are queued.

See also: `src/motion/controller.rs` and `src/motion/planner/mod.rs` for implementation details and further extension points.
