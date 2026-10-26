use std::collections::BTreeMap;
use std::fmt;

use fleet_core::RobotKind;
use serde::{Deserialize, Serialize};
use tpt_math_numeric::ode::rk4_step;
use uuid::Uuid;

use crate::scenario::Obstacle;

/// Top forward or reverse speed of a ground robot, in m/s.
pub const GROUND_MAX_SPEED: f64 = 2.0;
/// Top turn rate of a ground robot, in rad/s.
pub const GROUND_MAX_TURN: f64 = 1.5;
/// Largest thrust acceleration a drone can command, in m/s². Hover needs `GRAVITY`.
pub const DRONE_MAX_THRUST_ACCEL: f64 = 15.0;
/// Gravitational acceleration, in m/s².
pub const GRAVITY: f64 = 9.81;
/// Radius used for collision checks against obstacles, in metres.
pub const ROBOT_RADIUS: f64 = 0.25;

/// A command for one agent. The variant must match the agent's kind.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Command {
    /// Ground robot: forward speed (m/s, negative reverses) and turn rate (rad/s).
    Ground { speed: f64, turn_rate: f64 },
    /// Drone: commanded thrust acceleration vector (m/s²), gravity included.
    Drone { accel: [f64; 3] },
}

impl Command {
    /// Does nothing: stopped for a ground robot, hovering for a drone.
    pub fn idle(kind: RobotKind) -> Self {
        match kind {
            RobotKind::Ground => Command::Ground {
                speed: 0.0,
                turn_rate: 0.0,
            },
            RobotKind::Drone => Command::Drone {
                accel: [0.0, 0.0, GRAVITY],
            },
        }
    }
}

/// Where an agent is and how it is moving. `collided` is sticky: a collided agent stops.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AgentState {
    pub position: [f64; 3],
    pub velocity: [f64; 3],
    pub heading: f64,
    pub collided: bool,
}

/// A collision between an agent and an obstacle.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Collision {
    pub agent: Uuid,
    pub time: f64,
    pub position: [f64; 3],
}

/// Errors from a simulation backend.
#[derive(Debug, Clone, PartialEq)]
pub enum SimError {
    UnknownAgent(Uuid),
    DuplicateAgent(Uuid),
    /// A command whose variant does not match the agent's kind.
    KindMismatch(Uuid),
    /// A scenario or input value that cannot be simulated.
    Invalid(String),
}

impl fmt::Display for SimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SimError::UnknownAgent(id) => write!(f, "no simulated agent with id {id}"),
            SimError::DuplicateAgent(id) => write!(f, "agent {id} is already spawned"),
            SimError::KindMismatch(id) => {
                write!(f, "command does not match the kind of agent {id}")
            }
            SimError::Invalid(msg) => write!(f, "invalid simulation input: {msg}"),
        }
    }
}

impl std::error::Error for SimError {}

/// The interface every simulation engine implements.
pub trait SimBackend {
    /// Adds an agent at `start`. Ground robots ignore the z component.
    fn spawn(&mut self, id: Uuid, kind: RobotKind, start: [f64; 3]) -> Result<(), SimError>;
    /// Sets the command an agent follows until the next command.
    fn command(&mut self, id: Uuid, command: Command) -> Result<(), SimError>;
    /// Advances the world by `dt` seconds and returns the collisions that happened in this step.
    fn step(&mut self, dt: f64) -> Vec<Collision>;
    /// Current state of one agent.
    fn state(&self, id: Uuid) -> Option<AgentState>;
    /// Ids of all agents, in a stable order.
    fn agents(&self) -> Vec<Uuid>;
    /// Simulated time in seconds.
    fn time(&self) -> f64;
}

#[derive(Debug, Clone)]
struct Agent {
    kind: RobotKind,
    state: AgentState,
    command: Command,
}

/// The default backend: kinematic ground robots, point-mass drones and box obstacles.
#[derive(Debug, Clone)]
pub struct BuiltinBackend {
    obstacles: Vec<Obstacle>,
    agents: BTreeMap<Uuid, Agent>,
    time: f64,
}

impl BuiltinBackend {
    pub fn new(obstacles: Vec<Obstacle>) -> Self {
        Self {
            obstacles,
            agents: BTreeMap::new(),
            time: 0.0,
        }
    }
}

impl SimBackend for BuiltinBackend {
    fn spawn(&mut self, id: Uuid, kind: RobotKind, start: [f64; 3]) -> Result<(), SimError> {
        if self.agents.contains_key(&id) {
            return Err(SimError::DuplicateAgent(id));
        }
        let position = match kind {
            RobotKind::Ground => [start[0], start[1], 0.0],
            RobotKind::Drone => start,
        };
        self.agents.insert(
            id,
            Agent {
                kind,
                state: AgentState {
                    position,
                    velocity: [0.0; 3],
                    heading: 0.0,
                    collided: false,
                },
                command: Command::idle(kind),
            },
        );
        Ok(())
    }

