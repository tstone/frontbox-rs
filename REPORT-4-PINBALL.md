# Report 4: frontbox-pinball

| # | Finding | Effect | Size of fix |
|---|---|---|---|
| B1 | The game manager does four jobs behind one trait | Teams, split flipper, or any other game type means rewriting all of it | Large |
| B2 | Game types can't be picked at the start of a game | No tournament, team, or split flipper choice from a menu | Medium, later |
| C | Missing pinball behavior | Credits, bonus, match, multiball, and more | Varies |

## B. Architecture and types

### B1. The game manager does four jobs

`CompetitiveGame` is about 600 lines of logic doing four things:

| Part | Lines | Differs between game types? |
|---|---|---|
| Turn: serve, going active, the drain queue and `BallDraining`, `TurnEnding`, waiting for balls, the in-play check, lost balls, coin door drains, tilt | ~300 | No |
| Game: `GameStarting` and `GameEnding`, player entry, the ball return timer, quit | ~120 | Rarely |
| Scores and extra balls: points, multipliers, ignoring points after a tilt | ~80 | Only in who owns them |
| Rotation: who is up next, which group to activate, when the game is over | ~60 | Yes |

`GameManager` exposes all four. `GameSession` keeps players as five parallel `Vec`s, with a `Cooperative` variant
nothing constructs, and every accessor written out twice. Beside the phases, `CompetitiveGame` keeps flags that each
mean something in one phase only (`ball_served`, `draining`, `tilted`, `waiting_for_balls`,
`found_no_ball_in_play`), and `start_turn`, `tilt`, and `start_game_ending` each reset a different subset.

The game types this has to support:

- **Teams (co-op).** Each player still has their own turns, extra balls, and score. Players on a team share one system
  group (modes, progress) and their points: a point scored by any teammate is added to every teammate's score.
  Competitive play is the case where every team has one player.
- **Split flipper.** Two players compete at the same time, one flipper each, on the whole playfield. Each owns the
  shots made from their flipper. There's one ball, so a drain ends the turn for both.
- **Tournament.** Standard rules with different config.
- Others later, such as timed play, where a turn ends for other reasons.

**Suggestion.** Split along what a turn is, who it belongs to, and who is up next:

| Piece | Kind | Job | Knows about players? |
|---|---|---|---|
| `GameSession` | data | Players and teams | yes |
| `ScoreSystem` | root system | Scores and multipliers per team | teams only |
| `TurnSystem` | root system | Runs one turn in one or more groups, owns `TurnPhase` | no |
| Game manager | root system | Game lifecycle and rotation | yes |

- **`GameSession` as a struct.** `players: Vec<Player>` (turn, extra balls, extra balls granted, team index) and
  `teams: Vec<Team>` (group, score). Competitive play makes one team per player. This replaces both variants.
- **`ScoreSystem`.** Scores are kept per team, and a player's score is their team's. The core call is
  `add_points(team, points)`, for split flipper, where the shot decides who scores. Rules code mostly uses
  `add_points_to_current`, which scores for the team whose turn it is. Points are ignored for the rest of a tilted
  turn, and the game manager reads final scores from it. `add_points` and the multiplier methods leave the
  `GameManager` trait.
- **`TurnSystem`.** `start(groups, reason)` runs one turn: the serve, going active, drains, `TurnEnding`, and waiting
  for balls. It emits `TurnCompleted { reason }` once no ball is left in play. It takes a list of groups, as split
  flipper runs both players' groups on one ball. `end(reason)` ends the turn from outside: a tilt now, a timer later.
  It knows nothing about players or turn order. Inside, a private enum replaces the flags: `Starting { served }`,
  `Active { deciding_drain }`, `Ending { routine_completed }`, so the match in `on_event` says what each phase does
  with each event. Most of today's `CompetitiveGame` tests are turn tests, and move with it.
- **The game manager** keeps the game lifecycle, player entry, and rotation. On `TurnCompleted` it picks who is up
  next or ends the game. Extra balls stay here, since "the same player again" means something different in each game
  type. Rename it `Game`, since it handles teams as well.
