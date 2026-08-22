use crate::animation::*;
use crate::prelude::*;

#[derive(Clone)]
pub enum LedProgram1d {
  Fixed {
    ids: Box<dyn Contextual<LedIdentifications> + Send + Sync>,
    color: ColorSequence,
  },
  Animated {
    ids: Box<dyn Contextual<LedIdentifications> + Send + Sync>,
    anim: Box<dyn Animation<Duration, ColorSequence> + Send + Sync>,
  },
  Modulated {
    ids: Box<dyn Contextual<LedIdentifications> + Send + Sync>,
    color: ColorSequence,
    modulators: MultiModulator<ColorSequence, Duration>,
  },
  Timeline {
    active: bool,
    entries: Vec<TimelineAccumulator>,
  },
}

impl LedProgram1d {
  /// Accumulate and declare current LED state
  pub fn apply(&mut self, delta: Duration, ctx: &SystemContext) {
    match self {
      LedProgram1d::Fixed { ids, color } => {
        ctx.declare_leds(ids, color.clone());
      }
      LedProgram1d::Animated { ids, anim } => {
        if anim.is_complete() || !anim.active() {
          ctx.undeclare_leds(ids);
        } else {
          anim.accumulate(delta);
          ctx.declare_leds(ids, anim.sample());
        }
      }
      LedProgram1d::Modulated {
        ids,
        color,
        modulators,
      } => {
        if modulators.is_complete() || !modulators.active() {
          ctx.undeclare_leds(ids);
        } else {
          modulators.apply(delta, color);
          ctx.declare_leds(ids, color.clone());
        }
      }
      LedProgram1d::Timeline { entries, active } => {
        if *active {
          for entry in entries {
            if entry.launched {
              entry.launch();
              entry.program.apply(delta, ctx);
            } else if entry.completed() {
              entry.program.stop(ctx);
            } else if !entry.launched {
              entry.accumulate_launch(delta);
            }
          }
        } else {
          for entry in entries {
            entry.program.stop(ctx);
          }
        }
      }
    }
  }

  pub fn play(&mut self) {
    match self {
      LedProgram1d::Animated { anim, .. } => {
        anim.play();
      }
      LedProgram1d::Modulated { modulators, .. } => {
        modulators.play();
      }
      LedProgram1d::Timeline { active, .. } => *active = true,
      _ => {}
    }
  }

  /// Program starts in a playing state by default. Use this to chain it to prevent that.
  pub fn stopped(mut self) -> Self {
    self.stop_in_place();
    self
  }

  fn stop_in_place(&mut self) {
    match self {
      LedProgram1d::Animated { anim, .. } => {
        anim.stop();
      }
      LedProgram1d::Modulated { modulators, .. } => {
        modulators.stop();
      }
      LedProgram1d::Timeline {
        active, entries, ..
      } => {
        for e in entries.iter_mut() {
          e.program.stop_in_place();
        }
        *active = false;
      }
      _ => {}
    }
  }

  pub fn stop(&mut self, ctx: &SystemContext) {
    match self {
      LedProgram1d::Animated { ids, anim, .. } => {
        anim.stop();
        ctx.undeclare_leds(ids);
      }
      LedProgram1d::Modulated {
        ids, modulators, ..
      } => {
        modulators.stop();
        ctx.undeclare_leds(ids);
      }
      LedProgram1d::Timeline {
        active, entries, ..
      } => {
        for entry in entries {
          entry.program.stop(ctx);
        }
        *active = false
      }
      LedProgram1d::Fixed { ids, .. } => {
        ctx.undeclare_leds(ids);
      }
    }
  }

  pub fn reset(&mut self) {
    match self {
      LedProgram1d::Animated { anim, .. } => {
        anim.reset();
      }
      LedProgram1d::Modulated { modulators, .. } => {
        modulators.reset();
      }
      LedProgram1d::Timeline { entries, .. } => {
        for entry in entries {
          if entry.launched {
            entry.reset();
          }
        }
      }
      _ => {}
    }
  }

