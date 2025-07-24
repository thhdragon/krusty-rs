// snap_crackle_tests.rs
// Unit tests for G⁴ profile, Bézier blending, and input shaper logic

#[cfg(test)]
mod tests {
    use super::*;
    use crate::motion::shaper::{ZVDShaper, SineWaveShaper, InputShaperTrait};

    #[test]
    fn test_g4_profile_solver_and_eval() {
        let mut profile = G4MotionProfile {
            phases: G4ProfilePhases { phases: [0.0; 31] },
            limits: G4KinematicLimits {
                max_velocity: 100.0,
                max_accel: 1000.0,
                max_jerk: 100.0,
                max_snap: 1000.0,
                max_crackle: 5000.0,
            },
            start_velocity: 0.0,
            end_velocity: 0.0,
            distance: 10.0,
        };
        profile.solve_phases();
        // Check that the phases array is the right size and all values are positive
        assert_eq!(profile.phases.phases.len(), 31);
        for &phase in &profile.phases.phases {
            assert!(phase > 0.0);
        }
        // Check that the sum of all phase durations matches expected total time
        let v_lim = 100.0_f64
            .min(1000.0_f64.sqrt())
            .min(100.0_f64.powf(1.0/3.0))
            .min(1000.0_f64.powf(1.0/4.0))
            .min(5000.0_f64.powf(1.0/5.0));
        let expected_total_time = 10.0 / v_lim;
        let total_time: f64 = profile.phases.phases.iter().sum();
        assert!((total_time - expected_total_time).abs() < 1e-6);
        // Evaluate at several points in time
        for t in [0.0, total_time/4.0, total_time/2.0, 3.0*total_time/4.0, total_time] {
            let v = profile.velocity_at(t);
            let a = profile.acceleration_at(t);
            let j = profile.jerk_at(t);
            let s = profile.snap_at(t);
            let c = profile.crackle_at(t);
            assert!(v.abs() <= profile.limits.max_velocity + 1e-6);
            assert!(a.abs() <= profile.limits.max_accel + 1e-6);
            assert!(j.abs() <= profile.limits.max_jerk + 1e-6);
            assert!(s.abs() <= profile.limits.max_snap + 1e-6);
            assert!(c.abs() <= profile.limits.max_crackle + 1e-6);
        }
    }

    #[test]
    fn test_bezier_blender_blend_corner() {
        let blender = BezierBlender::new(15, 0.5);
        let p0 = [0.0, 0.0];
        let p1 = [1.0, 1.0];
        let p2 = [2.0, 0.0];
        let blended = blender.blend_corner(p0, p1, p2);
        // Should return at least 3 points (start, control, end)
        assert!(blended.len() >= 3);
        // Start and end should match input
        assert_eq!(blended[0], p0);
        assert_eq!(blended[blended.len()-1], p2);
    }

    #[test]
    fn test_zvd_shaper_basic() {
        let mut shaper = ZVDShaper::new(1, [0.5, 0.5]);
        let input = 1.0;
        let output = shaper.do_step(input);
        // Output should be finite
        assert!(output.is_finite());
    }

    #[test]
    fn test_sine_wave_shaper_basic() {
        let mut shaper = SineWaveShaper::new(0.1, 1.0, 0.01);
        let input = 1.0;
        let output = shaper.do_step(input);
        // Output should be finite
        assert!(output.is_finite());
    }

    #[test]
    fn test_g4_profile_phases_struct() {
        let phases = G4ProfilePhases { phases: [0.1; 31] };
        assert_eq!(phases.phases.len(), 31);
        assert!((phases.phases[0] - 0.1).abs() < 1e-8);
    }

    #[test]
    fn test_g4_motion_profile_struct() {
        let profile = G4MotionProfile {
            phases: G4ProfilePhases { phases: [0.0; 31] },
            limits: G4KinematicLimits {
                max_velocity: 100.0,
                max_accel: 1000.0,
                max_jerk: 10000.0,
                max_snap: 100000.0,
                max_crackle: 1000000.0,
            },
            start_velocity: 0.0,
            end_velocity: 0.0,
            distance: 100.0,
        };
        assert_eq!(profile.phases.phases.len(), 31);
        assert_eq!(profile.limits.max_velocity, 100.0);
    }

    #[test]
    fn test_g4_profile_zero_and_negative_limits() {
        let mut profile = G4MotionProfile {
            phases: G4ProfilePhases { phases: [0.0; 31] },
            limits: G4KinematicLimits {
                max_velocity: 0.0,
                max_accel: -1000.0,
                max_jerk: 0.0,
                max_snap: -1000.0,
                max_crackle: 0.0,
            },
            start_velocity: 0.0,
            end_velocity: 0.0,
            distance: 0.0,
        };
        // Should not panic, but all phases should be zero or non-negative
        profile.solve_phases();
        for &phase in &profile.phases.phases {
            assert!(phase >= 0.0);
        }
    }

