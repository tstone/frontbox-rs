# Frontbox

An asynchronous, Rust native framework for interacting with Fast pinball hardware.

> [!WARNING]
> Work in progress

### Preview (Subject to Change)

#### 1. Define Your IO network

```rust
let cabinet = FastBoard::CabinetIO::new();
let playfield_front = FastBoard::IO3208::new();
let playfield_rear = FastBoard::IO1616::new();

let io_net = IoNetwork.define([ cabinet, playfield_front, playfield_rear ]);
```

#### 2. Define Your Devices

```rust
let left_sling = io_net.define_device(
  Slingshot {
    // port assignments are relative to the board, not the IO network
    // reconfiguring network order does not affect device port assignments
    driver: playfield_front.drivers(0),
    // these assignments always match what is silkscreened on the PCB
    switch: playfield_front.switches(31),
    ..default()
  }
)

// ...

let left_flipper = io_net.define_device(
  // more complex devices can handle multiple coils
  DualWoundFlipper {
    main_driver: playfield_front.drivers(1),
    hold_driver: playfield_front.drivers(2),
    eos_switch: playfield_front.switches(30),
    flipper_button: cabinet.switches(15)
    ..default()
  }
)
```

#### 3. Run the IO loop

```rust
#[tokio::main]
async fn main() {
  env_logger::init();
  let mut neuron = Mainboard::new(MainboardConfig {
    platform: FastPlatform::Neuron,
    io_net,
    ..default()
  })
  neuron.run().await;
}
```
