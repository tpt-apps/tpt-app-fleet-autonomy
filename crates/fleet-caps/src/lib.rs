//! Capability-based permissions.
//!
//! A [`Capability`] is a colon-separated path such as `move`, `actuate:gripper` or
//! `sensor:read:lidar`. A grant covers a capability when the grant is the same path or a
//! prefix of it, so `sensor:read` covers `sensor:read:lidar` but not `sensor:write`.
//!
//! Everything is denied by default. Every decision made through [`Policy::authorize`] or
//! [`Policy::grant_as`] is recorded in the audit log, allowed or denied.
//!
//! The policy is in memory. Persisting the audit log (to Keystone or to a project's event
//! log) is the caller's job.

use std::collections::{BTreeSet, HashMap};
use std::fmt;

use uuid::Uuid;

/// A permission path such as `actuate:gripper`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Capability {
    segments: Vec<String>,
}

/// Why a capability string could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid capability: {}", self.0)
    }
}

impl std::error::Error for ParseError {}

impl Capability {
    /// Parses `verb[:qualifier...]`. Segments must be non-empty and contain no whitespace.
    pub fn parse(text: &str) -> Result<Self, ParseError> {
        if text.is_empty() {
            return Err(ParseError("empty string".to_owned()));
        }
        let mut segments = Vec::new();
        for segment in text.split(':') {
            if segment.is_empty() {
                return Err(ParseError(format!("'{text}' has an empty segment")));
            }
            if segment.chars().any(char::is_whitespace) {
                return Err(ParseError(format!("'{text}' contains whitespace")));
            }
            segments.push(segment.to_owned());
        }
        Ok(Self { segments })
    }

    /// True if `self` is `other` or a broader capability that includes it.
    ///
    /// `sensor:read` covers `sensor:read:lidar`. `sensor:read:lidar` does not cover `sensor:read`.
    pub fn covers(&self, other: &Capability) -> bool {
        self.segments.len() <= other.segments.len() && other.segments.starts_with(&self.segments)
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.segments.join(":"))
    }
}

/// Result of an authorisation check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Allowed,
    Denied { reason: String },
}

impl Decision {
    pub fn is_allowed(&self) -> bool {
        matches!(self, Decision::Allowed)
    }
}

/// One audit log entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEntry {
    /// The robot or agent that asked.
    pub subject: Uuid,
    /// What was asked for, as text.
    pub action: String,
    /// The outcome.
    pub decision: Decision,
}

/// Per-subject grants plus an audit log of every decision.
#[derive(Debug, Default)]
pub struct Policy {
    grants: HashMap<Uuid, BTreeSet<Capability>>,
    audit: Vec<AuditEntry>,
}

impl Policy {
    pub fn new() -> Self {
        Self::default()
    }

    /// Grants `capability` to `subject` without any check. For setting up a project from a
    /// trusted source. Use [`Policy::grant_as`] for grants made by another subject.
    pub fn grant(&mut self, subject: Uuid, capability: Capability) {
        self.grants.entry(subject).or_default().insert(capability);
    }

    /// Removes an exact grant. Returns `true` if it was present. Broader or narrower grants
    /// are left alone, so revoking `sensor:read` does not remove `sensor:read:lidar`.
    pub fn revoke(&mut self, subject: Uuid, capability: &Capability) -> bool {
        let Some(set) = self.grants.get_mut(&subject) else {
            return false;
        };
        let removed = set.remove(capability);
        if set.is_empty() {
            self.grants.remove(&subject);
        }
        removed
    }

    /// Grants `capability` to `target`, on behalf of `actor`.
    ///
    /// Two checks apply, and each attempt is audited whether it succeeds or not:
    /// - the actor must hold `grant` (or a broader capability covering it);
    /// - the actor must already hold `capability`. An actor cannot hand out permissions it
    ///   does not have.
    pub fn grant_as(&mut self, actor: Uuid, target: Uuid, capability: Capability) -> Decision {
        let action = format!("grant {capability} to {target}");
        let decision = if !self.holds(actor, &Capability::parse("grant").expect("static")) {
            Decision::Denied {
                reason: "actor may not grant capabilities".to_owned(),
            }
        } else if !self.holds(actor, &capability) {
            Decision::Denied {
                reason: format!("actor does not hold {capability}"),
            }
        } else {
            self.grant(target, capability);
            Decision::Allowed
        };
        self.record(actor, action, decision.clone());
        decision
    }

    /// Checks whether `subject` may perform `capability`, and records the decision.
    pub fn authorize(&mut self, subject: Uuid, capability: &Capability) -> Decision {
        let decision = if self.holds(subject, capability) {
            Decision::Allowed
        } else {
            Decision::Denied {
                reason: format!("no grant covers {capability}"),
            }
        };
        self.record(subject, capability.to_string(), decision.clone());
        decision
    }

    /// Checks without recording. For UI that shows what would happen.
    pub fn holds(&self, subject: Uuid, capability: &Capability) -> bool {
        self.grants
            .get(&subject)
            .is_some_and(|set| set.iter().any(|grant| grant.covers(capability)))
    }

    /// Exact grants held by `subject`, sorted. This is the data behind a capability review screen.
    pub fn grants_for(&self, subject: Uuid) -> Vec<&Capability> {
        self.grants
            .get(&subject)
            .map(|set| set.iter().collect())
            .unwrap_or_default()
    }

    /// Every audit entry so far, oldest first.
    pub fn audit_log(&self) -> &[AuditEntry] {
        &self.audit
    }

    fn record(&mut self, subject: Uuid, action: String, decision: Decision) {
        self.audit.push(AuditEntry {
            subject,
            action,
            decision,
        });
    }
}
