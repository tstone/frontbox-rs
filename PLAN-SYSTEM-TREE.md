# Plan: systems as a tree

Groups are replaced by a tree of systems, on the `system-tree` branch. The model is documented in
`frontbox-core/src/systems/mod.rs` ("The system tree") and on `Group`.

## Remaining

- Rewrite `REPORT-4-PINBALL.md` B1 and B2 against the tree: player and team `Group`s, `TurnSystem::start` taking
  `Group` ids, and a game type being one system the menu despawns and replaces.
- `ctx.systems.spawn` always starts a child active, so the game manager spawns each player's `Group` and then
  deactivates it, and the player's systems get `on_spawn` then `on_deactivate`. That matches what groups did, but a
  `spawn_inactive` (or a `Group` that starts inactive) would say what's meant.
- Finding a system's children scans every group (`GroupsExt::children_group`), and a lookup does that once, then walks
  up the parents. Fine at today's size (around ten groups). Index children by owner if lookups show up in the run
  loop's timing warnings.
- Lotko's "Oak soon" callout at `GameEnding` reads the last player's `ModeManager`. With more than one player it
  probably wants the furthest any player got.

## Open

- Whether a top-level system should be able to look up "the active one" of a type inside groups (`Dmd` showing the
  current player's mode). `get_in` with the current player's group covers it, so this waits until something needs it.
