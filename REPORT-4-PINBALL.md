# Report 4: frontbox-pinball

Scope: all of `frontbox-pinball` (game management, ball management, ball save, tilt, game start, attract, high scores,
configs, metrics), read alongside lotko as its one consumer. Items already in `TODO.md` are referred to, not repeated.

Everything was found by reading.

## Summary

The turn lifecycle is in good shape. Routines for the turn's stages, `BallDraining` as the place drains are decided,
extra balls as another turn, and per-player system groups all hold up, and the trough's occupancy logic is careful and
well tested. Most of what's wrong sits in two places: where the game manager and ball management meet (no test covers
the two together), and where ball counts stop being derived and become a guess.

| # | Finding | Effect | Size of fix |
|---|---|---|---|
| A3 | Nothing waits for balls to come home when a game ends or starts | A turn that never ends, or a turn ended under a live ball | Small to medium |
| A5 | The autoplunger fires once and never checks | A ball left in the lane stalls the turn or multiball | Small |
| A6 | A drain is decided on a count taken earlier | A turn can end while a ball is on its way out | Small |
| A7 | Scores are `u32` with an `f32` multiplier | Rounded awards, overflow, multipliers that overwrite each other | Medium |
| B3 | Ball counts are half derived | Root of A3; `locked_balls` is a running counter | Medium |
| B1 | The game manager does four jobs behind one trait | A second game type means rewriting all of it | Medium |
| C | Missing pinball behavior | Credits, ball search, bonus, match, multiball, and more | Varies |

## A. Bugs

### A3. Nothing waits for balls to come home

`turn_ended` waits for `balls_in_play() == 0` before advancing, which is right. `quit_game` and `end_game` don't:
the game ends with balls still rolling, and the next `start_game` establishes a count that is short by however many
are out. From there:

- A stray ball that drains while the new turn is `Starting` with its ball served is dropped: that phase handles only
  `BallEnteredPlay` and the ball in play switches, not `BallsExitedPlay`. The count is now off by one in the other
  direction, and the real ball's drain is reconciled as an unexpected ball, with no `BallsExitedPlay`, so the turn
  never ends.
- A stray ball that drains during `Active` is reported as the drain of the ball in play, and ends the turn (or uses
  the ball save) while that ball is still live.

The second is ambiguous on any machine. The first is avoidable.

`waiting_for_balls` also has no time limit. A ball stuck on the playfield after a tilt, or held somewhere when
`TurnEnding` completes, stops the game until someone opens the machine. That is the ball search item in `TODO.md`;
this is one more place it has to run.

**Fixes.**

- `end_game` waits for balls the same way `turn_ended` does, with a limit, then a ball search.
- Handle `BallsExitedPlay` in `Starting` once the ball is served.
- A check that can't be skipped: if the turn is `Active`, no drain is being decided, and ball management reports
  nothing in play, treat it as a drain. This closes the whole class, whatever caused the count to slip.

### A5. The autoplunger fires once and never checks

`launch_from_lane` fires for `Launch::Auto` and returns. It runs again only on `BallReenteredPlungeLane`, which needs
the lane switch to open first. If the kick doesn't move the ball off the switch (weak coil, ball not seated yet),
nothing fires again, and for `Launch::Auto` the launch button isn't armed either. A ball save or multiball ball then
sits in the lane, with every later eject queued behind it.

**Fix.** After firing, start a cue; when it fires, if `is_ball_resting`, fire again, up to a limit. The trough already
does this (fire, verify, retry, give up) and the arrival timeout does a version of it. See B7 for sharing it.

### A6. A drain is decided on a count taken earlier

`BallDraining.in_play` is read when `BallsExitedPlay` is emitted. The decision in `drain_decided` comes later: after
every participant's step, and after any drains queued ahead in `pending_drains`. Also, `in_play` counts balls that
have left the trough, not balls requested and still queued. So `in_play == 0` at decision time can be wrong:

- Drain A is saved and its replacement requested. Drain B, queued behind it with `in_play: 0`, is decided after the
  save has expired. The turn ends while A's replacement is on its way.
- A mode asks for multiball balls, and the one live ball drains before the first of them leaves the trough. Requesting
  a ball save alongside, as the docs recommend, covers this, but nothing enforces it.

**Fix.** Decide on current numbers: in `drain_decided`, end the turn only if `balls_in_play() + balls_pending() == 0`,
where `balls_pending()` is a new count of queued and unfinished ejects. Keep the snapshot in the event for display.

