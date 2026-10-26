use fleet_core::RobotKind;
use fleet_sim::{
    lidar_scan, run, AgentSpec, BuiltinBackend, Command, LidarConfig, Obstacle, Rng, RunResult,
    Scenario, SimBackend, SimError, GOAL_TOLERANCE,
};
use uuid::Uuid;

fn scenario(agents: Vec<AgentSpec>, obstacles: Vec<Obstacle>) -> Scenario {
    Scenario {
        name: "test".into(),
        obstacles,
        agents,
        duration_s: 30.0,
        dt: 0.02,
        seed: 7,
        sensor_noise: 0.0,
    }
}

fn wall_at_x(x: f64) -> Obstacle {
    Obstacle {
        min: [x, -1.0, 0.0],
        max: [x + 1.0, 1.0, 1.0],
    }
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>().sqrt()
}

#[test]
fn ground_robot_drives_straight_at_commanded_speed() {
    let id = Uuid::new_v4();
    let mut sim = BuiltinBackend::new(vec![]);
    sim.spawn(id, RobotKind::Ground, [0.0, 0.0, 0.0]).unwrap();
    sim.command(
        id,
        Command::Ground {
            speed: 1.0,
            turn_rate: 0.0,
        },
    )
    .unwrap();
    for _ in 0..100 {
        sim.step(0.02);
    }
    let state = sim.state(id).unwrap();
    assert!(
        (state.position[0] - 2.0).abs() < 1e-9,
        "x = {}",
        state.position[0]
    );
    assert!(state.position[1].abs() < 1e-9);
    assert!((sim.time() - 2.0).abs() < 1e-9);
}

#[test]
fn drone_hovers_under_gravity_compensation() {
    let id = Uuid::new_v4();
    let mut sim = BuiltinBackend::new(vec![]);
    sim.spawn(id, RobotKind::Drone, [0.0, 0.0, 5.0]).unwrap();
    sim.command(id, Command::idle(RobotKind::Drone)).unwrap();
    for _ in 0..200 {
        sim.step(0.02);
    }
    let state = sim.state(id).unwrap();
    assert!(
        (state.position[2] - 5.0).abs() < 1e-9,
        "z = {}",
        state.position[2]
    );
}

#[test]
fn ground_robot_stops_at_an_obstacle() {
    let id = Uuid::new_v4();
    let mut sim = BuiltinBackend::new(vec![wall_at_x(3.0)]);
    sim.spawn(id, RobotKind::Ground, [0.0, 0.0, 0.0]).unwrap();
    sim.command(
        id,
        Command::Ground {
            speed: 1.0,
            turn_rate: 0.0,
        },
    )
    .unwrap();

    let mut collisions = Vec::new();
    for _ in 0..1000 {
        collisions.extend(sim.step(0.02));
    }
    let state = sim.state(id).unwrap();
    assert!(state.collided);
    assert_eq!(collisions.len(), 1, "a collided agent is reported once");
    assert_eq!(collisions[0].agent, id);
    // The robot stops about one radius short of the wall face at x = 3.
    assert!(state.position[0] < 3.0 - fleet_sim::ROBOT_RADIUS + 0.05);
    assert!(state.position[0] > 3.0 - fleet_sim::ROBOT_RADIUS - 0.05);
}

#[test]
fn spawn_and_command_reject_bad_input() {
    let id = Uuid::new_v4();
    let mut sim = BuiltinBackend::new(vec![]);
    sim.spawn(id, RobotKind::Ground, [0.0; 3]).unwrap();
    assert_eq!(
        sim.spawn(id, RobotKind::Ground, [0.0; 3]),
        Err(SimError::DuplicateAgent(id))
    );
    assert_eq!(
        sim.command(id, Command::Drone { accel: [0.0; 3] }),
        Err(SimError::KindMismatch(id))
    );
    let stranger = Uuid::new_v4();
    assert_eq!(
        sim.command(
            stranger,
            Command::Ground {
                speed: 0.0,
                turn_rate: 0.0
            }
        ),
        Err(SimError::UnknownAgent(stranger))
    );
}

#[test]
fn ground_robot_reaches_its_goal() {
    let id = Uuid::new_v4();
    let goal = [5.0, 3.0, 0.0];
    let s = scenario(
        vec![AgentSpec {
            id,
            kind: RobotKind::Ground,
            start: [0.0, 0.0, 0.0],
            goal: Some(goal),
        }],
        vec![],
    );
    let result = run(&s, &mut BuiltinBackend::new(vec![])).unwrap();
    assert!(result.reached.contains_key(&id), "never reached goal");
    let last = result.samples.iter().rev().find(|x| x.agent == id).unwrap();
    assert!(dist(last.position, goal) <= GOAL_TOLERANCE + 0.05);
}

#[test]
fn drone_converges_to_its_goal() {
    let id = Uuid::new_v4();
    let goal = [5.0, 5.0, 3.0];
    let s = scenario(
        vec![AgentSpec {
            id,
            kind: RobotKind::Drone,
            start: [0.0, 0.0, 0.0],
            goal: Some(goal),
        }],
        vec![],
    );
    let result = run(&s, &mut BuiltinBackend::new(vec![])).unwrap();
    assert!(result.reached.contains_key(&id), "never reached goal");
    let last = result.samples.iter().rev().find(|x| x.agent == id).unwrap();
    assert!(
        dist(last.position, goal) < 0.1,
        "final error {}",
        dist(last.position, goal)
    );
}

