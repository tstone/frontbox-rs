//! Tags for the parts of a pinball machine. Most describe where hardware is or what it's part of, and can go on
//! switches, drivers and LEDs alike. A game can define its own as well (see `frontbox_core::hardware::Tag`).

use frontbox_core::prelude::*;

// -- LEDs --

#[derive(Tag)]
pub struct GeneralIllumination;
#[derive(Tag)]
pub struct Insert;

// -- Switches --

/// A switch whose hit means a ball is moving freely in play. `BallSearchSystem` and `CradleSystem` take the
/// playfield to be quiet while none of these is hit, and a game manager's `ball_in_play_switches` are usually these
/// too.
///
/// Recommended for: targets, lanes (including inlanes and outlanes), spinners, pop bumpers and slingshots, ramp
/// entrances and exits, and the entrance switches of scoops and VUKs. A ball only gets to any of them by moving.
///
/// Not recommended for:
/// - The trough and the plunge lane, which ball management watches itself. A ball resting on them isn't loose.
/// - Flipper end-of-stroke switches, which stay closed for as long as a flipper is held up.
/// - Cabinet buttons, the coin door, and the tilt bob, none of which the ball touches.
#[derive(Tag)]
pub struct BallActivity;

// -- Any hardware --

/// On the playfield. This says where hardware is, not what a hit on it means: a switch whose hit means a ball is
/// moving is tagged `BallActivity`.
#[derive(Tag)]
pub struct Playfield;
/// A flipper: its button, its coil, or its end-of-stroke switch. Its buttons alone are
/// `SwitchQ::and(SwitchQ::tag::<Flipper>(), SwitchQ::tag::<Button>())`.
#[derive(Tag)]
pub struct Flipper;
#[derive(Tag)]
pub struct Flasher;
#[derive(Tag)]
pub struct Cabinet;
#[derive(Tag)]
pub struct Lane;
#[derive(Tag)]
pub struct Target;
#[derive(Tag)]
pub struct Ramp;
#[derive(Tag)]
pub struct Spinner;