### A7. Scoring

`add_points` computes `(points as f32 * multiplier) as u32` and adds it to a `u32`.

- `f32` holds integers exactly only up to 16,777,216. A larger award is rounded even at a multiplier of 1.0.
- `*score += points_received` overflows at 4.29 billion: a panic in debug builds, a wrap in release.
- There is one multiplier per player and the framework never resets it. A system that sets it and goes inactive at
  the end of the turn leaves it set for the player's next turn.
- Two systems that each want a multiplier (a 2x playfield award and a mode's 3x) overwrite each other, and the first
  to call `clear_multiplier` clears both.
- Points added during `GameEnding` go to player 1: `advance_turn` has already wrapped `current_player` to 0.

**Fix.** `u64` scores with `saturating_add`. Make the multiplier a set of requests keyed by owner (B5), multiplied
together, each removed when its owner despawns or releases it. Keep the math in integers (hundredths, say).

### A8. Smaller items

- `TroughSystem::read_switch_occupancy` unwraps `is_closed`. A misspelled trough switch name panics at spawn with only an
  unwrap message. Check the names when the config is built.
- `PLAYER_GROUP_NAMES` has six entries and `max_players` is unchecked; a seventh player panics.
- `GameManager` forwards eight `System` methods by hand and misses `on_render`. `BallManager` uses
  `delegate_system!`. Same trap as Report 3, section C.
- Docs that describe things that don't exist: `CreditsPlay` and cooperative play in `lib.rs`, `GameStartState` in
  `game_start/mod.rs`, and the README still titled "Frontbox Turn Based". `BallSearchSystem`'s doc is a note to self.
- Every log target here is `frontbox_core::...`. The log target item in `TODO.md` covers it.

## B. Architecture and types

### B1. The game manager does four jobs

`CompetitiveGame` runs the game and turn lifecycle, owns the player groups, keeps scores and multipliers, and keeps
extra balls. `GameManagement` exposes all four, so a second implementation (cooperative, timed, a tournament
variant) has to rewrite about 600 lines to change one of them. `GameState` shows the cost already: it has a
`Cooperative` variant nothing constructs, and every accessor is written out twice, once per variant, for 280 lines.

**Suggestion.**

- `GameState` as a struct: `players: Vec<Player>`, `current`, and `teams: Option<...>` when that exists. `Player`
  holds `score`, `turn`, `extra_balls`, `extra_balls_granted`. That replaces five parallel `Vec`s and both variants.
- Scoring as its own root system (`ScoreSystem`), with the fixes from A7. The game manager reads final scores from it.
- What varies between game types is who is up next and when the game is over. Make that the trait, a few methods over
  `GameState`, and keep one lifecycle implementation.

### B2. Phases with flags beside them

`CompetitiveGame` has `turn_phase` and `game_phase`, plus `ball_served`, `draining`, `tilted`, and
`waiting_for_balls`. Each flag only means something in one phase, and `start_turn`, `tilt`, and `start_game_ending`
each reset a different subset. The dropped drain in A3 lives in one of those combinations (`Starting` with
`ball_served`).

**Suggestion.** Keep the public `TurnPhase` state as it is, and hold a private enum with the detail inside it:
`Starting { served }`, `Active { deciding_drain }`, `Ending { routine_completed }`. The match in `on_event` then
has to say what each one does with each event.

### B3. Ball counts are half derived

`AGENTS.md` says counts are derived from what the hardware reports. Two inputs to `in_play` aren't:

- `max_balls_in_play` is whatever the trough held when the game started. That is a guess, and A3 is what happens when
  it's wrong.
- `locked_balls` is a running counter that any system can move, with no owner and no link to hardware. Lotko never
  calls it: its scoop holds balls without telling ball management, so a held ball counts as in play.

**Suggestion.** Two changes that fit together.

1. **A known ball count.** An operator config for balls installed, defaulting to the number of trough switches.
   `in_play` is then `installed - balls accounted for`, and a ball that isn't accounted for is known to be missing,
   not silently left out. That gives ball search a trigger at game start, and a place for the usual "play with one
   fewer" fallback when the search fails.
2. **Ball holders.** A small trait for anything that can hold a ball and report it from its switches: the trough,
   the plunge lane, scoops, locks, a VUK. Holders register with ball management, which sums their occupancy. This
   one idea covers several open items:
   - the lane's ball is counted like any other holder's, not as a special state in `StandardBallManagement`, and a
     held or locked ball is derived, so `locked_balls` goes away;
   - "Generalized systems: Scoop, lane, StartupEject" in `TODO.md`: lotko's `StartupEject` becomes "every holder
     with a ball at boot ejects it";
   - the `BallSearching` routine's participants are the holders, plus anything else that can pulse;
   - timers can pause while a ball is held (section C).

   Lotko's `LowerScoopSystem` is most of a generic holder already: a switch, an eject coil, and holds keyed by
   owner.

### B4. The interface wrapper is written by hand each time

`GameManager` and `BallManager` are each a `Box<dyn Trait>`, a `Deref`, forwarding of `System`, an `Ext` trait on
`SystemContext`, and a `with_...` helper. `BallSaveExt` and `TiltExt` repeat the last two. `TODO.md` wants
`LedSystem` and `DmdSystem` to become interfaces as well, which would make four copies of the wrapper.

**Suggestion.** Support it once in core: either a lookup by trait (`ctx.systems.get_dyn::<dyn BallManagement>()`,
registered at spawn) or a macro that writes the wrapper. `GameManager` isn't a pure wrapper today because it also
switches the playfield drivers. That part reads only routine events, so it can be a small system of its own
(`PlayfieldDrivers`), which also lets a game replace it.

### B5. Requests keyed by owner

`BallSaveSystem.requests`, lotko's scoop `holds`, LED declarations, and the multiplier in A7 are the same thing: a
value per requesting system, removed when that system releases it or despawns, sometimes with a time limit.
`TODO.md` asks whether a "hold" should be a framework feature. Yes: a small collection type in core
(`Requests<T>`: insert by owner id, release, expire on tick, drop on `SystemDespawned`) would replace the hand-written
copies, and each one today handles despawn and deactivation slightly differently.

### B6. Events and names

- `PlayerAdded` has no fields. Listeners need the player's index and the new count.
- `GameEnding.scores` is `Vec<(&'static str, u32)>`, where the string is the system group's name. Pass the index, or a
  `Player`, and let presenters name it.
- The same value is `current_player` in the routines and `player_index` in the events.
- `TurnActive::new` and `PointsAdded::new` add nothing over the public fields.
- `AGENTS.md` puts a module's public events in `mod.rs`. Game management keeps them in `game_events.rs`.

### B7. Redundancy

- **Fire, verify, retry is written three times**, or will be: trough ejects, the lane arrival kick, and the autoplunger
  (A5). Lotko's scoop has a fourth. One helper (coil, verifying switch, timeout, retry limit, optional stronger
  kick) belongs with the ball holder trait in B3, and gives the failed eject item in `TODO.md` one place to add
  backoff.