    fn command(&mut self, id: Uuid, command: Command) -> Result<(), SimError> {
        let agent = self.agents.get_mut(&id).ok_or(SimError::UnknownAgent(id))?;
        let matches = matches!(
            (agent.kind, command),
            (RobotKind::Ground, Command::Ground { .. }) | (RobotKind::Drone, Command::Drone { .. })
        );
        if !matches {
            return Err(SimError::KindMismatch(id));
        }
        agent.command = command;
        Ok(())
    }

    fn step(&mut self, dt: f64) -> Vec<Collision> {
        let obstacles = &self.obstacles;
        let time = self.time + dt;
        let mut collisions = Vec::new();

        for (&id, agent) in self.agents.iter_mut() {
            if agent.state.collided {
                continue;
            }
            advance(agent, dt);
            if touches_obstacle(obstacles, agent.state.position) {
                agent.state.collided = true;
                agent.state.velocity = [0.0; 3];
                collisions.push(Collision {
                    agent: id,
                    time,
                    position: agent.state.position,
                });
            }
        }

        self.time = time;
        collisions
    }

    fn state(&self, id: Uuid) -> Option<AgentState> {
        self.agents.get(&id).map(|agent| agent.state)
    }

    fn agents(&self) -> Vec<Uuid> {
        self.agents.keys().copied().collect()
    }

    fn time(&self) -> f64 {
        self.time
    }
}

/// Integrates one agent forward by `dt` under its current command.
fn advance(agent: &mut Agent, dt: f64) {
    match (agent.kind, agent.command) {
        (RobotKind::Ground, Command::Ground { speed, turn_rate }) => {
            let v = speed.clamp(-GROUND_MAX_SPEED, GROUND_MAX_SPEED);
            let w = turn_rate.clamp(-GROUND_MAX_TURN, GROUND_MAX_TURN);
            let p = agent.state.position;
            // State is [x, y, heading]; the command is held constant over the step.
            let next = rk4_step(
                |_t: f64, s: &[f64; 3]| [v * s[2].cos(), v * s[2].sin(), w],
                0.0,
                [p[0], p[1], agent.state.heading],
                dt,
            );
            agent.state.position = [next[0], next[1], 0.0];
            agent.state.heading = next[2];
            agent.state.velocity = [v * next[2].cos(), v * next[2].sin(), 0.0];
        }
        (RobotKind::Drone, Command::Drone { accel }) => {
            let a = clamp_norm(accel, DRONE_MAX_THRUST_ACCEL);
            let p = agent.state.position;
            let v = agent.state.velocity;
            // State is [position, velocity]; gravity acts on the z velocity.
            let next = rk4_step(
                |_t: f64, s: &[f64; 6]| [s[3], s[4], s[5], a[0], a[1], a[2] - GRAVITY],
                0.0,
                [p[0], p[1], p[2], v[0], v[1], v[2]],
                dt,
            );
            agent.state.position = [next[0], next[1], next[2]];
            agent.state.velocity = [next[3], next[4], next[5]];
            if agent.state.position[2] < 0.0 {
                // The ground stops a landing drone. Touching the ground is not a collision.
                agent.state.position[2] = 0.0;
                agent.state.velocity[2] = 0.0;
            }
            let speed = agent.state.velocity[0].hypot(agent.state.velocity[1]);
            if speed > 1e-9 {
                agent.state.heading = agent.state.velocity[1].atan2(agent.state.velocity[0]);
            }
        }
        // Mismatched kind and command is rejected in `command`, so this is unreachable
        // through the public API. Hold still rather than panic if it happens.
        _ => {}
    }
}

/// Scales `v` down to length `max` if it is longer. Shorter vectors are unchanged.
fn clamp_norm(v: [f64; 3], max: f64) -> [f64; 3] {
    let norm = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if norm > max {
        let scale = max / norm;
        [v[0] * scale, v[1] * scale, v[2] * scale]
    } else {
        v
    }
}

/// True if `p` is within `ROBOT_RADIUS` of any obstacle box.
fn touches_obstacle(obstacles: &[Obstacle], p: [f64; 3]) -> bool {
    obstacles
        .iter()
        .any(|o| (0..3).all(|i| p[i] >= o.min[i] - ROBOT_RADIUS && p[i] <= o.max[i] + ROBOT_RADIUS))
}
