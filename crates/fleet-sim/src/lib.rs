//! Built-in simulation for the TPT Fleet Autonomy Suite.
//!
//! The simulation runs in SI units (metres, seconds, radians). Ground robots use a unicycle
//! model and drones use a point-mass model with gravity. Both are integrated with the
//! fixed-step RK4 solver from `tpt-math-numeric`.
//!
//! The [`SimBackend`] trait is the seam for other engines (such as Gazebo, later). The
//! [`BuiltinBackend`] is the default implementation. [`run`] drives a [`Scenario`] through
//! any backend and records a trace that a seeded run reproduces exactly.

mod backend;
mod control;
mod rng;
mod run;
mod scenario;
mod sensor;

pub use backend::{
    AgentState, BuiltinBackend, Collision, Command, SimBackend, SimError, DRONE_MAX_THRUST_ACCEL,
    GRAVITY, GROUND_MAX_SPEED, GROUND_MAX_TURN, ROBOT_RADIUS,
};
pub use control::GOAL_TOLERANCE;
pub use rng::Rng;
pub use run::{run, RunResult, Sample};
pub use scenario::{AgentSpec, Obstacle, Scenario};
pub use sensor::{lidar_scan, LidarConfig};
