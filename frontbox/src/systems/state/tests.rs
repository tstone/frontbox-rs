use super::*;
use crate::app::event_box::EventBox;

#[derive(State, Serialize, Default, Clone, PartialEq, Debug)]
enum Phase {
  #[default]
  Idle,
  Active,
  Ending,
}

struct Owner;
impl System for Owner {}

struct Other;
impl System for Other {}

/// Puts the system in the group, returning its handle
fn insert(t: &mut TestContext, group: GroupId, system: impl System + 'static) -> SystemHandle {
  let system = SystemContainer::new(system);
  let handle = SystemHandle::new(system.id(), group);
  t.groups.get_mut(&group).unwrap().insert(system);
  handle
}

fn add_group(t: &mut TestContext) -> GroupId {
  let group = GroupId::next();
  t.groups.insert(group, SystemGroup::new("player"));
  group
}

/// Each `StateChanged<Phase>` as its `(from, to)`
fn changes(events: Vec<EventBox>) -> Vec<(Phase, Phase)> {
  events
    .iter()
    .map(|event| {
      let changed = event
        .event
        .downcast_ref::<StateChanged<Phase>>()
        .expect("only phases change in these tests");
      (changed.from.clone(), changed.to.clone())
    })
    .collect()
}

fn read(t: &TestContext, handle: SystemHandle) -> Phase {
  t.svc_ctx().state::<Phase>(handle)
}

#[test]
fn a_state_reads_as_its_default_until_set() {
  let t = TestContext::default();
  assert_eq!(t.sys_ctx().state::<Phase>(), Phase::Idle);
}

#[test]
fn a_state_is_read_back_once_set_and_its_change_broadcast() {
  let mut t = TestContext::default();
  let owner = insert(&mut t, ROOT_GROUP, Owner);

  t.svc_ctx().for_system(owner).set_state(Phase::Active);
  // not until the run loop gets to it
  assert_eq!(read(&t, owner), Phase::Idle);

  assert_eq!(
    changes(t.apply_states()),
    vec![(Phase::Idle, Phase::Active)]
  );
  assert!(t.svc_ctx().for_system(owner).in_state(Phase::Active));
}

#[test]
fn a_state_can_be_matched_against_several_values() {
  let mut t = TestContext::default();
  let owner = insert(&mut t, ROOT_GROUP, Owner);
  let ctx = t.svc_ctx().for_system(owner);

  assert!(ctx.in_any_state(&[Phase::Idle, Phase::Ending]));
  assert!(!ctx.in_any_state(&[Phase::Active, Phase::Ending]));
  assert!(!ctx.in_any_state::<Phase>(&[]));

  ctx.set_state(Phase::Ending);
  t.apply_states();
  let ctx = t.svc_ctx().for_system(owner);
  assert!(ctx.in_any_state(&[Phase::Active, Phase::Ending]));
}

#[test]
fn setting_the_same_value_broadcasts_nothing() {
  let mut t = TestContext::default();
  let owner = insert(&mut t, ROOT_GROUP, Owner);

  t.svc_ctx().for_system(owner).set_state(Phase::Active);
  t.apply_states();
  t.svc_ctx().for_system(owner).set_state(Phase::Active);

  assert_eq!(changes(t.apply_states()), vec![]);
}

#[test]
fn only_the_owner_can_set_a_state() {
  let mut t = TestContext::default();
  let owner = insert(&mut t, ROOT_GROUP, Owner);
  let other = insert(&mut t, ROOT_GROUP, Other);

  t.svc_ctx().for_system(owner).set_state(Phase::Active);
  t.apply_states();
  t.svc_ctx().for_system(other).set_state(Phase::Ending);

  assert_eq!(changes(t.apply_states()), vec![]);
  assert_eq!(read(&t, other), Phase::Active);
}

#[test]
fn a_group_reads_its_own_value_before_the_roots() {
  let mut t = TestContext::default();
  let owner = insert(&mut t, ROOT_GROUP, Owner);
  let group = add_group(&mut t);
  let player = insert(&mut t, group, Other);

  t.svc_ctx().for_system(owner).set_state(Phase::Active);
  t.apply_states();
  assert_eq!(read(&t, player), Phase::Active);

  // a separate state of its own, rather than the root's, so it's the player system's to set
  t.svc_ctx().for_system(player).set_state(Phase::Ending);
  assert_eq!(
    changes(t.apply_states()),
    vec![(Phase::Idle, Phase::Ending)]
  );
  assert_eq!(read(&t, player), Phase::Ending);
  assert_eq!(read(&t, owner), Phase::Active);
}

#[test]
fn a_despawned_owners_states_go_back_to_their_default() {
  let mut t = TestContext::default();
  let owner = insert(&mut t, ROOT_GROUP, Owner);
  let other = insert(&mut t, ROOT_GROUP, Other);

  t.svc_ctx().for_system(owner).set_state(Phase::Active);
  t.apply_states();
  t.groups.get_mut(&ROOT_GROUP).unwrap().remove(owner.id);

  let cleared = clear_states_owned_by(&mut t.groups, owner);
  let cleared = cleared
    .iter()
    .filter_map(StoredState::changed_to_default)
    .collect();
  assert_eq!(changes(cleared), vec![(Phase::Active, Phase::Idle)]);
  assert_eq!(read(&t, other), Phase::Idle);

  // up for grabs again
  t.svc_ctx().for_system(other).set_state(Phase::Ending);
  assert_eq!(
    changes(t.apply_states()),
    vec![(Phase::Idle, Phase::Ending)]
  );
}

#[test]
fn a_state_set_by_a_system_since_despawned_is_ignored() {
  let mut t = TestContext::default();
  let owner = insert(&mut t, ROOT_GROUP, Owner);

  t.svc_ctx().for_system(owner).set_state(Phase::Active);
  t.groups.get_mut(&ROOT_GROUP).unwrap().remove(owner.id);

  assert_eq!(changes(t.apply_states()), vec![]);
  assert_eq!(t.sys_ctx().state::<Phase>(), Phase::Idle);
}

#[test]
fn a_state_change_names_the_state_in_its_json() {
  let changed = StateChanged {
    from: Phase::Idle,
    to: Phase::Active,
  };
  let json = changed.to_json().unwrap();

  assert!(json["name"].as_str().unwrap().ends_with("Phase"));
  assert_eq!(json["from"], "Idle");
  assert_eq!(json["to"], "Active");
}
