// src/motion/integration.rs - Integration test for advanced planner
use crate::motion::advanced_planner::{AdvancedMotionPlanner, MotionConfig, MotionType};

pub async fn test_advanced_motion_planning() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Testing advanced motion planning...");
    
    // Create advanced motion planner
    let config = MotionConfig::default();
    let mut planner = AdvancedMotionPlanner::new(config);
    
    // Test complex motion sequence
    let moves = vec![
        ([100.0, 100.0, 10.0, 0.0], 300.0, MotionType::Travel),
        ([150.0, 150.0, 10.0, 5.0], 200.0, MotionType::Print),
        ([200.0, 100.0, 10.0, 10.0], 300.0, MotionType::Print),
        ([150.0, 50.0, 10.0, 15.0], 250.0, MotionType::Print),
        ([100.0, 100.0, 10.0, 20.0], 300.0, MotionType::Print),
    ];
    
    tracing::info!("Planning {} moves with advanced motion planning...", moves.len());
    
    for (target, feedrate, motion_type) in moves {
        match planner.plan_advanced_move(target, feedrate, motion_type).await {
            Ok(()) => {
                tracing::debug!("Planned move to [{:.1}, {:.1}, {:.1}, {:.1}] @ {:.0}mm/s",
                               target[0], target[1], target[2], target[3], feedrate);
            }
            Err(e) => {
                tracing::error!("Failed to plan move: {}", e);
                return Err(e);
            }
        }
    }
    
    // Test queue optimization
    match planner.optimize_queue().await {
        Ok(()) => tracing::info!("Motion queue optimized successfully"),
        Err(e) => tracing::warn!("Motion queue optimization failed: {}", e),
    }
    
    // Test update loop
    for i in 0..100 {
        match planner.update().await {
            Ok(()) => {
                if i % 20 == 0 {
                    tracing::debug!("Motion update cycle {}", i);
                }
            }
            Err(e) => {
                tracing::error!("Motion update error: {}", e);
                return Err(e);
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
    
    tracing::info!("Advanced motion planning test completed successfully");
    Ok(())
}