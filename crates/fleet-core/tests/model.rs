use fleet_core::{
    Adapter, Deployment, Error, FleetProject, ObservedState, Pose, Robot, RobotKind, Sensor,
    SensorKind, Skill, SCHEMA_VERSION,
};
use uuid::Uuid;

fn sample_project() -> FleetProject {
    let mut project = FleetProject::new("Lab fleet").expect("valid name");
    let mut scout = Robot::new("scout", RobotKind::Drone);
    scout.sensors.push(Sensor {
        name: "lidar".into(),
        kind: SensorKind::Lidar,
        rate_hz: 10.0,
    });
    scout.adapter = Adapter::Mavlink {
        address: "udp://192.168.1.20:14550".into(),
    };
    scout.capabilities = vec!["move".into(), "sensor:read:lidar".into()];
    let scout_id = scout.id;
    project.add_robot(scout).expect("add scout");

    let skill = Skill {
        id: Uuid::new_v4(),
        name: "patrol".into(),
        version: 2,
        required_capabilities: vec!["move".into()],
    };
    project.deployments.push(Deployment {
        id: Uuid::new_v4(),
        skill_id: skill.id,
        robot_ids: vec![scout_id],
    });
    project.skills.push(skill);
    project.observed.push(ObservedState {
        robot_id: scout_id,
        online: true,
        pose: Some(Pose {
            x: 1.0,
            y: 2.0,
            z: 0.5,
        }),
    });
    project
}

#[test]
fn round_trip_preserves_project() {
    let project = sample_project();
    let text = project.to_json().expect("serialises");
    let parsed = FleetProject::from_json(&text).expect("parses");
    assert_eq!(parsed, project);
}

#[test]
fn new_project_is_at_current_schema() {
    let project = FleetProject::new("p").unwrap();
    assert_eq!(project.schema_version, SCHEMA_VERSION);
}

#[test]
fn empty_names_are_rejected() {
    assert!(matches!(FleetProject::new("   "), Err(Error::Invalid(_))));
    assert!(matches!(
        Robot::new("", RobotKind::Ground).validate(),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn nan_pose_is_rejected() {
    let mut robot = Robot::new("rover", RobotKind::Ground);
    robot.home = Pose {
        x: f64::NAN,
        y: 0.0,
        z: 0.0,
    };
    let err = robot.validate().unwrap_err();
    assert!(err.to_string().contains("finite"), "{err}");
}

#[test]
fn infinite_sensor_rate_is_rejected() {
    let mut robot = Robot::new("rover", RobotKind::Ground);
    robot.sensors.push(Sensor {
        name: "imu".into(),
        kind: SensorKind::Telemetry,
        rate_hz: f64::INFINITY,
    });
    assert!(robot.validate().is_err());
}

#[test]
fn zero_sensor_rate_is_rejected() {
    let mut robot = Robot::new("rover", RobotKind::Ground);
    robot.sensors.push(Sensor {
        name: "imu".into(),
        kind: SensorKind::Telemetry,
        rate_hz: 0.0,
    });
    assert!(robot.validate().is_err());
}

#[test]
fn duplicate_robot_names_are_rejected() {
    let mut project = FleetProject::new("p").unwrap();
    project
        .add_robot(Robot::new("twin", RobotKind::Ground))
        .unwrap();
    let err = project
        .add_robot(Robot::new("twin", RobotKind::Drone))
        .unwrap_err();
    assert!(err.to_string().contains("already exists"));
}

#[test]
fn network_adapter_needs_an_address() {
    let mut robot = Robot::new("rover", RobotKind::Ground);
    robot.adapter = Adapter::TcpUdp {
        address: " ".into(),
    };
    assert!(robot.validate().is_err());
}

#[test]
fn dangling_deployment_reference_is_rejected() {
    let mut project = sample_project();
    project.deployments[0].robot_ids.push(Uuid::new_v4());
    let err = project.validate().unwrap_err();
    assert!(err.to_string().contains("unknown robot"), "{err}");
}

#[test]
fn removing_a_robot_clears_its_references() {
    let mut project = sample_project();
    let id = project.robots[0].id;
    project.remove_robot(id).unwrap();
    assert!(project.deployments[0].robot_ids.is_empty());
    assert!(project.observed.is_empty());
    assert!(project.validate().is_ok());
}

#[test]
fn newer_schema_is_refused() {
    let text = sample_project().to_json().unwrap().replace(
        &format!("\"schema_version\": {SCHEMA_VERSION}"),
        &format!("\"schema_version\": {}", SCHEMA_VERSION + 1),
    );
    match FleetProject::from_json(&text) {
        Err(Error::SchemaTooNew { found, supported }) => {
            assert_eq!(found, SCHEMA_VERSION + 1);
            assert_eq!(supported, SCHEMA_VERSION);
        }
        other => panic!("expected SchemaTooNew, got {other:?}"),
    }
}

#[test]
fn missing_schema_version_is_refused() {
    let err =
        FleetProject::from_json(r#"{"id":"00000000-0000-0000-0000-000000000000","name":"x"}"#)
            .unwrap_err();
    assert!(err.to_string().contains("schema_version"));
}

#[test]
fn truncated_document_is_refused() {
    let text = sample_project().to_json().unwrap();
    assert!(FleetProject::from_json(&text[..text.len() / 2]).is_err());
}

#[test]
fn invalid_content_in_a_valid_json_document_is_refused() {
    let text = sample_project()
        .to_json()
        .unwrap()
        .replace("\"lidar\"", "\"\"");
    assert!(FleetProject::from_json(&text).is_err());
}
