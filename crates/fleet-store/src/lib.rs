//! File-backed storage for Fleet Projects.
//!
//! Each project lives in its own directory:
//!
//! ```text
//! <project dir>/
//!   project.json   the Fleet Project document (schema-checked on load)
//!   events.jsonl   append-only history, one JSON event per line
//! ```
//!
//! Saves write to a temporary file and rename it into place, so a crash mid-save leaves
//! the previous document intact.
//!
//! The Keystone-backed store from the TODO is not implemented yet. This file-backed store
//! is the storage layer for now.

use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use fleet_core::FleetProject;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Name of the project document inside a project directory.
pub const PROJECT_FILE: &str = "project.json";
/// Name of the append-only event log inside a project directory.
pub const EVENTS_FILE: &str = "events.jsonl";

/// Errors from project storage.
#[derive(Debug)]
pub enum StoreError {
    /// The project document failed to load or validate.
    Core(fleet_core::Error),
    /// The underlying file operation failed.
    Io(io::Error),
    /// The directory does not contain a `project.json`.
    NotAProject(PathBuf),
    /// Creating a project where one already exists.
    AlreadyExists(PathBuf),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::Core(err) => write!(f, "{err}"),
            StoreError::Io(err) => write!(f, "storage error: {err}"),
            StoreError::NotAProject(path) => {
                write!(f, "{} is not a Fleet Project directory", path.display())
            }
            StoreError::AlreadyExists(path) => {
                write!(f, "a Fleet Project already exists at {}", path.display())
            }
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StoreError::Core(err) => Some(err),
            StoreError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<fleet_core::Error> for StoreError {
    fn from(err: fleet_core::Error) -> Self {
        StoreError::Core(err)
    }
}

impl From<io::Error> for StoreError {
    fn from(err: io::Error) -> Self {
        StoreError::Io(err)
    }
}

/// Result type used by `fleet-store`.
pub type Result<T> = std::result::Result<T, StoreError>;

/// One entry in the project history log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    /// Seconds since the Unix epoch.
    pub timestamp: u64,
    /// Robot the event concerns, if any.
    #[serde(default)]
    pub robot_id: Option<Uuid>,
    /// Short machine-readable kind, such as `robot.added`.
    pub kind: String,
    /// Free-form human-readable detail.
    #[serde(default)]
    pub detail: String,
}

impl Event {
    /// An event stamped with the current time.
    pub fn now(kind: &str, robot_id: Option<Uuid>, detail: &str) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            timestamp,
            robot_id,
            kind: kind.to_owned(),
            detail: detail.to_owned(),
        }
    }
}

/// Summary of a project found by [`list`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSummary {
    pub path: PathBuf,
    pub id: Uuid,
    pub name: String,
}

/// Creates a new, empty project in `dir`. The directory is created if needed.
pub fn create(dir: &Path, name: &str) -> Result<FleetProject> {
    if dir.join(PROJECT_FILE).exists() {
        return Err(StoreError::AlreadyExists(dir.to_path_buf()));
    }
    let project = FleetProject::new(name)?;
    fs::create_dir_all(dir)?;
    save(dir, &project)?;
    Ok(project)
}

/// Loads and validates the project in `dir`.
pub fn open(dir: &Path) -> Result<FleetProject> {
    let path = dir.join(PROJECT_FILE);
    if !path.is_file() {
        return Err(StoreError::NotAProject(dir.to_path_buf()));
    }
    let text = fs::read_to_string(&path)?;
    Ok(FleetProject::from_json(&text)?)
}

/// Validates and writes the project to `dir` atomically.
pub fn save(dir: &Path, project: &FleetProject) -> Result<()> {
    let text = project.to_json()?;
    fs::create_dir_all(dir)?;
    let target = dir.join(PROJECT_FILE);
    let temp = dir.join(format!("{PROJECT_FILE}.tmp"));
    {
        let mut file = File::create(&temp)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
    }
    fs::rename(&temp, &target)?;
    Ok(())
}

/// Lists the projects found directly inside `root`, sorted by name.
///
/// Subdirectories without a `project.json` are ignored. A project that fails to load is an error.
pub fn list(root: &Path) -> Result<Vec<ProjectSummary>> {
    let mut summaries = Vec::new();
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if !path.join(PROJECT_FILE).is_file() {
            continue;
        }
        let project = open(&path)?;
        summaries.push(ProjectSummary {
            path,
            id: project.id,
            name: project.name,
        });
    }
    summaries.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.path.cmp(&b.path)));
    Ok(summaries)
}

/// Deletes the project in `dir` and its event log.
///
/// Refuses to act unless `dir` contains a `project.json`, so pointing it at an unrelated
/// directory cannot remove it.
pub fn delete(dir: &Path) -> Result<()> {
    if !dir.join(PROJECT_FILE).is_file() {
        return Err(StoreError::NotAProject(dir.to_path_buf()));
    }
    fs::remove_dir_all(dir)?;
    Ok(())
}

/// Appends one event to the project history.
pub fn append_event(dir: &Path, event: &Event) -> Result<()> {
    if !dir.join(PROJECT_FILE).is_file() {
        return Err(StoreError::NotAProject(dir.to_path_buf()));
    }
    let mut line = serde_json::to_string(event).map_err(io::Error::from)?;
    line.push('\n');
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(EVENTS_FILE))?;
    file.write_all(line.as_bytes())?;
    file.sync_data()?;
    Ok(())
}

/// Name of the directory inside a project that holds saved run results.
pub const RUNS_DIR: &str = "runs";

/// Saves a serialisable run result as `runs/<name>.json` inside the project in `dir`.
///
/// `name` must be a plain file name: no path separators and not empty.
pub fn save_run<T: Serialize>(dir: &Path, name: &str, run: &T) -> Result<PathBuf> {
    if !dir.join(PROJECT_FILE).is_file() {
        return Err(StoreError::NotAProject(dir.to_path_buf()));
    }
    let valid_name = !name.is_empty() && !name.contains(['/', '\\']) && name != "." && name != "..";
    if !valid_name {
        return Err(StoreError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("run name '{name}' must be a plain file name"),
        )));
    }
    let runs = dir.join(RUNS_DIR);
    fs::create_dir_all(&runs)?;
    let path = runs.join(format!("{name}.json"));
    let text = serde_json::to_string_pretty(run).map_err(io::Error::from)?;
    fs::write(&path, text)?;
    Ok(path)
}

/// Loads a run result saved by [`save_run`].
pub fn load_run<T: DeserializeOwned>(dir: &Path, name: &str) -> Result<T> {
    let path = dir.join(RUNS_DIR).join(format!("{name}.json"));
    let text = fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&text).map_err(io::Error::from)?)
}

/// Reads the full history in the order it was written. A missing log is an empty history.
pub fn read_events(dir: &Path) -> Result<Vec<Event>> {
    let path = dir.join(EVENTS_FILE);
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let reader = BufReader::new(File::open(path)?);
    let mut events = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        events.push(serde_json::from_str(&line).map_err(io::Error::from)?);
    }
    Ok(events)
}