#[test]
fn a_seeded_run_is_reproducible() {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let mut s = scenario(
        vec![
            AgentSpec {
                id: a,
                kind: RobotKind::Drone,
                start: [0.0, 0.0, 0.0],
                goal: Some([3.0, 0.0, 2.0]),
            },
            AgentSpec {
                id: b,
                kind: RobotKind::Ground,
                start: [0.0, 2.0, 0.0],
                goal: Some([4.0, 2.0, 0.0]),
            },
        ],
        vec![wall_at_x(10.0)],
    );
    s.sensor_noise = 0.05;
    let first = run(&s, &mut BuiltinBackend::new(s.obstacles.clone())).unwrap();
    let second = run(&s, &mut BuiltinBackend::new(s.obstacles.clone())).unwrap();
    assert_eq!(first, second);
}

#[test]
fn a_different_seed_changes_observations_but_not_true_motion() {
    let id = Uuid::new_v4();
    let mut s = scenario(
        vec![AgentSpec {
            id,
            kind: RobotKind::Drone,
            start: [0.0, 0.0, 0.0],
            goal: Some([2.0, 0.0, 2.0]),
        }],
        vec![],
    );
    s.sensor_noise = 0.1;
    let a = run(&s, &mut BuiltinBackend::new(vec![])).unwrap();
    s.seed = 8;
    let b = run(&s, &mut BuiltinBackend::new(vec![])).unwrap();
    assert_eq!(a.samples.len(), b.samples.len());
    for (x, y) in a.samples.iter().zip(&b.samples) {
        assert_eq!(
            x.position, y.position,
            "true motion must not depend on the seed"
        );
    }
    assert_ne!(a.samples[10].observed, b.samples[10].observed);
}

#[test]
fn a_recorded_run_replays_from_its_serialised_scenario() {
    let id = Uuid::new_v4();
    let s = scenario(
        vec![AgentSpec {
            id,
            kind: RobotKind::Ground,
            start: [0.0, 0.0, 0.0],
            goal: Some([3.0, 1.0, 0.0]),
        }],
        vec![wall_at_x(8.0)],
    );
    let recorded = run(&s, &mut BuiltinBackend::new(s.obstacles.clone())).unwrap();

    let text = serde_json::to_string(&s).unwrap();
    let restored: Scenario = serde_json::from_str(&text).unwrap();
    let replayed = run(
        &restored,
        &mut BuiltinBackend::new(restored.obstacles.clone()),
    )
    .unwrap();
    assert_eq!(recorded, replayed);

    let result_text = serde_json::to_string(&recorded).unwrap();
    let result_back: RunResult = serde_json::from_str(&result_text).unwrap();
    assert_eq!(result_back, recorded);
}

#[test]
fn invalid_scenarios_are_rejected() {
    let id = Uuid::new_v4();
    let base = scenario(
        vec![AgentSpec {
            id,
            kind: RobotKind::Ground,
            start: [0.0; 3],
            goal: None,
        }],
        vec![],
    );

    let mut bad_dt = base.clone();
    bad_dt.dt = 0.0;
    assert!(matches!(bad_dt.validate(), Err(SimError::Invalid(_))));

    let mut too_coarse = base.clone();
    too_coarse.dt = 1.0;
    assert!(too_coarse.validate().is_err());

    let mut nan_start = base.clone();
    nan_start.agents[0].start[0] = f64::NAN;
    assert!(nan_start.validate().is_err());

    let mut inverted_box = base.clone();
    inverted_box.obstacles.push(Obstacle {
        min: [1.0, 0.0, 0.0],
        max: [0.0, 1.0, 1.0],
    });
    assert!(inverted_box.validate().is_err());

    let mut duplicate = base.clone();
    duplicate.agents.push(duplicate.agents[0]);
    assert_eq!(duplicate.validate(), Err(SimError::DuplicateAgent(id)));
}

#[test]
fn lidar_reports_distance_to_the_first_obstacle() {
    let obstacle = Obstacle {
        min: [2.0, -1.0, -1.0],
        max: [3.0, 1.0, 1.0],
    };
    let config = LidarConfig {
        rays: 4,
        max_range: 10.0,
        noise: 0.0,
    };
    let scan = lidar_scan(&[obstacle], [0.0; 3], 0.0, &config, &mut Rng::new(1));
    assert!((scan[0] - 2.0).abs() < 1e-9, "forward ray {}", scan[0]);
    assert_eq!(scan[1], 10.0, "left ray misses");
    assert_eq!(scan[2], 10.0, "backward ray misses");
    assert_eq!(scan[3], 10.0, "right ray misses");
}

#[test]
fn lidar_ignores_obstacles_beyond_range() {
    let obstacle = Obstacle {
        min: [20.0, -1.0, -1.0],
        max: [21.0, 1.0, 1.0],
    };
    let config = LidarConfig {
        rays: 1,
        max_range: 5.0,
        noise: 0.0,
    };
    let scan = lidar_scan(&[obstacle], [0.0; 3], 0.0, &config, &mut Rng::new(1));
    assert_eq!(scan, vec![5.0]);
}

#[test]
fn rng_is_deterministic_and_in_range() {
    let mut a = Rng::new(42);
    let mut b = Rng::new(42);
    for _ in 0..1000 {
        let x = a.next_unit();
        assert_eq!(x, b.next_unit());
        assert!((0.0..1.0).contains(&x));
    }
}