- **Two templates.** `team_systems` (modes, progress) is spawned once per team, and `player_systems` (`TiltSystem`)
  once per player, so teammates don't share tilt warnings that last the game. In competitive play they come to the
  same thing.
- **Teams in `Game`.** A team count in config, with players assigned in turn (`player % teams`: with 4 players and 2
  teams, players 1 and 3 play against 2 and 4). A team's group is spawned with its first player, and a player who
  joins later joins that group and its score.
- **`dyn GameManager` stays the only interface** the start button, credits, and displays use: `add_player`,
  `quit_game`, `session`. A game type with very different rules (split flipper) is its own game manager that reuses
  `TurnSystem` and `ScoreSystem`. Some duplication of the game lifecycle there is fine.

Open: `GameEnding.scores` lists a team's shared score once per teammate, so every teammate enters initials for the
same score.

### B2. Picking a game type

Not for now. Machines usually offer a menu while no game is running (hold a flipper or the action button): standard,
tournament, teams, split flipper. Most choices are config, not different systems:

| Choice | What changes |
|---|---|
| Standard | nothing |
| Tournament | config: fixed seed, no extra balls, no random awards |
| Teams | config: team count |
| Split flipper | a different game manager, and which flippers are enabled for whom |

- **Config overrides for one game.** "No extra balls" must not overwrite the operator's saved setting, so overrides
  are reverted when the game ends. This covers three of the four choices.
- **Swapping systems.** Lookups and states search the caller's group, then the root, and nothing else
  (`GroupsExt::nearest`, `nearest_state`). So the game manager, `TurnSystem`, and `ScoreSystem` must be root
  systems, and swapping a game type means despawning and spawning root systems one at a time. There are two ways
  out, to choose between when split flipper is built:
  1. Lookups follow the owner chain: the caller's group, then the group of the system that owns it, and so on up to
     the root. A game type is then one group the selector spawns, with the player groups inside it, and despawning it
     removes everything. Root systems still need `GamePhase` and `TurnPhase`, so those are set at the root
     explicitly.
  2. A `GameType` returns the systems it needs, and the selector despawns the old set and spawns the new one. This
     needs no change to core.
- The menu opens only with no game running and the balls home, the same as `PlayerEntry`.

For now, `TurnSystem` and `ScoreSystem` should not assume they are the only instance, or that they live at the root.

### B5. Requests keyed by owner

`BallSaveSystem.requests`, lotko's scoop `holds`, LED declarations, and score multipliers are the same thing: a value
per requesting system, removed when that system releases it or despawns, sometimes with a time limit. `TODO.md` asks
whether a "hold" should be a framework feature. Yes: a small collection type in core (`Requests<T>`: insert by owner
id, release, expire on tick, drop on `SystemDespawned`) would replace the hand-written copies, and each one today
handles despawn and deactivation slightly differently. `ScoreSystem`'s multipliers would be its first new user.

### B6. Events and names

- Turn events and routines name one `current_player`. They should carry the team and the players who are up, since
  two are up at once in split flipper. Do this before teams, as every listener changes.
- `PointsAdded` names one player. It should carry the team, since every teammate's display changes.
- `PlayerAdded` has no fields. Listeners need the player's index, their team, and the new count.
- `GameEnding.scores` is `Vec<(&'static str, u32)>`, where the string is the system group's name. Pass the index, or a
  `Player`, and let presenters name it.
- The same value is `current_player` in the routines and `player_index` in the events.
- `TurnActive::new` and `PointsAdded::new` add nothing over the public fields.
- `AGENTS.md` puts a module's public events in `mod.rs`. Game management keeps them in `game_events.rs`.

### B7. Redundancy

- **Fire, verify, retry is written three times**, or will be: trough ejects, the lane arrival kick, and the autoplunger
  (A5). Lotko's scoop has a fourth. One helper (coil, verifying switch, timeout, retry limit, optional stronger
  kick) belongs in a shared helper, and gives the failed eject item in `TODO.md` one place to add
  backoff.
