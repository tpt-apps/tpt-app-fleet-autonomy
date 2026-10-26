use std::f64::consts::{PI, TAU};

use crate::backend::{Command, GRAVITY, GROUND_MAX_SPEED, GROUND_MAX_TURN};

/// Distance at which an agent counts as having reached its goal, in metres.
pub const GOAL_TOLERANCE: f64 = 0.2;

/// Heading error above which a ground robot turns in place rather than driving.
const DRIVE_HEADING_LIMIT: f64 = PI / 4.0;
/// Proportional gain on heading error for ground robots.
const TURN_GAIN: f64 = 2.0;
/// Proportional gain on position error for drones.
const DRONE_KP: f64 = 1.0;
/// Velocity damping gain for drones. With `DRONE_KP = 1` this is critically damped.
const DRONE_KD: f64 = 2.0;

/// Drives a ground robot toward `goal` with a proportional heading controller.
pub fn ground_command(position: [f64; 3], heading: f64, goal: [f64; 3]) -> Command {
    let dx = goal[0] - position[0];
    let dy = goal[1] - position[1];
    let distance = dx.hypot(dy);
    if distance <= GOAL_TOLERANCE {
        return Command::Ground {
            speed: 0.0,
            turn_rate: 0.0,
        };
    }
    let error = wrap_angle(dy.atan2(dx) - heading);
    let turn_rate = (TURN_GAIN * error).clamp(-GROUND_MAX_TURN, GROUND_MAX_TURN);
    let speed = if error.abs() > DRIVE_HEADING_LIMIT {
        0.0
    } else {
        distance.min(GROUND_MAX_SPEED)
    };
    Command::Ground { speed, turn_rate }
}

/// Steers a drone toward `goal` with a PD law plus gravity compensation.
pub fn drone_command(position: [f64; 3], velocity: [f64; 3], goal: [f64; 3]) -> Command {
    let accel = [0, 1, 2].map(|i| DRONE_KP * (goal[i] - position[i]) - DRONE_KD * velocity[i]);
    Command::Drone {
        accel: [accel[0], accel[1], accel[2] + GRAVITY],
    }
}

/// Maps an angle into `(-PI, PI]`.
fn wrap_angle(angle: f64) -> f64 {
    (angle + PI).rem_euclid(TAU) - PI
}
