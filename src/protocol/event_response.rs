#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum EventResponse {
  Switch {
    switch_id: usize,
    state: SwitchState,
  },
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum SwitchState {
  Open,
  Closed,
}
