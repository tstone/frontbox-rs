# Frontbox

An asynchronous, Rust native framework for interacting with Fast pinball hardware.

> [!WARNING]
> Work in progress

### Preview (Subject to Change)

#### 1. Define IO Network

Boards are listed in order connected. For example, if the hardware is layed out as `Neuron => Cabinet IO => Io 3208 => Neuron` then the IO network would be defined as follows. Defining pins over what the board allows will panic on startup.

```rust
let io_network = define_io_network! {
  cabinet: FastIoBoards::cabinet() => {
    switches {
      // label each pin
      0: start_button,
      1: left_flipper_button,
      2: right_flipper_button,
    }
    drivers {
      0: start_button_light,
    }
  },
  playfield: FastIoBoards::io_3208() => {
    switches {
      0: left_outlane,
      4: left_inlane,
    }
    drivers {
      0: left_flipper_main,
      1: left_flipper_hold,
      // ...
    }
  }
};
```

#### 2. Define App Startup

```rust
fn main() {
  env_logger::init();

  App::new()
    // ... Bevy preamble ...
    .add_plugins(Frontbox {
      mainboard_config: MainboardConfig {
        platform: FastPlatform::Neuron,
        io_net_port_path: "/dev/ttyACM0",
        exp_port_path: "/dev/ttyACM1",
        ..Default::default()
      },
      io_network,
    })
    .run();
}
```

#### 3. Spawn Hardware into World

```rust
  // ... as above ...
  .add_systems(Startup, spawn_hardware)

fn spawn_hardware() {
  // TODO
}
```
