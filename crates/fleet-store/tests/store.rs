use std::path::Path;

use fleet_core::{FleetProject, Robot, RobotKind, SCHEMA_VERSION};
use fleet_store::{Event, StoreError};
use tempfile::TempDir;

#[test]
fn create_then_open_round_trips() {
    let root = TempDir::new().unwrap();
    let dir = root.path().join("lab");
    let mut created = fleet_store::create(&dir, "Lab").unwrap();
    created
        .add_robot(Robot::new("rover", RobotKind::Ground))
        .unwrap();
    fleet_store::save(&dir, &created).unwrap();

    let opened = fleet_store::open(&dir).unwrap();
    assert_eq!(opened, created);
    assert!(
        !dir.join("project.json.tmp").exists(),
        "temp file should be renamed away"
    );
}

#[test]
fn create_refuses_to_overwrite() {
    let root = TempDir::new().unwrap();
    let dir = root.path().join("lab");
    fleet_store::create(&dir, "Lab").unwrap();
    assert!(matches!(
        fleet_store::create(&dir, "Other"),
        Err(StoreError::AlreadyExists(_))
    ));
}

#[test]
fn open_refuses_a_plain_directory() {
    let root = TempDir::new().unwrap();
    assert!(matches!(
        fleet_store::open(root.path()),
        Err(StoreError::NotAProject(_))
    ));
}

#[test]
fn save_refuses_invalid_project_and_keeps_previous_file() {
    let root = TempDir::new().unwrap();
    let dir = root.path().join("lab");
    let mut project = fleet_store::create(&dir, "Lab").unwrap();
    let before = std::fs::read_to_string(dir.join("project.json")).unwrap();

    project.name = "   ".into();
    assert!(matches!(
        fleet_store::save(&dir, &project),
        Err(StoreError::Core(_))
    ));
    assert_eq!(
        std::fs::read_to_string(dir.join("project.json")).unwrap(),
        before
    );
}

#[test]
fn open_refuses_newer_schema_on_disk() {
    let root = TempDir::new().unwrap();
    let dir = root.path().join("lab");
    fleet_store::create(&dir, "Lab").unwrap();
    let path = dir.join("project.json");
    let text = std::fs::read_to_string(&path).unwrap().replace(
        &format!("\"schema_version\": {SCHEMA_VERSION}"),
        &format!("\"schema_version\": {}", SCHEMA_VERSION + 1),
    );
    std::fs::write(&path, text).unwrap();

    assert!(matches!(
        fleet_store::open(&dir),
        Err(StoreError::Core(fleet_core::Error::SchemaTooNew { .. }))
    ));
}

#[test]
fn open_refuses_truncated_file() {
    let root = TempDir::new().unwrap();
    let dir = root.path().join("lab");
    fleet_store::create(&dir, "Lab").unwrap();
    let path = dir.join("project.json");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, &text[..text.len() / 2]).unwrap();

    assert!(fleet_store::open(&dir).is_err());
}

#[test]
fn list_finds_projects_and_skips_other_directories() {
    let root = TempDir::new().unwrap();
    fleet_store::create(&root.path().join("b"), "Beta").unwrap();
    fleet_store::create(&root.path().join("a"), "Alpha").unwrap();
    std::fs::create_dir(root.path().join("not-a-project")).unwrap();

    let found = fleet_store::list(root.path()).unwrap();
    let names: Vec<_> = found.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["Alpha", "Beta"]);
}

#[test]
fn delete_removes_project_and_refuses_unrelated_directories() {
    let root = TempDir::new().unwrap();
    let dir = root.path().join("lab");
    fleet_store::create(&dir, "Lab").unwrap();
    fleet_store::delete(&dir).unwrap();
    assert!(!dir.exists());

    let unrelated = root.path().join("keep");
    std::fs::create_dir(&unrelated).unwrap();
    std::fs::write(unrelated.join("notes.txt"), "keep me").unwrap();
    assert!(matches!(
        fleet_store::delete(&unrelated),
        Err(StoreError::NotAProject(_))
    ));
    assert!(unrelated.join("notes.txt").exists());
}

#[test]
fn events_append_and_read_back_in_order() {
    let root = TempDir::new().unwrap();
    let dir = root.path().join("lab");
    fleet_store::create(&dir, "Lab").unwrap();
    assert!(fleet_store::read_events(&dir).unwrap().is_empty());

    let robot_id = Robot::new("rover", RobotKind::Ground).id;
    let first = Event::now("robot.added", Some(robot_id), "rover");
    let second = Event::now("project.saved", None, "");
    fleet_store::append_event(&dir, &first).unwrap();
    fleet_store::append_event(&dir, &second).unwrap();

    assert_eq!(fleet_store::read_events(&dir).unwrap(), vec![first, second]);
}

#[test]
fn events_refuse_directories_without_a_project() {
    let root = TempDir::new().unwrap();
    let event = Event::now("orphan", None, "");
    assert!(matches!(
        fleet_store::append_event(root.path(), &event),
        Err(StoreError::NotAProject(_))
    ));
}

#[test]
fn a_corrupt_event_line_is_an_error() {
    let root = TempDir::new().unwrap();
    let dir = root.path().join("lab");
    fleet_store::create(&dir, "Lab").unwrap();
    std::fs::write(dir.join("events.jsonl"), "{not json}\n").unwrap();
    assert!(fleet_store::read_events(&dir).is_err());
}

#[test]
fn project_is_loadable_as_a_plain_document() {
    let root = TempDir::new().unwrap();
    let dir = root.path().join("lab");
    fleet_store::create(&dir, "Lab").unwrap();
    let text = std::fs::read_to_string(dir.join("project.json")).unwrap();
    let project = FleetProject::from_json(&text).unwrap();
    assert_eq!(project.name, "Lab");
}

#[test]
fn run_results_save_and_load_inside_the_project() {
    let root = TempDir::new().unwrap();
    let dir = root.path().join("lab");
    fleet_store::create(&dir, "Lab").unwrap();

    let saved = fleet_store::save_run(&dir, "patrol-1", &vec![1.5f64, 2.25]).unwrap();
    assert!(saved.ends_with(Path::new("runs").join("patrol-1.json")));
    let loaded: Vec<f64> = fleet_store::load_run(&dir, "patrol-1").unwrap();
    assert_eq!(loaded, vec![1.5, 2.25]);
}

#[test]
fn run_names_cannot_escape_the_runs_directory() {
    let root = TempDir::new().unwrap();
    let dir = root.path().join("lab");
    fleet_store::create(&dir, "Lab").unwrap();
    for bad in ["", "../escape", "a/b", r"a\b", ".."] {
        assert!(
            fleet_store::save_run(&dir, bad, &1u32).is_err(),
            "accepted {bad:?}"
        );
    }
}
