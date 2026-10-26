use std::collections::BTreeMap;

use fleet_core::RobotKind;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::backend::{Collision, SimBackend, SimError};
use crate::control::{drone_command, ground_command, GOAL_TOLERANCE};
use crate::rng::Rng;
use crate::scenario::Scenario;

/// One agent's position at one step. `observed` includes sensor noise.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Sample {
    pub time: f64,
    pub agent: Uuid,
    pub position: [f64; 3],
    pub observed: [f64; 3],
}

/// The recorded outcome of a run. Two runs of the same scenario compare equal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunResult {
    pub scenario: String,
    pub seed: u64,
    pub samples: Vec<Sample>,
    pub collisions: Vec<Collision>,
    /// Time each agent first came within `GOAL_TOLERANCE` of its goal.
    pub reached: BTreeMap<Uuid, f64>,
}

/// Runs `scenario` on `backend` and records the trace.
///
/// The backend must be empty. Agents are spawned from the scenario, and each step's command
/// comes from the built-in controllers. Agents without a goal are left idle.
pub fn run<B: SimBackend>(scenario: &Scenario, backend: &mut B) -> Result<RunResult, SimError> {
    scenario.validate()?;

    let mut kinds = BTreeMap::new();
    for spec in &scenario.agents {
        backend.spawn(spec.id, spec.kind, spec.start)?;
        kinds.insert(spec.id, spec.kind);
    }

    let goals: BTreeMap<Uuid, [f64; 3]> = scenario
        .agents
        .iter()
        .filter_map(|spec| spec.goal.map(|goal| (spec.id, goal)))
        .collect();

    let mut rng = Rng::new(scenario.seed);
    let mut samples = Vec::new();
    let mut collisions = Vec::new();
    let mut reached = BTreeMap::new();
    let steps = (scenario.duration_s / scenario.dt).round() as usize;

    for _ in 0..steps {
        for (&id, &goal) in &goals {
            let Some(state) = backend.state(id) else {
                continue;
            };
            let command = match kinds[&id] {
                RobotKind::Ground => ground_command(state.position, state.heading, goal),
                RobotKind::Drone => drone_command(state.position, state.velocity, goal),
            };
            backend.command(id, command)?;
        }

        collisions.extend(backend.step(scenario.dt));
        let time = backend.time();

        for id in backend.agents() {
            let Some(state) = backend.state(id) else {
                continue;
            };
            let observed =
                [0, 1, 2].map(|i| state.position[i] + rng.uniform(scenario.sensor_noise));
            samples.push(Sample {
                time,
                agent: id,
                position: state.position,
                observed,
            });
            if let Some(goal) = goals.get(&id) {
                let distance = (0..3)
                    .map(|i| (goal[i] - state.position[i]).powi(2))
                    .sum::<f64>()
                    .sqrt();
                if distance <= GOAL_TOLERANCE {
                    reached.entry(id).or_insert(time);
                }
            }
        }
    }

    Ok(RunResult {
        scenario: scenario.name.clone(),
        seed: scenario.seed,
        samples,
        collisions,
        reached,
    })
}
