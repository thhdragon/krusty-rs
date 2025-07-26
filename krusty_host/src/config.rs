//! # Motion Shaper and Blending Configuration
//!
//! This module defines configuration structs for advanced motion planning, input shaper, and blending options.
//!
//! ## Example: TOML Configuration
//!
//! ```toml
//! [motion.shaper.x]
//! type = "zvd"
//! frequency = 40.0
//! damping = 0.1
//!
//! [motion.shaper.y]
//! type = "sine"
//! frequency = 35.0
//!
//! [motion.blending]
//! type = "bezier"
//! max_deviation = 0.2
//! ```
//!
//! - Each axis (x, y, z, e) can have its own shaper type and parameters.
//! - Blending (corner smoothing) is configured globally or per-axis as needed.
//!
//! ## Example: Rust Usage
//!
//! ```rust
//! use krusty_host::config::Config;
//! let toml_str = r#"
//! [motion.shaper.x]
//! type = "zvd"
//! frequency = 40.0
//! damping = 0.1
//!
//! [motion.blending]
//! type = "bezier"
//! max_deviation = 0.2
//! "#;
//! let config: Config = toml::from_str(toml_str).unwrap();
//! let motion = config.motion.as_ref().unwrap();
//! assert_eq!(motion.shaper["x"].frequency, 40.0);
//! assert_eq!(motion.blending.as_ref().unwrap().max_deviation, 0.2);
//! // Validate config
//! assert!(motion.validate().is_ok());
//! ```
//!
//! See also: `src/motion/planner/mod.rs` for planner integration and assignment logic.

// src/config.rs - Single configuration file
pub use krusty_shared::config::*;