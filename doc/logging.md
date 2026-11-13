# Log Targets

Every log call in frontbox names a target, so you can choose which parts of the framework to hear from. A target
names a subject you'd want to watch (the trough, the serial link), not the file the log comes from.

```rust
log::info!(target: "frontbox::scoop", "Ejecting {}", self.name);
```

## Filtering

Frontbox uses the [`log`](https://docs.rs/log) crate, so any logger works. With
[`env_logger`](https://docs.rs/env_logger), set `RUST_LOG`:

```sh
# Everything at info, plus ball handling at debug
RUST_LOG=info,frontbox::ball_management=debug

# Systems at trace, without the per-event skip messages
RUST_LOG=info,frontbox::systems=trace,frontbox::systems::delivery=off
```

A filter matches every target that starts with it, so `frontbox::ball_management` also covers
`frontbox::ball_management::trough`. When two filters match, the longer one wins.

## Targets

### App and run loop

| Target | Covers |
|---|---|
| `frontbox::app` | Startup, opening the metrics store |
| `frontbox::run_loop` | Message handling and timing, slow messages, shutdown |
| `frontbox::config` | Reading and saving TOML files and operator config |
| `frontbox::metrics` | Writing metrics |

### Systems

| Target | Covers |
|---|---|
| `frontbox::systems` | Spawning, despawning, activating and deactivating systems and groups; panics; lookups of systems that aren't running or are in use; state |
| `frontbox::systems::delivery` | Each time an inactive system is skipped for an event (trace, noisy) |
| `frontbox::events` | Each event emitted |
| `frontbox::cues` | Creating, firing and completing cues |
| `frontbox::routines` | Routines starting, handing between participants, halting, pausing, cancelling and completing |

### Hardware

| Target | Covers |
|---|---|
| `frontbox::hardware` | Hardware definitions, LED and reference plane setup, hardware queries |
| `frontbox::switches` | Switch opens and closes, unknown switches, switch combos |
| `frontbox::machine` | Losing and reconnecting to the machine, hardware that doesn't match the configuration, malformed data |
| `frontbox::machine::boot` | Opening ports, resetting boards, configuring drivers |
| `frontbox::machine::serial` | Every frame sent and received, lost ports, unclaimed responses |
| `frontbox::machine::vm` | Commands sent to the virtual machine |
| `frontbox::watchdog` | Enabling, disabling and pinging the watchdog |

### Pinball

| Target | Covers |
|---|---|
| `frontbox::ball_management` | Ball counts, ejects into play, drains |
| `frontbox::ball_management::trough` | Trough occupancy and ejects |
| `frontbox::ball_management::plunge_lane` | Balls arriving in and leaving the plunge lane |
| `frontbox::ball_management::autoplunger` | Autoplunger launches |
| `frontbox::ball_management::ball_search` | Ball search |
| `frontbox::ball_save` | Ball save starting, ending, and saving drains |
| `frontbox::tilt` | Tilt warnings, tilts and slam tilts |
| `frontbox::scoop` | Scoop captures and ejects |
| `frontbox::coin_door` | Coin door open and close |
| `frontbox::game_manager` | Games, players, turns, free play, the quit combo |
| `frontbox::high_scores` | Saving high scores |

### Output

| Target | Covers |
|---|---|
| `frontbox::scene` | Scene playbacks starting and ending |
| `frontbox::led` | LED declarations and rendering |
| `frontbox::sound` | Audio output, sound effects, callouts, music |
| `frontbox::dmd` | DMD rendering, menus, initials entry |
| `frontbox::console` | The web console server, its clients, and commands sent from it |

## Adding a target

Use an existing target when the subject fits. Add a child target (`frontbox::machine::boot`) when a part of a subject
is noisy or worth watching on its own; a new subject gets a new top-level target. Add it to the tables above.