  pub fn is_complete(&self) -> bool {
    match self {
      LedProgram1d::Fixed { .. } => true,
      LedProgram1d::Animated { anim, .. } => anim.is_complete(),
      LedProgram1d::Modulated { modulators, .. } => modulators.is_complete(),
      LedProgram1d::Timeline { entries, .. } => entries.iter().all(|e| e.completed()),
    }
  }

  pub fn color_mut(&mut self) -> Option<&mut ColorSequence> {
    match self {
      LedProgram1d::Fixed { color, .. } => Some(color),
      LedProgram1d::Modulated { color, .. } => Some(color),
      _ => None,
    }
  }

  // -- Constructors --

  /// Keep targets the exact same ColorSequence
  pub fn fixed<T: Contextual<LedIdentifications> + Send + Sync + 'static>(
    targets: T,
    color: ColorSequence,
  ) -> Self {
    Self::Fixed {
      ids: Box::new(targets),
      color,
    }
  }

  /// Apply a ColorSequence animation
  pub fn animated<T: Contextual<LedIdentifications> + Send + Sync + 'static>(
    targets: T,
    mut animation: impl Animation<Duration, ColorSequence> + Send + Sync + 'static,
  ) -> Self {
    animation.stop();
    Self::Animated {
      ids: Box::new(targets),
      anim: Box::new(animation),
    }
  }

  /// Cycle (tween) through all given ColorSequences, over `duration`.
  /// For abrupt changes use Curve::Steps(N), where N is the total ColorSequences in the cycle
  ///
  /// ```rust,ignore
  /// // fade everything from red to blue
  /// LedProgram1d::tween(q, Duration::from_secs(1), Curve::Linear, vec![
  ///   ColorSequence::solid(Rgba::blue()),
  ///   ColorSequence::solid(Rgba::red()),
  /// ])
  ///
  /// // fade between all red to striped red
  /// LedProgram1d::tween(q, Duration::from_secs(1), Curve::Linear, vec![
  ///   ColorSequence::solid(Rgba::red()),
  ///   ColorSequence::tile(vec![Rgba::red(), Rgba::white()]),
  /// ])
  ///
  /// // "dancing lights" effect
  /// LedProgram1d::tween(q, Duration::from_secs(1), Curve::Steps(2), vec![
  ///   ColorSequence::tile(vec![Rgba::white(), Rgba::red()]),
  ///   ColorSequence::tile(vec![Rgba::red(), Rgba::white()]),
  /// ])
  /// ```
  pub fn tween<T: Contextual<LedIdentifications> + Send + Sync + 'static>(
    targets: T,
    duration: Duration,
    curve: Curve,
    cycle: Cycle,
    colors: Vec<ColorSequence>,
  ) -> Self {
    Self::animated(targets, Tween::new(duration, curve, colors, cycle))
  }

  /// Typical on/off behavior
  pub fn flash<T: Contextual<LedIdentifications> + Send + Sync + 'static>(
    targets: T,
    color: ColorSequence,
    cycle: Cycle,
  ) -> Self {
    Self::tween(
      targets,
      Duration::from_millis(185),
      Curve::EaseInOut,
      cycle,
      vec![color, ColorSequence::solid(Rgba::default())],
    )
  }

  /// Rhythmic, organic breathing
  pub fn breathe<T: Contextual<LedIdentifications> + Send + Sync + 'static>(
    targets: T,
    color: Rgba<u8>,
    cycle: Cycle,
  ) -> Self {
    Self::tween(
      targets,
      Duration::from_millis(720),
      Curve::EaseInOut,
      cycle,
      vec![
        ColorSequence::solid(color),
        ColorSequence::solid(color.darken(0.325)),
      ],
    )
  }

  /// Apply one or more manual mutations to an initial ColorSequence
  pub fn initial<T: Contextual<LedIdentifications> + Send + Sync + 'static>(
    targets: T,
    initial: ColorSequence,
  ) -> Self {
    Self::Modulated {
      ids: Box::new(targets),
      color: initial,
      modulators: MultiModulator::stopped(Vec::new()),
    }
  }

  /// Add an additional mutation onto the modulation
  pub fn modulate<T: Clone + Send + Sync + 'static>(
    mut self,
    animation: impl Animation<Duration, T> + Send + Sync + 'static,
    setter: impl Fn(&mut ColorSequence, T) + Send + Sync + 'static,
  ) -> Self {
    if let LedProgram1d::Modulated { modulators, .. } = &mut self {
      let modulator = Modulator::<ColorSequence, T, Duration>::new(animation, setter);
      modulators.add(modulator);
    }
    self
  }

  pub fn rotating<T: Contextual<LedIdentifications> + Send + Sync + 'static>(
    targets: T,
    initial: ColorSequence,
    duration: Duration,
    curve: Curve,
    cycle: Cycle,
  ) -> Self {
    Self::initial(targets, initial.rotate(0.0)).modulate(
      Tween::new(duration, curve, vec![0.0, 360.0], cycle),
      |colors, angle| {
        if let Some(alt) = colors.alterations.last_mut()
          && let Some(rotation) = alt.rotation_mut()
        {
          *rotation = Extent::Relative(angle);
        } else {
          log::warn!("LedProgram1: Unexpected - there is no rotation alteration");
        }
      },
    )
  }
}