- **Unused**: `TroughEjectStarted` is emitted and nothing listens; `TroughSystem::eject_many` has no caller;
  `HighScoresSystem.custom_scores` goes with the leaderboards item.
- **`GameStartable`** flashes a lamp through a cycling timer and plays an LED effect, each with its own rule for when it
  runs. The lamp half is done: `CreditsSystem` declares its start lamp, a driver light, flashing through light
  management.

### B8. What lotko wrote that belongs here

Lotko is the evidence for what the crate is missing, since it had to write these itself:

| In lotko | What it would be here |
|---|---|
| `EffectsSystem` | The one-shot LED effect item in `TODO.md` (Report 1, 5.3) |
| Multiball start and end, written twice (`heartwood_multiball`, `mini_wizards/run.rs`) | A multiball helper (section C) |
| `LowerScoopSystem`, `RearScoopSystem` | A generic scoop that ejects a ball found at boot, reports `set_balls_held`, and takes part in ball search (B3) |
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
| Faulty switch tolerance | A trough or lane switch stuck closed changes every count. Report switches not seen in N games, and let the operator mark one as ignored |
| Tilt bob settling between players | The bob is ignored until the serve plus a grace period. A bob still swinging from the last player's tilt can warn the next one. Hold the serve until it has been quiet |
| Coin door and tests | Door-open handling for coil power (`CoinDoorSystem` knows the door), and switch, coil, lamp, and trough tests. These probably live with the DMD menu, but need hooks here (trough test, scoop test) |
| Lane ball left waiting | Operator option to launch after N seconds, and a reminder effect |

**Rules building blocks**

| Missing | Notes |
|---|---|
| Multiball helper | The recipe in the `BallSaveSystem` docs as a type: eject N, save until launched, then timed save, end on `RoutineCompleted<BallDraining>` with one left, add-a-ball. Lotko has it twice |
| Locks | Physical and virtual locks, counted per player, reporting `set_balls_held` |
| Bonus | Named counters, a multiplier, optional carry-over, and a `TurnEnding` participant that skips on tilt. The presentation stays with the game |
| Match, replay, special | `GameEnding` participants and score thresholds, with the award configurable. "Config: No extra balls/points instead" in `TODO.md` is one case |
| Timers that pause | Mode timers, hurry-ups, and ball save should stop while a ball is held or waiting in the lane. One clock that systems read, paused by `set_balls_held` and the lane |
| Shots and sequences | Ordered switch hits within a window (orbits, ramps, combos). Core has `SwitchComboSystem` for held buttons only; lotko has an orbit system per orbit |
| Playfield devices | Drop target bank, kickback, lane change, spinner, and standard driver definitions for flippers, slings, and pops. The crate defines coils only for the trough and autoplunger |
| Status report | Flipper held during play shows a status page; needs a way for systems to contribute lines |

**Game types**

| Missing | Notes |
|---|---|
| Team and cooperative play | B1 |
| Split flipper | B1 and B2, plus flippers enabled per player |
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
delivered by hand. `tests/game_flow.rs` fills the gap between them: a real `CompetitiveGame`, `StandardBallManager`,
trough, and lane, driven only by switch changes. It now covers the lane, tilt, quit, and multiball cases. It is the check
that B1's split doesn't change behavior, and teams are the next scenario for it.

## Suggested order

1. **`GameSession` as a struct** of players and teams, with one team per player. No change in behavior.
2. **`ScoreSystem`**, with explicit owners. B5 if the multipliers need it.
3. **`TurnSystem`**, starting a list of groups, with the private phase enum. The turn tests move with it, and
   `tests/game_flow.rs` passes unchanged.
4. **B6**, at least the team and players on turn events and `PointsAdded`.
5. **Teams in `Game`**: groups by team, the two templates, and team count in config. A `game_flow` test of 4 players
   on 2 teams, where player 3 picks up a mode player 1 started.
6. From section C: credits, the multiball helper, and attract as a routine, in whichever order the next game needs.
7. Later: config overrides for one game, then the game type menu (B2).
