# Plan: Lights

Light management handles RGB LEDs on the EXP bus and lamps on drivers. The rest of this plan makes a single channel of
an LED address a light like any other, so it gets declarations, flashing, z-layers, deactivation with its system, and
scene placement the same way an LED does.

## The model

The `Exp` address grows a single-channel `Channels` case:

```rust
enum Channels {
  /// All three, as one color (an RGB LED)
  Rgb,
  /// One of them, as one brightness
  One(Channel), // R, G or B
}
```

What's wired to an LED address might be one RGB LED or up to three single-channel lights. No two lights share a
`LightAddress`; for single-channel lights, core's boot check makes sure of that.

A light with `Channels::One` shows one brightness, as a driver light does: the largest channel of its color after
compositing over black, so alpha dims it. When lowered, it writes that brightness into its channel of the LED address.

### Kinds, not products

Light management knows lights and the two kinds of address, and nothing about any specific product. Support for
products is built on top:

- **Single-channel flashers**: single-channel lights on a shared LED address, made with a definition helper. They
  need no new code in light management.
- **NeoSeg**:
  - A definition helper turns a digit into its segment lights.
  - Text comes from a separate system holding the character table, which declares on those lights through the public
    API.

## Phases

Each phase builds, passes tests, and leaves the machine behaving as before, except where a phase says otherwise.

### 5. Single-channel lights on an LED address

- **Definition.** `LedDefinition::single` takes a channel (`.channel(Channel::G)`), giving `Channels::One`.
- **A generic helper for 1 to 3 lights on one LED address**, in R, G, B order.
- **Core's boot check** is that no channel is used by two lights, including an `Rgb` light, which uses all three.
  Reject the config with an error naming the LED address and both lights. This is what makes a `LightAddress`
  unique to one light.
- **No channel order with single channels.** Core reorders an LED address's RGB by its `ChannelOrder` when writing
  it, which would move a single-channel light onto another channel. So `One(..)` names a position on the wire, and an
  LED address with a single-channel light must keep the default order. Reject a config that gives it another, naming
  the LED address and the light.
- **The flasher helper** defines a chain of single-channel flashers on a port. The boards take channels one after
  another along the chain, so the helper assigns LED addresses and channels in that order. Only the last LED address
  can be partly used. The rule lives in the helper because it's how flasher boards read the chain, not a rule about
  LED addresses. Other hardware, such as a NeoSeg digit, can leave channels unused mid-port.
- **`TODO.md`.** Remove "Single channel flasher support".

### Later, not in this plan

- **NeoSeg** as its own module. Nothing about it is added to `LightAddress` or `Channels`.
  - **Segments.** Each segment is a single-channel light (`Exp { led, channels: One(..) }`) with a name such as
    `score.3.g` and a position from the segment layout. Scenes see only these lights, so sweeps across the display
    work with nothing NeoSeg-specific.
  - **The definition helper.** It takes a port, a starting index, a digit count and a layout, and produces the
    segment lights and a `SegmentDisplay`: per digit, the light for each segment, in the character table's order.
  - **Channel order.** Brief testing showed the channels out of the expected order, possibly because segments pack
    across LED address boundaries, so one digit starts partway through the previous one's last LED address. The helper
    owns working out LED addresses and channels, so either layout fits. Work out the real order on hardware first.
  - **The text system.** It takes the `SegmentDisplay`, looks each character up in its table, and declares brightness
    on the segment lights through light management.
- **Text from scenes.** Playing text from a scene needs a new layer kind, and `SceneLayer` is a closed enum in
  `frontbox-scene`. Decide then whether that kind is generic ("text on a named character display") or whether scenes
  get a way to add layer kinds from outside.
- **Pulsed driver lights**, such as incandescent flashers on high-voltage drivers. A ball search query narrowed to
  `DriverQ::voltage(DriverVoltage::High)` still matches these, so leaving them out needs a way to exclude drivers
  that have a light, such as a `DriverQ::not`. Lowering one to a pulse instead of a hold is a second way to write a
  `Driver` address.
- **PWM dimming** of driver lights (by reconfiguring hold power).
- **Hysteresis** on the threshold, if a scene shows a lamp chattering.

## Check afterwards

- **Performance of keying by `LightAddress`.** Light management keys its per-frame work by `LightAddress`, an enum,
  rather than by an index. A real machine has about 250 to 350 LEDs. DMD-sized displays don't go through light
  management, and what usually pushes the count higher is cabinet lighting, such as a 3m strip. Once phase 5 is in,
  profile a frame at that size and at a few long strips' worth more. If hashing addresses shows up, give lights an
  index inside `LayeredLightManager`, without changing the trait.