    #[test]
    fn test_g4_profile_infinite_limits() {
        let mut profile = G4MotionProfile {
            phases: G4ProfilePhases { phases: [0.0; 31] },
            limits: G4KinematicLimits {
                max_velocity: f64::INFINITY,
                max_accel: f64::INFINITY,
                max_jerk: f64::INFINITY,
                max_snap: f64::INFINITY,
                max_crackle: f64::INFINITY,
            },
            start_velocity: 0.0,
            end_velocity: 0.0,
            distance: 10.0,
        };
        profile.solve_phases();
        // All phases should be zero (instant motion)
        let total: f64 = profile.phases.phases.iter().sum();
        assert!(total.abs() < 1e-8);
    }

    #[test]
    fn test_g4_profile_negative_velocity_and_displacement() {
        let mut profile = G4MotionProfile {
            phases: G4ProfilePhases { phases: [0.0; 31] },
            limits: G4KinematicLimits {
                max_velocity: 100.0,
                max_accel: 1000.0,
                max_jerk: 100.0,
                max_snap: 1000.0,
                max_crackle: 5000.0,
            },
            start_velocity: -50.0,
            end_velocity: -10.0,
            distance: -20.0,
        };
        profile.solve_phases();
        // Should not panic, phases should be non-negative
        for &phase in &profile.phases.phases {
            assert!(phase >= 0.0);
        }
    }

    #[test]
    fn test_g4_profile_zero_displacement_nonzero_velocity() {
        let mut profile = G4MotionProfile {
            phases: G4ProfilePhases { phases: [0.0; 31] },
            limits: G4KinematicLimits {
                max_velocity: 100.0,
                max_accel: 1000.0,
                max_jerk: 100.0,
                max_snap: 1000.0,
                max_crackle: 5000.0,
            },
            start_velocity: 10.0,
            end_velocity: 10.0,
            distance: 0.0,
        };
        profile.solve_phases();
        // Should not panic, phases should be zero
        let total: f64 = profile.phases.phases.iter().sum();
        assert!(total.abs() < 1e-8);
    }

    #[test]
    #[should_panic]
    fn test_g4_profile_all_limits_zero_should_panic() {
        let mut profile = G4MotionProfile {
            phases: G4ProfilePhases { phases: [0.0; 31] },
            limits: G4KinematicLimits {
                max_velocity: 0.0,
                max_accel: 0.0,
                max_jerk: 0.0,
                max_snap: 0.0,
                max_crackle: 0.0,
            },
            start_velocity: 0.0,
            end_velocity: 0.0,
            distance: 10.0,
        };
        profile.solve_phases();
    }

    #[test]
    fn test_g4_profile_randomized_stability() {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        for _ in 0..100 {
            let limits = G4KinematicLimits {
                max_velocity: rng.gen_range(1.0..1000.0),
                max_accel: rng.gen_range(1.0..10000.0),
                max_jerk: rng.gen_range(1.0..1000.0),
                max_snap: rng.gen_range(1.0..10000.0),
                max_crackle: rng.gen_range(1.0..10000.0),
            };
            let mut profile = G4MotionProfile {
                phases: G4ProfilePhases { phases: [0.0; 31] },
                limits,
                start_velocity: rng.gen_range(-100.0..100.0),
                end_velocity: rng.gen_range(-100.0..100.0),
                distance: rng.gen_range(-100.0..100.0),
            };
            profile.solve_phases();
            for &phase in &profile.phases.phases {
                assert!(phase >= 0.0);
            }
        }
    }

    #[test]
    fn test_g4_profile_performance() {
        use std::time::Instant;
        let limits = G4KinematicLimits {
            max_velocity: 100.0,
            max_accel: 1000.0,
            max_jerk: 100.0,
            max_snap: 1000.0,
            max_crackle: 5000.0,
        };
        let start = Instant::now();
        for _ in 0..1000 {
            let mut profile = G4MotionProfile {
                phases: G4ProfilePhases { phases: [0.0; 31] },
                limits: limits.clone(),
                start_velocity: 0.0,
                end_velocity: 0.0,
                distance: 10.0,
            };
            profile.solve_phases();
        }
        let elapsed = start.elapsed();
        // Should complete in under 1 second for 1000 solves
        assert!(elapsed.as_secs_f64() < 1.0);
    }
}
