//! Fleet Project document model: robots, skills, worlds, deployments and observed state.
//!
//! This crate is pure data plus validation. It performs no I/O; persistence lives in `fleet-store`.

mod error;
mod project;
mod robot;

pub use error::Error;
pub use project::{Deployment, FleetProject, ObservedState, Skill, World, SCHEMA_VERSION};
pub use robot::{Actuator, Adapter, Pose, Robot, RobotKind, Sensor, SensorKind};

/// Result type used throughout `fleet-core`.
pub type Result<T> = std::result::Result<T, Error>;
