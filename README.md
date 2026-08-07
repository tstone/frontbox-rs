![Frontbox](logo.png)

## Overview

Frontbox is a homebrew arcade framework built for [FAST Pinball](https://fastpinball.com/) hardware, designed around an actor-like constructs called "Systems", which send and receive signal.

> [!WARNING]
> Frontbox is in active, pre-release development with unstable APIs

### Features

- **Lightweight**: Built in [Rust](https://rust-lang.org/) to run on minimal hardware
- **Flexible**: All hardware can be referenced in 3 ways: by name, tag, or location in space
- **Dynamic**: Flexible animation + LED support out of the box
- **Retro**: Pin2DMD and NeoSeg\* (alpha numeric) display support out of the box
- **Sonically Immersive**: Sound system with preloading and automatic music ducking\*

\* = Coming Soon

## Guides

- App -- Booting things up
- Hardware -- Defining what exists
- Systems -- The behavorial unit everything happens within
- Drivers -- Turning things like coils on and off
- Switches -- Capturing input
- LEDs -- Lighting things up
- DMD -- Dot matrix display rendering

## Documentation
Documentation is embedded within the source and is available via rustdoc: `cargo doc --open`.

## Examples
- See included [examples](frontbox/examples)
- Or [Last of the Kilmore Oaks](https://github.com/tstone/lotko-homebrew) for source of a full game

## License
Frontbox is dual licensed Apache + MIT.

## AI Usage

This project generally follows the [Rust language LLM use policy](https://forge.rust-lang.org/policies/llm-usage.html).

- It’s fine to use LLMs to answer questions, analyze, distill, refine, check, suggest, review. But not to **create**.
- LLMs work best when used as a tool to write _better_, not _faster_.

**Design ("style") should be human-driven.** Frontbox is not "vibe-coded".