- **Unused**: `TroughEjectStarted` is emitted and nothing listens; `TroughSystem::eject_many` has no caller;
  `BallSearchSystem` and `PerformBallSearch` are the stub `TODO.md` describes; `HighScoresSystem.custom_scores`
  goes with the leaderboards item.
- **`GameStartable`** flashes a lamp through a cue pair and plays an LED effect, each with its own rule for when it
  runs. The lamp half is waiting on the `LampDriver` item in `TODO.md`.

### B8. What lotko wrote that belongs here

Lotko is the evidence for what the crate is missing, since it had to write these itself:

| In lotko | What it would be here |
|---|---|
| `EffectsSystem` | The one-shot LED effect item in `TODO.md` (Report 1, 5.3) |
| Multiball start and end, written twice (`heartwood_multiball`, `mini_wizards/run.rs`) | A multiball helper (section C) |
| `LowerScoopSystem`, `StartupEject` | Ball holders (B3) |
| `ComboSystem`'s "time in the scoop doesn't count" | A timer that pauses while a ball is held (section C) |
| `cabinet::double_flip_combo` to skip bonus screens | A standard "skip" for routine participants |
| `AttractModeDmdSystem` beside `AttractSystem` | One attract rotation (below) |

**Attract mode as a routine.** `AttractSystem` plays LED scenes only. A real attract mode rotates pages that each
involve the display, lights, and sometimes sound: logo, high scores, last scores, credits. That is what a routine
already does: `app.routine::<Attracting>(routine![LogoPage, HighScoresPage, LastScoresPage])`, started again each
time it completes while `GamePhase` is `None`. A flipper press is then "complete the current step".

## C. Missing pinball behavior

Not in `TODO.md`, in rough order of how soon a machine needs them.

**To run unattended**

