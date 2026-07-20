# Frontbox

## Overview

Frontbox is a homebrew arcade framework built for [FAST Pinball](https://fastpinball.com/) hardware, designed around an actor-like constructs called "Systems", which send send and receive signal.

> [!WARNING]
> Frontbox is in active, pre-release development with unstable APIs

### Features

- **Modular**: Almost every facet of machine operation can be replaced
- **Lightweight**: Built in [Rust](https://rust-lang.org/) to run on minimal hardware
- **Coherent**: Limited number of abstractions and consistent architecture throughout
- **Dynamic**: Flexible animation + accumulation system that applies to just about everything
- **Retro**: Pin2DMD and NeoSeg\* (alpha numeric) display support out of the box
- **Immersive**: Sound system with automatic music ducking\*

\* = Coming Soon

## Guide

### Systems

The heart of Frontbox is a `System`. Almost everything is a System: game modes, credit modes, sound mixer, even the display. Systems interact with the world through events. Systems are just Rust structs, which can manage their own state and be extended with private functions. They have a handful of callback type methods, including general lifecycle `on_startup` and `on_shutdown` handlers.

```rust
struct Example {
  private_data: u64,
}

impl System for Example {
  fn on_startup(&mut self, ctx: &Context) {
    // <do cool stuff here>
  }
}
```

Systems include four lifecycle handlers:

- `on_startup`
- `on_deactivate`
- `on_reactivate`
- `on_shutdown`

#### Startup

Systems can be given on startup, and will be started automatically, or dynamically spawned at runtime. Likewise, running systems can be despawned or replaced.

```rust
// Start a new system
ctx.spawn_system(Example::new());

// Stop the current system and immediately spawn a replacement
ctx.replace_self(Example::new());

// Just stop the current system
ctx.despawn_self();
```

#### Types of Systems

- `System` - System which can be started on boot
- `SpawnableSystem` - System which can be dynamically started at runtime. Must be `Send + Sync` compatible
- `ChildSystem` - System which can be managed within a group (see "System Groups" below). Must implement `Clone`.

### Events

Frontbox systems receive events through the `on_event` handler.

```rust
impl System for Example {
  fn on_event(&mut self, event: &dyn Signal, ctx: &Context) { }
}
```

Events are typically handled by attempting a downcast into the expected type.

```rust
impl System for Example {
  fn on_event(&mut self, event: &dyn Signal, ctx: &Context) {
    // detect if the event is of type `SwitchClosed`
    if let Some(e) = event.downcast_ref::<SwitchClosed>() {
      log::debug!("Switch {} was closed!", e.name);
    }

    // simple tests are also possible:
    let is_switch_closed = event.is::<SwitchClosed>();
  }
}
```

Events are both something that the framework provides (e.g. switch open/closed) and something that can be defined by the end user. The only requirement is that values be thread safe (`Send + Sync`).

```rust
// Events can simply be a body-less struct representing a typed thing
pub struct MyCustomThing;

// Events can also contain data
pub struct MyCustomThing2 {
  pub prop1: u8,
  pub prop2: String,
}
pub struct MyTupleLikeThing(i8, i8);
```

#### Emitting Event

Events are broadcast to to all systems. While it's technically possible for every system to emit every event, in practice typically only a small handle of systems emit a particular event.

```rust
ctx.emit(MyCustomThing2 { prop1: 4, prop2: "example".to_string() });

// ...

impl System for Example {
  fn on_event(&mut self, event: &dyn Signal, ctx: &Context) {
    if let Some(custom) = event.downcast_ref::<MyCustomThing2>() {
      log::debug!("Custom thing happened with {}, {}", custom.prop1, custom.prop2);
    }
  }
}
```

##### Event Layering

Sometimes systems receive lower level events (e.g. switch state changed) and process them into higher level events. These higher level events themselves get processed into game level events.

For example...

- The framework might emit a `SwitchClosed` event
- The `Trough` system interprets this and emits `TroughOccupancyChanged` and possibly `TroughFull`
- These trough level events are received by a game manager that may emit `PlayerTurnEnding`.

### Cues

Cues are events that a system can send to itself. There are four primitive types of cues:

1. **Once** -- Cue happens exactly once, after a given amount of time has elapsed
2. **Times** -- Cue happens N times, with an interval in between
3. **Loop** -- Cue happens until canceled, with an interval in between
4. **Now** -- Cue happens immediately, once

```rust
struct SomethingImportant(u8);

impl System for Example {
  fn on_startup(&mut self, ctx: &Context) {
    // setup the cue
    ctx.cue(
      SomethingImportant(100),
      Cue::Times(3, Duration::from_secs(3)),
    )
  }

  fn on_event(&mut self, event: &dyn Signal, ctx: &mut Context) {
    // what to do when the cue happens (in this case, 3 times)
    if let Some(v) = cmd.downcast_ref::<SomethingImportant>() {
      log::debug!("Something important: {}!", v.0);
    }
  }
}
```

#### Handles

Creating a cue returns a handle that can later be used to cancel it.

```rust
let handle = ctx.cue(SomethingRather, Cue::Forever(Duration::from_secs(1)));

ctx.cancel_cue(handle);
```

#### Cycling & Flashing

Cycling through a set of states is a common occurrence in pinball. For example, flashing is in fact the cycling of two values.

```rust
// note the use of `events!` here rather than `vec!`
ctx.cue_cycling(events![
    On("example"),
    Off("example"),
  ],
  Cue::Forever(Duration::from_secs(1))
);
```

Cycling works by rotating through the list of values each time the cue is complete. Think of it like a normal cue that just keeps rotating which signal is emitted, in order. In the example above, 1 second would elapse, then `On("example")` would be cued. Another second would elapse and `Off("example")` would be cued. Another second would elapse and `On("example")` would be cued, and so on.

### Generic Events

In some cases, particularly with cueing, it might be a bit tedious to create a custom type for every little thing that happens. Generally this is preferred, but for insignificant situations the framework provides a few pre-built events that can be used as one-offs:

- `&'static str` - It's possible to use a static string as a signal
- `Action`
- `Anonymous`
- `On` / `Off`

#### Timelines

Sometimes it's easier to express things as a linear timeline. The same example as above could also be expressed as...

```rust
ctx.cue_timeline(CueTimeline::new()
  .cue_at(Duration::from_secs(3), SomethingImportant(5))
  .cue_at(Duration::from_millis(4150), SomethingImportant(50))
  .cue_at(Duration::from_secs(9), SomethingImportant(500))
);
```

Timelines are a way to group a bunch of one-off cues into a sequence. Canceling a timeline cancels all remaining cues within it.

### Context

Each handler receives a reference to `Context`. As this guide has shown, it's through Context that access several features is provided, including:

- Register cues and interrupts
- Emit events
- Access hardware configuration and state

```rust
ctx.switches
ctx.drivers
ctx.io_network
ctx.exp_network
```

### Commands & Services

Systems can choose to expose public (`pub`) functions that are accessible to other systems. Each handler also receives a reference to `Systems` which grants access to this public methods.

```rust
ctx.systems.get::<Trough>().eject();
```

There are several ways to access another system, depending on the situation:

- `get::<S>` - Returns a mutable reference to `Object<S>`
- `expect::<S>` - Returns mutable reference to `S`, also panics if it does not exist
- `get_by_id::<S>` - Returns a mutable reference to `Option<S>`
- `contains::<S>` - Returns `bool` if the system of type `S` currently exists or not

### Active

By default, all systems spawned are active. Systems can be despawned, which removes them entirely, but sometimes it's necessary to keep a system around, having it automatically become active in certain situations. Frontbox supports this feature by way of the `is_active() -> bool` handler.

If `is_active` returns `false`, the framework will by skip all other handlers (the ones starting with `on_*`). Within `is_active`, only read access to `self` and `Context` is provided.

```rust
// a common 'active toggle' is that a system is only active when at least one ball is in play for a player
impl System for Example {
  fn is_active(&self, ctx: &Context) -> bool {
    ctx.is(CurrentPlayerTurnState::Active)
  }
}
```

### Color

Frontbox standardizes on the the `image` crate `Rgba<u8>` color type. This color accounts for four channel colors: red, green, blue, and alpha. All rendering functions within Frontbox account for alpha channel blending.

Colors can be created manually.

```rust
let red = Rgba([255, 0, 0, 255]);
```

Or by using a handful of named colors.

```rust
let red = Rgba::red();
let cyan = Rgba::cyan();
```

Colors can also be modified by lightness, saturation, or hue shifted.

```rust
let c = Rgba::red()
  .lighten(0.4)
  .desaturate(0.1)
  .hue_shift(-15.0)
  .inverted();
```

#### Color Sequence

Generating more than one color is typically done by away of a `ColorSequence`. Colors sequences are not a list of colors, but contain a description of a sequences of colors. This description includes base fill, defined area for the fill, and modifications.

Color sequences can be resolved into a list of colors.

```rust
// turn a color sequence into a list of colors
let seq = ColorSequence::fade(Rgba::red(), Rgba::blue());
let colors: Vec<Rgba<u8>> = seq.generate(6); // generates 6 colors, linearly interpolated from red to blue
```

#### Extents

Because color sequences are _relative_, when specifying numerical values like offsets or lengths, this is done using an `Extent`. Extents can be either absolute or relative. The concrete value is resolved when `generate` is called with a length.

```rust
Extent::relative(0.5) // half way, 50%
Extent::absolute(2) // concretely at index 2
```

##### Fill Types

- **Pattern** - Defines an optionally repeating, fixed pattern. e.g. "red, white, blue three times"
- **Gradient** - Defines a linear fade between N colors

```rust
/// red, white, and blue, three times
ColorSequence::pattern(vec![Rgba::red(), Rgba::white(), Rgba::blue()], Cycle::Times(3));

// multi-gradient
ColorSequence::gradient(vec![
  GradientStop::new(Rgba::red(), Extent::zero()),
  GradientStop::new(Rgba::magenta(), Extent::relative(0.35)),
  GradientStop::new(Rgba::blue(), Extent::full()),
])
```

A handful of convenience construction methods are provided as well:

```rust
// 2 point gradient
ColorSequence::fade(Rgba::red(), Rgba::blue())

// Single pixel forever repeating pattern
ColorSequence::solid(Rgba::red(), Rgba::blue())

// Forever repeating pattern
ColorSequence::tile(vec![Rgba::red(), Rgba::white()])

// Three point gradient with given color as the center point, and hue arc of the given degrees
// This produces a red to orange to yellow gradient
ColorSequence::analogous(Rgba::orange(), 60.0)

// Three point gradient with the given lightness range, with the given color as the center point
// This produces a pink to red to dark red gradient
ColorSequence::monochromatic(Rgba::red(), 0.8)
```

#### Fill Area

Color sequence fills can also be offset or length-constrained and aligned.

```rust
// skip the outer 2 pixels
let seq = ColorSequence::solid(Rgba::red())
  .padded(Extent::absolute(1), Extent:: absolute(1));
let colors = seq.generate(3);
// Result: vec![Rgba::default(), Rgba::red(), Rgba::default()]

// render only half of the total length, center-aligned
let seq = ColorSequence::solid(Rgba::red())
  .anchored(Anchor::Center, Extent::relative(0.5));
let colors = seq.generate(4);
// Result: vec![Rgba::default(), Rgba::red(), Rgba::red(), Rgba::default()]
```

Modifying the fill area is useful for creating progress bar-like effects.

```rust
// red to blue gradient progress bar, left aligned
let seq = ColorSequence::fade(Rgba::red(), Rgba::blue())
  .anchored(Anchor::Left, Extent::relative(percent_complete));
```

#### Color Sequence Modifications

Modifications are chained onto a ColorSequence by way of `modify`.

```rust
let seq = ColorSequence::fade(Rgba::purple(), Rgba::white())
  .modify(Modification::rotated(180.0));
```

##### Modifications

- **Reversed** - Applies color sequence in opposite order
- **Rotated** - Positive degree shifts clockwise, negative degree shifts counter-clockwise
- **Shuffle** - Randomly re-order sequence
- **InnerFill** - Over-write base fill with a child fill

InnerFill can also be used to apply a masking effect, removing some pixels from the sequence.

```rust
let seq = ColorSequence::solid(Rgba::purple())
  .modify(Modification::transparent_at(Extent::absolute(1)));
let colors = seq.generate(3);
// Result: vec![Rgba::red(), Rgba::default(), Rgba::red()]
```

### LEDs

LEDs can be managed in multiple ways. At the lowest level, LEDs can be set by commanding the machine directly. However this skips out on many features. A better choice is to include the bundled `LedPlugin` which adds the `LedSystem` providing the following benefits:

- Conflict Resolution -- Multiple systems can declare a color on the same layer for an LED and the LedSystem will handle resolving that conflict automatically (conflict resolution mode is user settable)
- Layer (z-index) support -- It's possible to keep an "under layer" active while playing temporary animations a layer above
- Alpha Compositing -- LEDs are rendered in RGBA which supports transparency (under colors show through partially)
- An easy way to de-activate LED declarations when a system is de-activated
- Automatic clearing of unset LEDs per frame

#### Hardware Querying

Like other hardware, LEDs can be tagged. See _Tagging & Querying Hardware_ for additional examples.

#### LedSystem

Using the LedSystems works by way of a _declaration_. A declaration doesn't forcibly set an LED, instead it's more like a request, "Hello, I am system 12345 and would prefer for this LED to be this color at this level of priority" (you can think of Z-index layers as levels of priority). Each render frame, the LedSystem looks through all active declarations, chooses the highest priority one, resolves any conflicting declarations, and updates the state of LEDs that need to change. This process also detects LEDs that are no longer set and clears them automatically.

```rust
// declare LEDs by name...
ctx.declare_leds(
  &leds::EXAMPLE.q().at_z(3),
  ColorSequence::solid(Rgba::yellow())
);

// ...or by group
ctx.declare_leds(
  vec![&leds::EX1.q(), &leds::EX2.q(), &leds::EX3.q()],
  ColorSequence::gradient(vec![Rgba::red(), Rgba::yellow()])
);
```

`declare_leds` takes a `HardwareQuery`, which is the reason for `.q()`. More about this later.

Later on if these declarations need to be temporarily suspended because the System is going inactive, they can be temporarily disabled:

```rust
ctx.deactivate_led_declarations();
```

In fact, this behavior is built-in to `System` by default. When a system goes inactive, if `LedSystem` is live, it will de-activate declarations, then re-activate them once the System comes back.

#### Layering

It is possible to declare multiple layers for the same LED. If higher layers are opaque they will be rendered. If higher layers are transparent, they will render with a degree of "see-through" to layers below them.

```rust
// higher layer declares 50% transparent red
ctx.declare_leds(
  &leds::EXAMPLE.q().at_z(1),
  ColorSequence::solid(Rgba::red().with_alpha_f32(0.5))
);

// over top of white
ctx.declare_leds(&leds::EXAMPLE.q(), ColorSequence::solid(Rgba::white()));

// final color renders as pink [255, 127, 127, 255]
```

#### Animations

Animations are a fundamental part of any arcade machine and especially to pinball. Whereas a `Cue` is about an event in time that a system handles, an animation is about a value that changes over time (though not necessarily bound to time). It's useful to establish first what exactly an animation is, before demonstrating how to use it.

Animations describe "how does this value change over an accumulated amount?" Usually the thing being accumulated is time.

```rust
let anim = Tween::new(
  Duration::from_secs(1),
  Curve::Linear,
  vec![0, 100],
  AnimationCycle::Once
);

log::debug!("Current value: {}", anim.sample());
// => "0"

anim.accumulate(Duration::from_millis(500));
log::debug!("Current value: {}", anim.sample());
// => "50"
```

This example describes how a value will start at `0` and end up at `100` over the duration of 1 second. The current value of the animation can be read by sampling it (`.sample()`). Calling `tick` causes time to march forward. Sampling the value of changed time will yield a new value.

#### Ticking Forward

Animations are actually built on a lower level trait called a `Accumulator`. Accumulator are, as the name implies, accumulators of values. When used with `Duration` they accumulate time.

```rust
acc.accumulate(Duration::from_millis(100));
log::debug!("Is complete? {}", acc.is_complete());

acc.reset();
```

Systems have an `on_tick` handler, invoked by the framework, that marches forward based on the framework frequency much like all game frameworks. This internal tick is separate from hardware event handling, which is done in real time. Inactive systems do not tick forward (see "Active" section).

```rust
impl System for Example {
  fn on_tick(&mut self, delta: Duration, ctx: &mut Context) {
    self.anim.tick(delta);
  }
}
```

#### Accumulation

While in the example above the animation was accumulating time by way of `Duration`, it's possible to accumulate anything that is, well, accumulatable. There are a few trait restrictions, like it must have a default value and be comparable (`PartialOrd`), summable, etc. but beyond that any accumulatable value can be accumulated.

This means that animations work, not just on time, by for integers that represent hit counts or switch counts. For example, to change the color of LED based on how many time a spinner has spun, an animation can be used for this.

```rust
// Require 100 hits, animating a from yellow to red
self.anim = Tween::new(
  100, // target
  Curve::Linear,
  vec![Rgba::yellow(), Rgba::red()],
  AnimationCycle::Once
);


fn on_event(&mut self, event: &dyn Signal, ctx: &Context) {
  if let Some(e) = event.downcast_ref::<SwitchClosed>() {
    match e.name {
      switches::SPINNER => {
        let result = self.anim.accumulate(1);
        if result.completed_just_now {
          // do something
        }
      }
    }
  }
}


// elsewhere the animation value can be used to set the LED color (see below)
self.anim.sample()
```

#### Modulators

A modulator combines an accumulator with a setter, mutating a value over time. Like accumulators, these can be used direct if needed, but are generally used through higher level constructors (like LedEffects).

```rust
let modulator = Modulator::new(
  self.anim,
  |value| {  }
)
```

#### LED Animations (Manual)

LEDs colors can of course be combined with animations. This works by accumulating the animation _and_ re-declaring the LED on the same tick.

```rust
pub struct AnimExample {
  anim: Tween<Duration, Color>
}

impl System for AnimExample {
  fn on_tick(&mut self, delta: Duration, ctx: &Context) {
    self.anim.accumulate(delta);

    // re-declaring the same LED will overwrite the previous declaration
    ctx.declare_leds(
      // declare the current animated value as the color of that LED
      &leds::EXAMPLE.q(), ColorSequence::solid(self.anim.sample())
    )
  }
}
```

Any declarable attribute is animatable. For example, a common technique with pinball machines that have 3 or more LEDs for a lane is to use those LEDs to animate a pointing motion. This could be achieved by creating a group of all lane LEDs, then turning one of them on, and animation which one is lit. By giving the declaration a higher z-index, the state of the lane indicators below remains the same, but the animated effect applies "over top of" it.

```rust
pub struct AnimExample {
  // notice the value being animated is a u8, not Color
  anim: Tween<Duration, u8>
}

impl System for AnimExample {
  fn on_tick(&mut self, delta: Duration, ctx: &Context) {
    self.anim.accumulate(delta);

    ctx.declare_leds(
      vec![&leds::LEFT_LANE_ARROW.q(), &leds::LEFT_LANE1.q(), &leds::LEFT_LANE2.q()].at_z(2),
      ColorSequence::pattern(self.anim.sample(), vec![Rgba::red()])
    )
  }
}
```

#### LED Animations (Effects)

A simpler want to manage LED animations is through modulations. A modulation combines an animation with a lens-style setter.

### Sounds

Frontbox includes `SoundSystem` that supports three types of sounds:

##### Effects

- Must be preloaded
- Can play unlimited at a time

##### Callouts

- Must be preloaded
- Can play one at a time
- Overlapping requests will queue
- Automatically lowers volume on music track when playing

##### Music

- Stream from disk
- Can only play one at a time
- Overlapping requests overwrite previous track
- Can crossfade into each other

```rust
let sound_system = ctx.systems.expect::<SoundSystem>();

// typically done `on_startup`
sound_system.preload("name", "/game/assets/sfx/example.wav");
sound_system.preload("multiball", "/game/assets/callouts/multiball.wav");

sound_system.play_sfx("name");
sound_system.play_callout("multiball");
sound_system.play_music("/game/assets/music/track1.mp3");
sound_system.crossfade_music("/game/assets/music/track2.mp3");
```

> [!WARNING]
> This is an evolving feature

### Operator Config

Operator config provides a standard way to read operator-level settings and provide structure to build a menu. Operator configs mainly show up in two places: (1) when declaring hardware properties (e.g. the power of a coil) and (2) for use by systems.

#### Configurable Hardware Values

Many hardware settings actually require a `HardwareValue`. For static values that remain for the life of the program, `HardwareValue::fixed` supports this. But for values that can be configured by the operator config, `HardwareValue::config` will make the value configurable.

```rust
hardware_defs! {
  pub MY_COIL: DriverDefinition = DriverDefinition::new("my_coil")
    .mode(PulseMode {
      trigger_mode: DriverTriggerMode::VirtualSwitchTrue,
      // static value for the life of the program
      initial_pwm_length: HardwareValue::fixed(Duration::from_millis(250)),
      // configurable value that can be adjusted via operator config
      initial_pwm_power: HardwareValue::config(
        "coil_power",  // name
        Power::percent(0.75) // default
        Ranges::power(0.5, 1.0), // domain
      ),
      ..Default::default()
    });
}
```

#### System Config Values

Systems can also register operator config values independent of hardware (e.g. ball count, max extra balls, etc.). This is done through the `config_values` method of `System`.

```rust
pub static MAX_EXTRA_BALLS: LazyLock<ConfigValue<u8, Range<u8>>> = LazyLock::new(|| {
  ConfigValue::new(
    "Max Extra Balls", // name
    "The most extra balls a player can have per game", // description
    5, // default
    Ranges::u8(0, 10),
  )
});

impl System for Example {
  fn config_values(&self) -> Vec<&'static dyn LoadableConfigValue> {
    vec![&*MAX_EXTRA_BALLS]
  }
}
```

#### System Config Registration

- Startup systems have their config values automatically registered
- Dynamically loaded systems must be manually registered

```rust
  App::boot(BootConfig::default()).await
    .configure(|app| {
      // config values will be automatically registered
      app.system(MySystem::new())

      // manually register configs on a system
      app.register_configs(some_system)

      // or manually register explicit configs
      app.register_configs(vec![MY_CONFIG1, MY_CONFIG2])
    })
```

#### Reading Operator Configs

```rust
// The value is always present since a default is provided
let value: u8 = ctx.operator_config.get(MAX_EXTRA_BALLS);
```

### Niche Features

#### Event Interrupts

Sometimes there are cases where the normal flow of operation needs to be halted. For example, if a player drains while ball save is active, this would _normally_ emit an event that the player has drained and the turn is over. In these cases it's necessary to allow a system to override this behavior. This happens by way of event interrupts.

Systems can register themselves as an event interrupt. Interrupt registration requires a priority. The framework will interrupts in priority order (highest first). This allows, for example, a temporary start-of-ball ball save to take precedence over an extra ball or outlane ball save.

Event interrupts can be applied to any event within the system.

```rust
fn on_startup(&mut self, ctx: &Context) {
  ctx.register_interrupt::<TurnEnd>(100); // 100 is the priority
}

fn on_interrupt(&mut self, event: &dyn Signal, ctx: &mut Context) -> InterruptResult {
  // interrupt handlers must return a result
  InterruptResult::Continue // or InterruptResult::Halt
}
```

#### System Groups

System groups are a feature that allows a group of systems to be toggled active or inactive together. This is independent from the `is_active` handler, which is a per-system feature. An entire group can be made inactive, which automatically makes each system within that group no longer receive events. The systems could still be declaring themselves as active. Within a group, the active/inactive nature is actually a combination `group is active && system is active`.

This feature is primarily used by the framework to implement automatic switching of systems based on active player, but it likewise could be used to implement scene switching.

Systems spawned into a group must implement `ChildSystem`, which requires that they be `Clone + Send + Sync`.

```rust
const group_name: &'static str = "example";

// Start an entire group of systems
ctx.spawn_system_group(group_name, vec![/* list of systems */]);

// Groups start deactivated by default
ctx.activate_system_group(group_name);
ctx.deactivate_system_group(group_name);

// The entire group can be despawned. All `on_shutdown` handlers will be invoked for child systems
ctx.despawn_system_group(group_name);
```

### Plugins

TODO: The implementation is minimal and this needs to be flushed out some more.

### Bootstrapping

Frontbox provides an `App` interface which is the root of the framework. With `App`, hardware can be defined and booted.

```rust
App::boot(
  BootConfig {
    platform: FastPlatform::Neuron,
    io_net_port_path: "/dev/ttyACM0",
    exp_port_path: "/dev/ttyACM1",
    ..Default::default()
  },
  io_network,
  expansion_boards,
).await
```

This returns an app builder, which can continue to have systems, plugins, operator configuration, systems, and similar chained onto it before finally running with the initial set of systems.

```rust
App::boot(
    BootConfig::default(),
    io_network,
    expansion_boards,
  )
  .await
  .systems(vec![ ... ]) // add initial systems
  .plugin(CompetitivePlay::default()) // add plugins
  .watchdog_tick(Duration::from_millis(1250)) // configure hardware interactions
  .operator_config_item("required_target_hits", ConfigItem::Integer {
    default: 5,
    min: 3,
    max: 7,
    ..Default::default()
  })
  .run() // call run to begin
  .await
```

#### Hardware Definition

##### Commonalities

Every piece of hardware shares three points of definition in common:

- **name** - Name is a unique, static string identifier that will always refer to that exact piece of hardware. In most cases this is explicitly given, but in same cases (e.g. LED strips) it is automatically generated.
- **tags** - Tags are a way to arbitrarily classify something along any dimension desired (e.g. part of a mech, on the playfield, of a type, etc.)
- **location** - Location specifies where something is in space within a certain context (tag). This location can then be used for spacial effects or bitmap rendering. The most obvious use is `Playfield`, treating all LEDs as a sparse DMD for rendering, but there are other cases where this could be useful as well (backbox, coin door, a custom mech with LEDs, etc.). Because location is associated with a tag, location tags are automatically copied over to **tags**.

Configured hardware moves through four phases within Frontbox:

1. Definition -- Static configuration such as name, tags, location, and any configuration _(user responsibility)_
2. Wiring -- A definition is assigned to a specific pin on a specific board _(user responsibility)_
3. Addressable -- A wired definition is automatically resolved to it's absolute address (id) on the network _(framework responsibility)_
4. Stateful -- Some hardware (e.g. switches) also become stateful, keeping track of things like open/closed state _(framework responsibility)_

Generally, static **definitions** should be available for reference throughout the app. Stateful representations, where applicable, are available through `Context`.

##### I/O Network

The I/O network is defined using the `IoNetworkBuilder`. See [Defining Hardware]() guide for more details. I/O network devices can either associate a name with a pin, or can optionally provide a configuration. Configurations given here are automatically applied at startup.

Hardware is defined on a board by specifying it's pin, `switch(3)` and giving it a name `switch(3).named("foo")`. It is a good idea to declare names as constants, wrapped in a module for easy access.

Hardware can also be tagged `.tagged(Playfield)`. This serves to _classify_ something about the switch, possibly location or purpose. This makes it easy to implement modes that need to say things like "if any playfield switch has been hit, then...". These tags are arbitrary. Frontbox comes with several, but they can be user-defined as well.

Lastly, depending on the type of hardware being defined, an optional config (`.config(...)`) or mode (`.mode(...)`) can be given.

```rust
// Step 1. Define hardware

pub mod hw {
  use super::*;

  pub left_inlane_switch: SwitchDefinition = SwitchDefinition::new("linlane")
    .tag(Playfield);

  pub left_outlane_switch: SwitchDefinition = SwitchDefinition::new("loutlane")
    .tag(Playfield)
    .tag(Drain)
    .inverted()
    .debounce_open(Duration::from_millis(40));

  pub trough_eject_coil: DriverDefinition = DriverDefinition::new("trough_eject").build();

  pub shooter_coil: DriverDefinition = DriverDefinition::new("shooter")
    .tag(AutoPlungeCoil)
    .mode(PulseMode {
      trigger_mode: DriverTriggerMode::VirtualSwitchTrue,
      initial_pwm_power: Power::FULL,
      ..Default::default()
    });
}

// Step 2. Define and wire the network
//
// NOTE: order matters here. Boards must be listed in the order they appear on the network
// e.g. Neuron => IO3208 => IO1616 => Neuron would be defined as:

let io_network = IoNetwork::new(vec![
  IoBoards::io_3208()
    .wire_switch(3, &hw::left_inlane)
    .wire_switch(4, &hw::left_outlane)
    .wire_driver(0, &hw::trough_eject_coil)
    .wire_driver(1, &hw::shooter_coil),
  IoBoards::io_1616(),
]);
```

##### Expansion Network

The expansion network is defined using the `ExpansionNetworkBuilder`. See [Defining Hardware]() guide for more details.

```rust
// Step 1. Define exp devices

pub mod leds {
  use super::*;

  hardware_defs! {
    pub LEFT_INLANE: LedDefinition = LedDefinition::single("linlane")
      .location(Vec2::new(3.4, 32.5).relative_to(PLAYFIELD))
      .channels(ColorChannels::GRBW);

    pub LEFT_OUTLANE: LedDefinition = LedDefinition::new("loutlane")
      .location(Vec2::new(2.125, 32.5).relative_to(PLAYFIELD));

    // Cabinet lighting along the left art blade area
    pub LEFT_CAB_STRIP: LedDefinition = LedDefinition::strip("lcab", 32)
      .tag(Cabinet)
      .locations(CabinetLeft, 15.0, (10.25, 0), (48.5, 3.0));
  }
}

// Step 2. Define boards and wire the network

let exp_network = ExpansionNetwork::new(vec![
  ExpansionBoard::fp_exp0061()
    .wire_led_port(0, LedPort::ws2812b().leds(vec![
        &leds::LEFT_INLANE,
        &leds::LEFT_OUTLANE,
      ]
    )),
    .wire_led_port(1, LedPort::wS2812b().leds(vec![&leds::LEFT_CAB_STRIP]))
]);
```

##### Hardware Organization

Despite the examples in this document showing everything defined all together, it usually makes more sense to group all definitions by region or mech, then wire the network in the same spot. For example, a game might be setup to have regions like `hardware/lower_thirds.rs`, `hardware/mid_field.rs`, and `hardware/upper_playfield.rs`, then a separate `io_network.rs` and `exp_network.rs` which references those hardware definitions and wires up the network.

Depending on the complexity of the playfield, it might also be useful to group regions into modules.

```rust
// upper_playfield.rs

pub const left_ramp_switch = ...;
pub const right_ramp_switch = ...;

pub mod custom_mech {
  pub const entrance_switch = ...;
  pub const kicker_coil = ...;
}
```

### Tagging & Querying Hardware

Hardware queries are a way to describe what hardware to use.

```rust
// select by name
let q = Q::name("foo");
let q = Q::names(vec!["foo", "bar"]);
let q = Q::name(left_inlane_led.name); // names can be referenced from definitions
let q = Q::name(left_cabinet_strip.name) // LED groups can be referenced by name
let q = Q::name(left_cabinet_strip.child(0).name) // Specific children within LED groups can be referenced

// select by tag (see hardware definition below for more on tagging)
let q = Q::tag::<Playfield>();

// select things in a location
let q = Q::location::<LowerThirds>();

// multiple criteria
let q = Q::name("start_button").or(Q::tag::<StartButton>());
let q = Q::location::<LowerThirds>().and(Q::tag::<Drain>()); // both criteria must be met

// masking/exclusions
// select everything that is not tagged Playfield
let q = Q::not(Q::tag::<Playfield>);
// select everything tagged Playfield that is not also tagged Target
let q = Q::tag::<Playfield>().not(Q::tag::<Target>());

// other
let q = Q::all(); // select everything of this type
let q = Q::rand(10); // take 10 random, even if there are more
let q = Q::tag::<Playfield>().rand(10);
```

Queries are just a description of hardware and don't contain the reference to matching hardware. However they can be used a predicate with an event or given `Context` to resolve into the matching hardware.

#### Query as Predicate

```rust
// Get all matching switches (resolve query to hardware reference)
let switches = ctx.switches.query(q);

// ...

fn on_event(&mut self, event: &dyn Event, ctx: &Context) {
  if let Some(e) = event.downcast_ref::<SwitchClosed>() {
    // use as a predicate
    if q.matches_switch(e.switch) {
      // ...
    }
  }
}
```

All of the included Systems and Plugins use selections, and most all include sane defaults. For example, to require payment in order to play, the `CreditsPlay` could be spun up as...

```rust
CreditsPlay::new(
  Q::name(switches::START_BUTTON),
  Q::names(vec![switches::COIN1, switches::COIN2]),
)
```

This _explicitly_ sets which switches to listen to based on IDs. Alternately just using default will use the tags you'd most expect. This latter approach follows more of a convention over configuration. Tag your hardware with all the tags that make sense and most things should just work™.

```rust
CreditsPlay::default()
// switches tagged with StartButton are used as the start button
// switches tagged with CoinDrop are used to identify incoming coins
```

## Roadmap

1. Playing sounds/music
2. Accumulations
3. Expansion network definition re-write
4. LED system revamp: Use new cueing + command systems; support led groups
5. NeoSeg support

## Complete Example System

This system implements a basic pinball "mode". A target is illuminated and must be struck 3 times. Each hit grants 1000 points. After 3 hits, the target will begin flashing. The player has 20 seconds to hit it again for 25,000 points (hurry up shot). After 20 seconds or being hit a 4th time the mode resets.

- `SwitchClosed` event monitors the target's switch
- `ctx.set_timer` and `TimerComplete` event monitors the hurry up timer
- `self.hurry_up_active` and `self.hits` manage state
- `fn leds` sets the LED state for the framework to apply (declarative)

```rust
const HURRY_UP_TIMER: &'static str = "hurry_up";

struct TargetHitter {
  // current times this target has been hit
  hits: u8,
  // animation for bonus hit
  flash_anim: Box<dyn Animation<Color>>,
  state: TargetHitterState,
  // ids for target switch and LED indicator
  target_switch_id: &'static str,
  indicator_id: &'static str,
}

enum TargetHitterState {
  // waiting to get to the desired number of hits
  Building,
  // bonus hurry-up mode for extra points
  HurryUp
}

impl TargetHitter {
  pub fn new(target_switch_id: &'static str, indicator_id: &'static str) -> Box<Self> {
    Box::new(Self {
      target_switch_id,
      indicator_id,
      hits: 0,
      state: TargetHitterState::Building,
      flash_anim: InterpolationAnimation::new(
        Duration::from_millis(450),
        Curve::ExponentialInOut,
        vec![Rgba::black(), Rgba::red()],
        AnimationCycle::Forever,
      )
    })
  }

  fn reset(&mut self) {
    self.hits = 0;
    self.hurry_up_active = false;
    self.flash_anim.reset();
  }

  // Here's what happens when the target is it -- if the mode is in "hurry up"
  fn on_target_hit(&mut self, ctx: &Context) {
    let game_manager = ctx.systems.expect::<GameManager>();

    match self.state {
      TargetHitterState::HurryUp => {
        game_manager.add_points(25_000, ctx);
        self.on_hurry_up_done();
      }
      TargetHitterState::Building => {
        self.hits = self.hits.saturating_add(1);
        game_manager.add_points(1000, ctx);

        if self.hits == 3 {
          self.hurry_up_active = true;
          cmds.set_timer(HURRY_UP_TIMER, Duration::from_secs(20), TimerMode::Once);
        }
      }
    }
  }

  fn on_hurry_up_done(&mut self) {
    self.reset();
  }
}

impl System for TargetHitter {
  fn on_event(&mut self, event: &dyn Signal, ctx: &Context) {
    if let Some(event) = event.downcast::<SwitchClosed>() {
      if event.switch.id == self.target_switch_id {
        self.on_target_hit(ctx);
      }
    }
  }

  fn on_timer(&mut self, name: &'static str, _ctx: &mut Context) {
    if event.name == HURRY_UP_TIMER {
      self.on_hurry_up_done();
    }
  }

  fn leds(&mut self, delta_time: Duration, _ctx: &Context) -> LedStates {
    // show the flashing state if hurry up is active otherwise use a static color
    match self.state {
      TargetHitterState::HurryUp => {
        LedDeclarationBuilder::new(delta_time)
          .next_frame(self.flash_anim)
          .collect()
      }
      TargetHitterState::Building => {
        let color = match self.hits {
          0 => Rgba::yellow(),
          1 => Rgba::orange(),
          2 => Rgba::red(),
        }
        LedDeclarationBuilder::new(delta_time)
          .on(self.indicator_id, color)
          .collect()
      }
    }
  }
}
```

See [examples](frontbox/examples) or the [included plugins](frontbox-rs/tree/main/frontbox-turn-based/src) for more.