#[derive(Clone)]
pub struct TimelineAccumulator {
  target: Duration,
  elapsed: Duration,
  program: LedProgram1d,
  launched: bool,
}

impl TimelineAccumulator {
  fn launch(&mut self) {
    self.launched = true;
    self.program.play();
  }

  fn accumulate_launch(&mut self, delta: Duration) {
    if !self.ready() {
      self.elapsed += delta;
    }
  }

  fn reset(&mut self) {
    self.elapsed = Duration::ZERO;
    self.program.reset();
  }

  fn ready(&self) -> bool {
    self.elapsed >= self.target
  }

  fn completed(&self) -> bool {
    self.ready() && self.program.is_complete()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn solid() {
    let mut context = TestContext::default();
    context.insert_system(LedSystem::new());
    let addr = LedAddress::new(ExpAddress::default(), 5);
    context.insert_led(LED {
      name: "led1".to_string(),
      address: addr.clone(),
      ..Default::default()
    });

    let mut program = LedProgram1d::fixed(Q::name("led1"), ColorSequence::solid(Rgba::blue()));
    program.apply(Duration::from_millis(50), &context.sys_ctx());

    let declarations = context
      .sys_ctx()
      .expect::<LedSystem>()
      .declarations_for(&addr);

    assert_eq!(declarations.len(), 1);
    assert_eq!(declarations[0].active, true);
    assert_eq!(declarations[0].color, Rgba::blue());
  }

  #[test]
  fn tile() {
    let mut context = TestContext::default();
    context.insert_system(LedSystem::new());
    let addr1 = LedAddress::new(ExpAddress::default(), 1);
    let addr2 = LedAddress::new(ExpAddress::default(), 2);
    let addr3 = LedAddress::new(ExpAddress::default(), 3);
    context.insert_led(LED {
      name: "led1".to_string(),
      address: addr1.clone(),
      ..Default::default()
    });
    context.insert_led(LED {
      name: "led2".to_string(),
      address: addr2.clone(),
      ..Default::default()
    });
    context.insert_led(LED {
      name: "led3".to_string(),
      address: addr3.clone(),
      ..Default::default()
    });

    let mut program = LedProgram1d::fixed(
      Q::names(vec!["led1", "led2", "led3"]),
      ColorSequence::tile(vec![Rgba::blue(), Rgba::red()]),
    );
    program.apply(Duration::from_millis(50), &context.sys_ctx());

    let declarations = context
      .sys_ctx()
      .expect::<LedSystem>()
      .declarations_for(&addr1);
    assert_eq!(declarations[0].color, Rgba::blue());

    let declarations = context
      .sys_ctx()
      .expect::<LedSystem>()
      .declarations_for(&addr2);
    assert_eq!(declarations[0].color, Rgba::red());

    let declarations = context
      .sys_ctx()
      .expect::<LedSystem>()
      .declarations_for(&addr3);
    assert_eq!(declarations[0].color, Rgba::blue());
  }
}
