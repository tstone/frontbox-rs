use crate::Placement;

/// How many initials a player enters
pub const INITIALS_LENGTH: usize = 3;

/// Shown in place of a letter to go back to the one before
pub const BACK: char = '<';

/// What a player can choose for each initial, in the order the buttons step through them. [`BACK`] comes after these,
/// from the second initial on.
const CHARACTERS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 ";

/// A player's initials as they're being entered: the letters accepted so far, and the one being chosen. It's the same
/// whatever the machine shows it on. [`LeaderboardsSystem`](crate::LeaderboardsSystem) runs one for each player who
/// made a board, and a system for the machine's display draws it.
#[derive(Debug, Clone)]
pub struct InitialsEntry {
  player: u8,
  placements: Vec<Placement>,
  accepted: Vec<char>,
  /// The letter being chosen, or [`BACK`]
  choosing: char,
}

impl InitialsEntry {
  /// Entry for `player` (0 for player 1), who made `placements`
  pub fn new(player: u8, placements: Vec<Placement>) -> Self {
    Self {
      player,
      placements,
      accepted: Vec::new(),
      choosing: 'A',
    }
  }

  /// Whose initials these are, 0 for player 1
  pub fn player(&self) -> u8 {
    self.player
  }

  /// What the player made, which their initials go on
  pub fn placements(&self) -> &[Placement] {
    &self.placements
  }

  /// Which initial is being chosen, from 0
  pub fn cursor(&self) -> usize {
    self.accepted.len()
  }

  /// What's shown at each position: the accepted letters, then the one being chosen (which can be [`BACK`]). `None`
  /// for a position not reached yet.
  pub fn letters(&self) -> [Option<char>; INITIALS_LENGTH] {
    let mut letters = [None; INITIALS_LENGTH];
    for (at, letter) in self.accepted.iter().enumerate() {
      letters[at] = Some(*letter);
    }
    letters[self.cursor()] = Some(self.choosing);
    letters
  }

  /// Change the letter being chosen to the one after it, going around at the end
  pub fn next(&mut self) {
    self.step(1);
  }

  /// Change the letter being chosen to the one before it, going around at the start
  pub fn previous(&mut self) {
    self.step(-1);
  }

  /// Accept the letter being chosen and move on to the next, or with [`BACK`] chosen, go back to change the one
  /// before. Returns the initials once the last is accepted.
  pub fn accept(&mut self) -> Option<String> {
    if self.choosing == BACK {
      self.choosing = self.accepted.pop().unwrap_or('A');
      return None;
    }
    if self.accepted.len() + 1 == INITIALS_LENGTH {
      return Some(self.initials());
    }
    self.accepted.push(self.choosing);
    self.choosing = 'A';
    None
  }

  /// The initials as they stand, for a player who stopped part way: the accepted letters and the one being chosen,
  /// without spaces at the end
  pub fn initials(&self) -> String {
    let mut initials: String = self.accepted.iter().collect();
    if self.choosing != BACK {
      initials.push(self.choosing);
    }
    initials.trim_end().to_string()
  }

  fn step(&mut self, by: isize) {
    let mut choices: Vec<char> = CHARACTERS.chars().collect();
    if !self.accepted.is_empty() {
      choices.push(BACK);
    }
    let at = choices
      .iter()
      .position(|c| *c == self.choosing)
      .unwrap_or(0) as isize;
    self.choosing = choices[(at + by).rem_euclid(choices.len() as isize) as usize];
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn entry() -> InitialsEntry {
    InitialsEntry::new(0, Vec::new())
  }

  /// Steps forward from `A` to `letter`, and accepts it
  fn choose(entry: &mut InitialsEntry, letter: char) -> Option<String> {
    for _ in 0..CHARACTERS.find(letter).unwrap() {
      entry.next();
    }
    entry.accept()
  }

  #[test]
  fn three_letters_are_accepted_one_at_a_time() {
    let mut entry = entry();
    assert_eq!(entry.letters(), [Some('A'), None, None]);

    assert_eq!(choose(&mut entry, 'T'), None);
    assert_eq!(entry.cursor(), 1);
    assert_eq!(entry.letters(), [Some('T'), Some('A'), None]);
    assert_eq!(choose(&mut entry, 'M'), None);
    assert_eq!(choose(&mut entry, 'S'), Some("TMS".to_string()));
  }

  #[test]
  fn the_letters_go_around_both_ways() {
    let mut entry = entry();
    entry.previous();
    assert_eq!(entry.letters()[0], Some(' '));
    entry.previous();
    assert_eq!(entry.letters()[0], Some('9'));
    entry.next();
    entry.next();
    assert_eq!(entry.letters()[0], Some('A'));
  }

  #[test]
  fn back_is_offered_after_the_first_letter_and_returns_to_the_one_before() {
    let mut entry = entry();
    choose(&mut entry, 'T');
    choose(&mut entry, 'M');

    entry.previous();
    assert_eq!(entry.letters(), [Some('T'), Some('M'), Some(BACK)]);
    assert_eq!(entry.accept(), None);
    // the second letter is being chosen again, from what it was
    assert_eq!(entry.letters(), [Some('T'), Some('M'), None]);
    entry.next();
    assert_eq!(entry.accept(), None);
    assert_eq!(choose(&mut entry, 'S'), Some("TNS".to_string()));
  }

  #[test]
  fn initials_part_way_leave_out_back_and_spaces_at_the_end() {
    let mut entry = entry();
    assert_eq!(entry.initials(), "A");
    choose(&mut entry, 'T');
    entry.previous();
    assert_eq!(entry.initials(), "T");
    entry.previous();
    assert_eq!(entry.letters()[1], Some(' '));
    assert_eq!(entry.initials(), "T");
  }
}
