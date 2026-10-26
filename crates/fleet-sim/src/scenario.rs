use std::collections::HashSet;

use fleet_core::RobotKind;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::backend::SimError;

/// An axis-aligned box that agents cannot enter. Coordinates are in metres.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Obstacle {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

/// One agent in a scenario. A `goal` of `None` means the agent is left idle.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AgentSpec {
    pub id: Uuid,
    pub kind: RobotKind,
    pub start: [f64; 3],
    #[serde(default)]
    pub goal: Option<[f64; 3]>,
}

/// Everything needed to reproduce a run: world, agents, timing and seed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scenario {
    pub name: String,
    pub obstacles: Vec<Obstacle>,
    pub agents: Vec<AgentSpec>,
    pub duration_s: f64,
    pub dt: f64,
    /// Seed for every random draw in the run. The same seed gives the same trace.
    pub seed: u64,
    /// Half-width of the uniform noise added to the observed position, in metres.
    pub sensor_noise: f64,
}

/// Largest step the built-in integrator accepts, in seconds.
pub const MAX_DT: f64 = 0.1;

impl Scenario {
    /// Rejects inputs that cannot be simulated. Checks ranges, finiteness and id uniqueness.
    pub fn validate(&self) -> Result<(), SimError> {
        let invalid = |msg: String| Err(SimError::Invalid(msg));
        if !self.duration_s.is_finite() || self.duration_s <= 0.0 {
            return invalid("duration_s must be a positive number".into());
        }
        if !self.dt.is_finite() || self.dt <= 0.0 || self.dt > MAX_DT {
            return invalid(format!("dt must be in (0, {MAX_DT}]"));
        }
        if !self.sensor_noise.is_finite() || self.sensor_noise < 0.0 {
            return invalid("sensor_noise must be zero or positive".into());
        }
        for obstacle in &self.obstacles {
            for axis in 0..3 {
                if !obstacle.min[axis].is_finite() || !obstacle.max[axis].is_finite() {
                    return invalid("obstacle coordinates must be finite".into());
                }
                if obstacle.min[axis] > obstacle.max[axis] {
                    return invalid("obstacle min must not exceed max".into());
                }
            }
        }
        let mut ids = HashSet::new();
        for agent in &self.agents {
            if !ids.insert(agent.id) {
                return Err(SimError::DuplicateAgent(agent.id));
            }
            let points = std::iter::once(agent.start).chain(agent.goal);
            for point in points {
                if point.iter().any(|v| !v.is_finite()) {
                    return invalid(format!("agent {} has a non-finite position", agent.id));
                }
            }
        }
        Ok(())
    }
}
