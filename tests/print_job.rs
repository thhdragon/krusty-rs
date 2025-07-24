use krusty_rs::print_job::{PrintJobManager, PrintJobError};
use krusty_rs::GCodeCommand;
use krusty_rs::gcode::parser::GCodeError;
use tokio::sync::mpsc;

fn dummy_command() -> Result<GCodeCommand<'static>, GCodeError> {
    use krusty_rs::gcode::parser::GCodeSpan;
    Ok(GCodeCommand::Comment("test", GCodeSpan { range: 0..0 }))
}

#[tokio::test]
async fn test_valid_state_transitions() {
    let (tx, _rx) = mpsc::channel(8);
    let manager = PrintJobManager::new(tx);
    let job_id = manager.enqueue_job(vec![dummy_command()]).await;
    assert_eq!(manager.start_next_job().await.unwrap(), job_id);
    assert_eq!(manager.pause_current_job().await.unwrap(), job_id);
    assert_eq!(manager.resume_current_job().await.unwrap(), job_id);
    assert_eq!(manager.cancel_current_job().await.unwrap(), job_id);
}

#[tokio::test]
async fn test_invalid_state_transitions() {
    let (tx, _rx) = mpsc::channel(8);
    let manager = PrintJobManager::new(tx);
    let job_id = manager.enqueue_job(vec![dummy_command()]).await;
    // Can't pause before running
    let err = manager.pause_current_job().await.unwrap_err();
    assert!(matches!(err, PrintJobError::InvalidTransition(_)));
    // Start and complete job
    assert_eq!(manager.start_next_job().await.unwrap(), job_id);
    manager.cancel_current_job().await.unwrap();
    // Can't resume a cancelled job
    let err = manager.resume_current_job().await.unwrap_err();
    assert!(matches!(err, PrintJobError::InvalidTransition(_)));
}

#[tokio::test]
async fn test_next_command_and_completion() {
    let (tx, _rx) = mpsc::channel(8);
    let manager = PrintJobManager::new(tx);
    manager.enqueue_job(vec![dummy_command(), dummy_command()]).await;
    manager.start_next_job().await.unwrap();
    // Pop first command
    let cmd1 = manager.next_command().await.unwrap();
    assert!(cmd1.is_some());
    // Pop second command, should complete job
    let cmd2 = manager.next_command().await.unwrap();
    assert!(cmd2.is_some());
    // Now job should be completed
    let err = manager.pause_current_job().await.unwrap_err();
    assert!(matches!(err, PrintJobError::InvalidTransition(_)));
}
