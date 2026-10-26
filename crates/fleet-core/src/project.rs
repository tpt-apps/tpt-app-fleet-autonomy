use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::error::{require_finite, require_text};
use crate::robot::{Pose, Robot};
use crate::{Error, Result};

/// Current on-disk schema version. Documents with a higher version are refused.
pub const SCHEMA_VERSION: u32 = 1;

/// A named, versioned skill. Deployment is refused if a robot lacks `required_capabilities`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Skill {
    pub id: Uuid,
    pub name: String,
    pub version: u32,
    #[serde(default)]
    pub required_capabilities: Vec<String>,
}

/// A simulation world. Scenario detail (obstacles, goals) is added in Phase 3.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct World {
    pub id: Uuid,
    pub name: String,
    pub duration_s: f64,
}

/// Sends one skill to a set of robots.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Deployment {
    pub id: Uuid,
    pub skill_id: Uuid,
    pub robot_ids: Vec<Uuid>,
}

/// Last known state of a robot, as reported by simulation or an adapter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObservedState {
    pub robot_id: Uuid,
    pub online: bool,
    #[serde(default)]
    pub pose: Option<Pose>,
}

/// A complete Fleet Project. This is the unit that is saved, exported and imported.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FleetProject {
    pub schema_version: u32,
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub robots: Vec<Robot>,
    #[serde(default)]
    pub skills: Vec<Skill>,
    #[serde(default)]
    pub worlds: Vec<World>,
    #[serde(default)]
    pub deployments: Vec<Deployment>,
    #[serde(default)]
    pub observed: Vec<ObservedState>,
}

impl FleetProject {
    /// An empty project with a fresh id.
    pub fn new(name: &str) -> Result<Self> {
        require_text(name, "project name")?;
        Ok(Self {
            schema_version: SCHEMA_VERSION,
            id: Uuid::new_v4(),
            name: name.to_owned(),
            robots: Vec::new(),
            skills: Vec::new(),
            worlds: Vec::new(),
            deployments: Vec::new(),
            observed: Vec::new(),
        })
    }

    /// Looks up a robot by id.
    pub fn robot(&self, id: Uuid) -> Option<&Robot> {
        self.robots.iter().find(|robot| robot.id == id)
    }

    /// Adds a robot after validating it. Ids and names must be unique within the project.
    pub fn add_robot(&mut self, robot: Robot) -> Result<()> {
        robot.validate()?;
        if self.robot(robot.id).is_some() {
            return Err(Error::Invalid(format!(
                "robot id {} already exists",
                robot.id
            )));
        }
        if self
            .robots
            .iter()
            .any(|existing| existing.name == robot.name)
        {
            return Err(Error::Invalid(format!(
                "robot name '{}' already exists",
                robot.name
            )));
        }
        self.robots.push(robot);
        Ok(())
    }

    /// Removes a robot and every reference to it in deployments and observed state.
    pub fn remove_robot(&mut self, id: Uuid) -> Result<Robot> {
        let index = self
            .robots
            .iter()
            .position(|robot| robot.id == id)
            .ok_or_else(|| Error::Invalid(format!("no robot with id {id}")))?;
        let removed = self.robots.remove(index);
        for deployment in &mut self.deployments {
            deployment.robot_ids.retain(|robot_id| *robot_id != id);
        }
        self.observed.retain(|state| state.robot_id != id);
        Ok(removed)
    }

