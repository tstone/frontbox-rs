use std::collections::VecDeque;
use std::time::Instant;

use frontbox_core::prelude::*;

/// A line to say. A plain key is a request with no priority, lead-in or limit on its wait.
///
/// ```rust
/// # use frontbox_core::prelude::*;
/// # use frontbox_sound::*;
/// # fn example(ctx: &SystemContext) {
/// ctx.play_callout("nice_shot");
/// ctx.play_callout(CalloutRequest::new("multiball").priority(1).lead_in("chime"));
/// ctx.play_callout(CalloutRequest::new("nice_shot").max_wait(Duration::from_secs(2)));
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalloutRequest {
  key: &'static str,
  priority: u8,
  lead_in: Option<&'static str>,
  max_wait: Option<Duration>,
}

impl CalloutRequest {
  pub fn new(key: &'static str) -> Self {
    Self {
      key,
      priority: 0,
      lead_in: None,
      max_wait: None,
    }
  }

  /// Queuing this replaces any waiting lines of a lower priority. The line already playing always plays out.
  pub fn priority(mut self, priority: u8) -> Self {
    self.priority = priority;
    self
  }

  /// A sound played right before the line, as part of it
  pub fn lead_in(mut self, key: &'static str) -> Self {
    self.lead_in = Some(key);
    self
  }

  /// Give up on the line if it's waited this long, for one that's only worth saying around the time it happened
  pub fn max_wait(mut self, max_wait: Duration) -> Self {
    self.max_wait = Some(max_wait);
    self
  }

  pub(crate) fn key(&self) -> &'static str {
    self.key
  }

  pub(crate) fn lead_in_key(&self) -> Option<&'static str> {
    self.lead_in
  }
}

impl From<&'static str> for CalloutRequest {
  fn from(key: &'static str) -> Self {
    Self::new(key)
  }
}

/// The lines waiting for the one playing to end, in the order they'll be said. Higher priorities go first, and priorities
/// tie by who asked first.
#[derive(Default)]
pub(crate) struct CalloutQueue {
  waiting: VecDeque<Waiting>,
}

impl CalloutQueue {
  /// Add a line, replacing any of a lower priority. Returns the keys of the lines replaced.
  pub fn push(&mut self, request: CalloutRequest, now: Instant) -> Vec<&'static str> {
    let replaced = self.remove(|line| line.request.priority < request.priority);
    // what's left is at least this priority, so the back is this line's place
    self.waiting.push_back(Waiting {
      request,
      queued_at: now,
    });
    replaced
  }

  pub fn pop(&mut self) -> Option<CalloutRequest> {
    self.waiting.pop_front().map(|line| line.request)
  }

  /// Remove the lines that have waited too long. Returns their keys.
  pub fn take_stale(&mut self, now: Instant) -> Vec<&'static str> {
    self.remove(|line| {
      line
        .request
        .max_wait
        .is_some_and(|max_wait| now.duration_since(line.queued_at) >= max_wait)
    })
  }

  fn remove(&mut self, matches: impl Fn(&Waiting) -> bool) -> Vec<&'static str> {
    let (removed, kept): (VecDeque<_>, VecDeque<_>) =
      self.waiting.drain(..).partition(|line| matches(line));
    self.waiting = kept;
    removed.iter().map(|line| line.request.key).collect()
  }
}

struct Waiting {
  request: CalloutRequest,
  queued_at: Instant,
}

#[cfg(test)]
mod tests {
  use super::*;

  fn order(queue: &mut CalloutQueue) -> Vec<&'static str> {
    std::iter::from_fn(|| queue.pop())
      .map(|request| request.key())
      .collect()
  }

  #[test]
  fn lines_are_said_in_the_order_they_were_asked_for() {
    let mut queue = CalloutQueue::default();
    let now = Instant::now();
    queue.push("first".into(), now);
    queue.push("second".into(), now);
    queue.push("third".into(), now);

    assert_eq!(order(&mut queue), vec!["first", "second", "third"]);
  }

  #[test]
  fn a_higher_priority_replaces_lower_ones_waiting() {
    let mut queue = CalloutQueue::default();
    let now = Instant::now();
    queue.push("hit".into(), now);
    queue.push("hit again".into(), now);

    let replaced = queue.push(CalloutRequest::new("mode start").priority(1), now);

    assert_eq!(replaced, vec!["hit", "hit again"]);
    assert_eq!(order(&mut queue), vec!["mode start"]);
  }

  #[test]
  fn a_lower_priority_waits_behind_a_higher_one_and_replaces_nothing() {
    let mut queue = CalloutQueue::default();
    let now = Instant::now();
    queue.push(CalloutRequest::new("mode start").priority(1), now);

    let replaced = queue.push("hit".into(), now);

    assert!(replaced.is_empty());
    assert_eq!(order(&mut queue), vec!["mode start", "hit"]);
  }

  #[test]
  fn the_same_priority_replaces_nothing() {
    let mut queue = CalloutQueue::default();
    let now = Instant::now();
    queue.push(CalloutRequest::new("one").priority(1), now);
    queue.push(CalloutRequest::new("two").priority(1), now);

    assert_eq!(order(&mut queue), vec!["one", "two"]);
  }

  #[test]
  fn only_lines_past_their_wait_go_stale() {
    let mut queue = CalloutQueue::default();
    let now = Instant::now();
    let limit = Duration::from_secs(2);
    queue.push(CalloutRequest::new("hit").max_wait(limit), now);
    queue.push("patient".into(), now);

    assert!(queue.take_stale(now + Duration::from_secs(1)).is_empty());
    assert_eq!(queue.take_stale(now + limit), vec!["hit"]);
    assert_eq!(order(&mut queue), vec!["patient"]);
  }
}
