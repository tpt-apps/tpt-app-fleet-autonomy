use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{require_finite, require_text};
use crate::{Error, Result};

/// Whether a robot moves on the ground or flies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RobotKind {
    Ground,
    Drone,
}

/// Position in metres. Every field must be finite.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Pose {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Pose {
    pub fn validate(&self, what: &str) -> Result<()> {
        require_finite(self.x, &format!("{what}.x"))?;
        require_finite(self.y, &format!("{what}.y"))?;
        require_finite(self.z, &format!("{what}.z"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensorKind {
    Pose,
    Lidar,
    PointCloud,
    Camera,
    Telemetry,
}

/// A sensor fitted to a robot. `rate_hz` is the sampling rate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sensor {
    pub name: String,
    pub kind: SensorKind,
    pub rate_hz: f64,
}

/// An actuator fitted to a robot, such as a drive motor, rotor or gripper.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Actuator {
    pub name: String,
    pub kind: String,
}

/// How the application talks to the robot. Only `Simulated` needs no address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Adapter {
    Simulated,
    TcpUdp { address: String },
    Mavlink { address: String },
}

/// A robot or drone definition inside a Fleet Project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Robot {
    pub id: Uuid,
    pub name: String,
    pub kind: RobotKind,
    #[serde(default)]
    pub sensors: Vec<Sensor>,
    #[serde(default)]
    pub actuators: Vec<Actuator>,
    pub adapter: Adapter,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub home: Pose,
}

impl Robot {
    /// A robot with a fresh id, no sensors or actuators, and a simulated adapter.
    pub fn new(name: &str, kind: RobotKind) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_owned(),
            kind,
            sensors: Vec::new(),
            actuators: Vec::new(),
            adapter: Adapter::Simulated,
            capabilities: Vec::new(),
            home: Pose::default(),
        }
    }

    /// Checks every field. Returns the first problem found.
    pub fn validate(&self) -> Result<()> {
        require_text(&self.name, "robot name")?;
        let context = format!("robot '{}'", self.name);

        let mut sensor_names = HashSet::new();
        for sensor in &self.sensors {
            require_text(&sensor.name, &format!("{context} sensor name"))?;
            if !sensor_names.insert(sensor.name.as_str()) {
                return Err(Error::Invalid(format!(
                    "{context} has duplicate sensor '{}'",
                    sensor.name
                )));
            }
            require_finite(
                sensor.rate_hz,
                &format!("{context} sensor '{}' rate_hz", sensor.name),
            )?;
            if sensor.rate_hz <= 0.0 {
                return Err(Error::Invalid(format!(
                    "{context} sensor '{}' rate_hz must be greater than zero",
                    sensor.name
                )));
            }
        }

        let mut actuator_names = HashSet::new();
        for actuator in &self.actuators {
            require_text(&actuator.name, &format!("{context} actuator name"))?;
            if !actuator_names.insert(actuator.name.as_str()) {
                return Err(Error::Invalid(format!(
                    "{context} has duplicate actuator '{}'",
                    actuator.name
                )));
            }
        }

        match &self.adapter {
            Adapter::Simulated => {}
            Adapter::TcpUdp { address } | Adapter::Mavlink { address } => {
                require_text(address, &format!("{context} adapter address"))?;
            }
        }

        for capability in &self.capabilities {
            require_text(capability, &format!("{context} capability"))?;
        }

        self.home.validate(&format!("{context} home"))
    }
}
