# PrintJobManager API Documentation

## Key Types
- `PrintJobManager`: Manages a queue of print jobs, each with its own state and command list.
- `PrintJob`: Represents a single print job, with state, commands, and progress.
- `JobState`: Enum for job state (`Queued`, `Running`, `Paused`, `Completed`, `Cancelled`, `Error`).
- `PrintJobError`: Error type for invalid transitions, missing jobs, or channel errors.

## Common Operations

```rust
// Create a PrintJobManager with a channel to the motion queue
let (tx, rx) = tokio::sync::mpsc::channel(8);
let manager = PrintJobManager::new(tx);

// Enqueue a new job (Vec of parsed GCodeCommand results)
let job_id = manager.enqueue_job(vec![Ok(GCodeCommand::Comment("test", GCodeSpan { range: 0..0 }))]).await;

// Start the next job in the queue
manager.start_next_job().await?;

// Pause, resume, or cancel the current job
manager.pause_current_job().await?;
manager.resume_current_job().await?;
manager.cancel_current_job().await?;

// Process the current job (send all commands to the motion queue)
manager.process_current_job().await?;

// Get the next command for the running job (manual streaming)
if let Some(cmd) = manager.next_command().await? {
    // handle cmd
}
```

## State Model
- Jobs start as `Queued`, become `Running` when started, can be `Paused`/`Resumed`, and end as `Completed` or `Cancelled`.
- All state transitions are validated; invalid transitions return `PrintJobError::InvalidTransition`.
- Commands are sent to the motion queue via a Tokio channel.

## Error Handling
- All async methods return `Result<T, PrintJobError>`.
- Errors are descriptive and cover all invalid state transitions and channel send failures.

## Extensibility
- The design supports future features like job prioritization, scheduling, and web API integration.

---