| Missing | Notes |
|---|---|
| Credits and coins | `lib.rs` names `CreditsPlay`; it doesn't exist. Coin switches, pricing, saved credits, service credits. The toggle group item in `TODO.md` is how free play and credit play would switch |
| Missing ball handling | Known ball count, search at game start, play on with fewer (B3). Beyond the ball search item |
| Faulty switch tolerance | A trough or lane switch stuck closed changes every count. Report switches not seen in N games, and let the operator mark one as ignored |
| Tilt bob settling between players | The bob is ignored until the serve plus a grace period. A bob still swinging from the last player's tilt can warn the next one. Hold the serve until it has been quiet |
| Coin door and tests | Door-open handling for coil power, and switch, coil, lamp, and trough tests. These probably live with the DMD menu, but need hooks here (trough test, ball holder test) |
| Lane ball left waiting | Operator option to launch after N seconds, and a reminder effect |

**Rules building blocks**

| Missing | Notes |
|---|---|
| Multiball helper | The recipe in the `BallSaveSystem` docs as a type: eject N, save until launched, then timed save, end on `RoutineCompleted<BallDraining>` with one left, add-a-ball. Lotko has it twice |
| Locks | Physical and virtual locks, counted per player, on top of ball holders |
| Bonus | Named counters, a multiplier, optional carry-over, and a `TurnEnding` participant that skips on tilt. The presentation stays with the game |
| Match, replay, special | `GameEnding` participants and score thresholds, with the award configurable. "Config: No extra balls/points instead" in `TODO.md` is one case |
| Timers that pause | Mode timers, hurry-ups, and ball save should stop while a ball is held or waiting in the lane. One clock that systems read, driven by ball holders |
| Shots and sequences | Ordered switch hits within a window (orbits, ramps, combos). Core has `SwitchComboSystem` for held buttons only; lotko has an orbit system per orbit |
| Playfield devices | Drop target bank, kickback, lane change, spinner, and standard driver definitions for flippers, slings, and pops. The crate defines coils only for the trough and autoplunger |
| Status report | Flipper held during play shows a status page; needs a way for systems to contribute lines |

**Game types**

| Missing | Notes |
|---|---|
| Team and cooperative play | Half declared in `GameState`. B1 makes it a small addition |
| Tournament settings | Fixed seed (the `RandSystem` item), no extra balls, no random awards |
| Last game's scores | Kept after `GameEnding` for attract mode |

## D. Things I checked and would leave alone

- **Routines for the turn's stages.** `TurnStarting`, `BallDraining`, `TurnEnding`, `GameStarting`, `GameEnding`, with
  the game manager hearing `RoutineCompleted<GameEnding>` first. Halting `BallDraining` to save a ball, with the app
  choosing the order, is the right shape, and lotko's outlane save dropped into it without changes here.
- **Extra balls as another turn** with the same turn number, limited per game, and usable when granted during
  `TurnEnding`.
- **Per-player system groups.** A player's state lives in their copy of the template and survives between turns,
  with no player-state API needed.
- **Trough occupancy.** Settling with a maximum, holding the report while an eject is verifying, and separating
  entries from exits are all correct as far as I traced them, with 40 tests. The known limits are written down as
  tests, which is the right way to keep them.
- **`Launch` and `LaunchMethod`.** Callers state intent and the hardware decides. The builder that only allows
  valid hardware setups is worth its three types.
- **Ball save.** Requests per system, a grace period after the last one, cleared on deactivation.
- **Tilt.** Warnings per player, a settle time, points ignored, drains that can't be saved, and waiting for the
  balls before the next turn.

One thing about the tests. Game management is tested against a fake ball manager, and ball management against events
delivered by hand. A3 and A6 are both in the gap between them. The end-to-end item in `TODO.md` is the most
useful testing work in this crate: a real `CompetitiveGame`, `StandardBallManagement`, trough, and lane, driven only
by switch changes. First scenarios: quit with a ball in the lane; tilt during a multiball start; a ball stuck through
a game start; boot with a ball in the lane.

## Suggested order

1. The small items in A8.
2. **A5**, with an end-to-end test that shows it first. It is contained in
   `standard_ball_management.rs` and the game manager's `on_event`.
3. **B3** (known ball count, ball holders), then **A3** and the ball search item from `TODO.md` on top of it. This is
   the largest piece and the one the others lean on.
4. **A7 with B1 and B5**: scoring as its own system, `GameState` as a struct, requests in core.
5. **A6**, once `balls_pending()` exists from step 3.
6. From section C: credits, the multiball helper, and attract as a routine, in whichever order the next game needs.
