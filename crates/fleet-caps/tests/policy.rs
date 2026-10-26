use fleet_caps::{Capability, Decision, Policy};
use uuid::Uuid;

fn cap(text: &str) -> Capability {
    Capability::parse(text).expect("valid capability")
}

#[test]
fn everything_is_denied_by_default() {
    let mut policy = Policy::new();
    let robot = Uuid::new_v4();
    assert!(!policy.authorize(robot, &cap("move")).is_allowed());
    assert!(!policy.holds(robot, &cap("deploy")));
}

#[test]
fn an_exact_grant_allows_the_action() {
    let mut policy = Policy::new();
    let robot = Uuid::new_v4();
    policy.grant(robot, cap("move"));
    assert_eq!(policy.authorize(robot, &cap("move")), Decision::Allowed);
}

#[test]
fn a_broad_grant_covers_narrower_actions() {
    let mut policy = Policy::new();
    let robot = Uuid::new_v4();
    policy.grant(robot, cap("sensor:read"));
    assert!(policy.holds(robot, &cap("sensor:read:lidar")));
    assert!(!policy.holds(robot, &cap("sensor:write")));
    assert!(
        !policy.holds(robot, &cap("sensor")),
        "a prefix of a grant is not covered"
    );
}

#[test]
fn a_narrow_grant_does_not_cover_broader_actions() {
    let mut policy = Policy::new();
    let robot = Uuid::new_v4();
    policy.grant(robot, cap("sensor:read:lidar"));
    assert!(!policy.holds(robot, &cap("sensor:read")));
    assert!(!policy.holds(robot, &cap("sensor:read:camera")));
}

#[test]
fn grants_do_not_leak_between_subjects() {
    let mut policy = Policy::new();
    let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
    policy.grant(a, cap("deploy"));
    assert!(policy.holds(a, &cap("deploy")));
    assert!(!policy.holds(b, &cap("deploy")));
}

#[test]
fn revoke_removes_only_the_exact_grant() {
    let mut policy = Policy::new();
    let robot = Uuid::new_v4();
    policy.grant(robot, cap("sensor:read"));
    policy.grant(robot, cap("move"));

    assert!(policy.revoke(robot, &cap("sensor:read")));
    assert!(!policy.holds(robot, &cap("sensor:read:lidar")));
    assert!(policy.holds(robot, &cap("move")));

    assert!(
        !policy.revoke(robot, &cap("sensor:read")),
        "second revoke finds nothing"
    );
}

#[test]
fn revoking_the_last_grant_leaves_nothing_behind() {
    let mut policy = Policy::new();
    let robot = Uuid::new_v4();
    policy.grant(robot, cap("move"));
    policy.revoke(robot, &cap("move"));
    assert!(policy.grants_for(robot).is_empty());
}

#[test]
fn an_actor_without_grant_cannot_grant_anything() {
    let mut policy = Policy::new();
    let (actor, target) = (Uuid::new_v4(), Uuid::new_v4());
    policy.grant(actor, cap("move"));

    let decision = policy.grant_as(actor, target, cap("move"));
    assert!(!decision.is_allowed());
    assert!(!policy.holds(target, &cap("move")));
}

#[test]
fn an_actor_cannot_hand_out_what_it_does_not_hold() {
    let mut policy = Policy::new();
    let (admin, target) = (Uuid::new_v4(), Uuid::new_v4());
    policy.grant(admin, cap("grant"));
    policy.grant(admin, cap("move"));

    let decision = policy.grant_as(admin, target, cap("actuate:gripper"));
    assert!(matches!(decision, Decision::Denied { .. }));
    assert!(!policy.holds(target, &cap("actuate:gripper")));
}

#[test]
fn a_permitted_delegation_succeeds() {
    let mut policy = Policy::new();
    let (admin, target) = (Uuid::new_v4(), Uuid::new_v4());
    policy.grant(admin, cap("grant"));
    policy.grant(admin, cap("sensor:read"));

    assert_eq!(
        policy.grant_as(admin, target, cap("sensor:read:lidar")),
        Decision::Allowed
    );
    assert!(policy.holds(target, &cap("sensor:read:lidar")));
}

#[test]
fn escalation_attempts_are_audited() {
    let mut policy = Policy::new();
    let (agent, victim) = (Uuid::new_v4(), Uuid::new_v4());
    policy.grant(agent, cap("move"));

    policy.grant_as(agent, agent, cap("deploy"));
    policy.authorize(victim, &cap("deploy"));

    let log = policy.audit_log();
    assert_eq!(log.len(), 2);
    assert_eq!(log[0].subject, agent);
    assert!(!log[0].decision.is_allowed());
    assert_eq!(log[1].subject, victim);
    assert!(!log[1].decision.is_allowed());
}

#[test]
fn allowed_actions_are_audited_too() {
    let mut policy = Policy::new();
    let robot = Uuid::new_v4();
    policy.grant(robot, cap("move"));
    policy.authorize(robot, &cap("move"));

    let log = policy.audit_log();
    assert_eq!(log.len(), 1);
    assert!(log[0].decision.is_allowed());
    assert_eq!(log[0].action, "move");
}

#[test]
fn holds_does_not_write_to_the_audit_log() {
    let mut policy = Policy::new();
    let robot = Uuid::new_v4();
    policy.grant(robot, cap("move"));
    let _ = policy.holds(robot, &cap("move"));
    assert!(policy.audit_log().is_empty());
}

#[test]
fn grants_for_lists_exact_grants_in_order() {
    let mut policy = Policy::new();
    let robot = Uuid::new_v4();
    policy.grant(robot, cap("sensor:read"));
    policy.grant(robot, cap("move"));
    let listed: Vec<String> = policy
        .grants_for(robot)
        .iter()
        .map(|c| c.to_string())
        .collect();
    assert_eq!(listed, ["move", "sensor:read"]);
}

#[test]
fn malformed_capabilities_are_rejected() {
    assert!(Capability::parse("").is_err());
    assert!(Capability::parse("move:").is_err());
    assert!(Capability::parse(":move").is_err());
    assert!(Capability::parse("sensor::lidar").is_err());
    assert!(Capability::parse("sensor read").is_err());
}