    /// Checks the whole project, including cross-references between sections.
    pub fn validate(&self) -> Result<()> {
        require_text(&self.name, "project name")?;

        let mut robot_ids = HashSet::new();
        let mut robot_names = HashSet::new();
        for robot in &self.robots {
            robot.validate()?;
            if !robot_ids.insert(robot.id) {
                return Err(Error::Invalid(format!("duplicate robot id {}", robot.id)));
            }
            if !robot_names.insert(robot.name.as_str()) {
                return Err(Error::Invalid(format!(
                    "duplicate robot name '{}'",
                    robot.name
                )));
            }
        }

        let mut skill_ids = HashSet::new();
        for skill in &self.skills {
            require_text(&skill.name, "skill name")?;
            if !skill_ids.insert(skill.id) {
                return Err(Error::Invalid(format!("duplicate skill id {}", skill.id)));
            }
            if skill.version == 0 {
                return Err(Error::Invalid(format!(
                    "skill '{}' version must be at least 1",
                    skill.name
                )));
            }
            for capability in &skill.required_capabilities {
                require_text(capability, &format!("skill '{}' capability", skill.name))?;
            }
        }

        let mut world_ids = HashSet::new();
        for world in &self.worlds {
            require_text(&world.name, "world name")?;
            if !world_ids.insert(world.id) {
                return Err(Error::Invalid(format!("duplicate world id {}", world.id)));
            }
            require_finite(
                world.duration_s,
                &format!("world '{}' duration_s", world.name),
            )?;
            if world.duration_s <= 0.0 {
                return Err(Error::Invalid(format!(
                    "world '{}' duration_s must be greater than zero",
                    world.name
                )));
            }
        }

        let mut deployment_ids = HashSet::new();
        for deployment in &self.deployments {
            if !deployment_ids.insert(deployment.id) {
                return Err(Error::Invalid(format!(
                    "duplicate deployment id {}",
                    deployment.id
                )));
            }
            if !skill_ids.contains(&deployment.skill_id) {
                return Err(Error::Invalid(format!(
                    "deployment {} references unknown skill {}",
                    deployment.id, deployment.skill_id
                )));
            }
            for robot_id in &deployment.robot_ids {
                if !robot_ids.contains(robot_id) {
                    return Err(Error::Invalid(format!(
                        "deployment {} references unknown robot {robot_id}",
                        deployment.id
                    )));
                }
            }
        }

        let mut observed_ids = HashSet::new();
        for state in &self.observed {
            if !robot_ids.contains(&state.robot_id) {
                return Err(Error::Invalid(format!(
                    "observed state references unknown robot {}",
                    state.robot_id
                )));
            }
            if !observed_ids.insert(state.robot_id) {
                return Err(Error::Invalid(format!(
                    "duplicate observed state for robot {}",
                    state.robot_id
                )));
            }
            if let Some(pose) = &state.pose {
                pose.validate(&format!("observed pose of robot {}", state.robot_id))?;
            }
        }

        Ok(())
    }

    /// Validates, then serialises to pretty-printed JSON.
    pub fn to_json(&self) -> Result<String> {
        self.validate()?;
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Parses and validates a document.
    ///
    /// The schema version is checked before the full parse, so a newer document is refused
    /// with `SchemaTooNew` rather than failing on an unknown field.
    pub fn from_json(text: &str) -> Result<Self> {
        let mut value: Value = serde_json::from_str(text)?;
        let found = value
            .get("schema_version")
            .and_then(Value::as_u64)
            .ok_or_else(|| Error::Invalid("missing or non-numeric schema_version".to_owned()))?;
        let found = u32::try_from(found)
            .map_err(|_| Error::Invalid(format!("schema_version {found} is out of range")))?;

        if found == 0 {
            return Err(Error::Invalid(
                "schema_version must be at least 1".to_owned(),
            ));
        }
        if found > SCHEMA_VERSION {
            return Err(Error::SchemaTooNew {
                found,
                supported: SCHEMA_VERSION,
            });
        }
        for version in found..SCHEMA_VERSION {
            value = migrate(value, version)?;
        }

        let project: FleetProject = serde_json::from_value(value)?;
        project.validate()?;
        Ok(project)
    }
}

/// Upgrades a document from `from` to `from + 1`.
///
/// There is only v1 so far, so there is nothing to upgrade. The first real migration adds its
/// step here and bumps `SCHEMA_VERSION`.
fn migrate(_value: Value, from: u32) -> Result<Value> {
    Err(Error::Invalid(format!(
        "no migration from schema version {from}"
    )))
}
