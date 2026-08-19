use crate::animation::*;
use crate::prelude::*;

#[derive(Clone)]
pub enum LedProgram {
  Fixed {
    ids: Box<dyn Contextual<LedIdentifications>>,
    color: ColorSequence,
  },
  Animated {
    ids: Box<dyn Contextual<LedIdentifications>>,
    anim: Box<dyn Animation<Duration, ColorSequence>>,
  },
  Modulated {
    ids: Box<dyn Contextual<LedIdentifications>>,
    color: ColorSequence,
    modulators: MultiModulator<ColorSequence, Duration>,
  },
  Timeline {
    active: bool,
    entries: Vec<TimelineAccumulator>,
  },
}

impl LedProgram {
  /// Accumulate and declare current LED state
  pub fn apply(&mut self, delta: Duration, ctx: &SystemContext) {
    match self {
      LedProgram::Fixed { ids, color } => {
        ctx.declare_leds(ids, color.clone());
      }
      LedProgram::Animated { ids, anim } => {
        anim.accumulate(delta);
        ctx.declare_leds(ids, anim.sample());
      }
      LedProgram::Modulated {
        ids,
        color,
        modulators,
      } => {
        modulators.apply(delta, color);
        ctx.declare_leds(ids, color.clone());
      }
      LedProgram::Timeline { entries, active } => {
        if *active {
          for entry in entries {
            if entry.launched {
              entry.launch();
              entry.program.apply(delta, ctx);
            } else if !entry.launched {
              entry.accumulate_launch(delta);
            }
          }
        }
      }
    }
  }

  pub fn play(&mut self) {
    match self {
      LedProgram::Animated { anim, .. } => {
        anim.play();
      }
      LedProgram::Modulated { modulators, .. } => {
        modulators.play();
      }
      LedProgram::Timeline { active, .. } => *active = true,
      _ => {}
    }
  }

  pub fn stop(&mut self, ctx: &SystemContext) {
    match self {
      LedProgram::Animated { ids, anim, .. } => {
        anim.stop();
        ctx.undeclare_leds(ids);
      }
      LedProgram::Modulated {
        ids, modulators, ..
      } => {
        modulators.stop();
        ctx.undeclare_leds(ids);
      }
      LedProgram::Timeline { active, .. } => *active = false,
      _ => {}
    }
  }

  pub fn reset(&mut self) {
    match self {
      LedProgram::Animated { anim, .. } => {
        anim.reset();
      }
      LedProgram::Modulated { modulators, .. } => {
        modulators.reset();
      }
      LedProgram::Timeline { entries, .. } => {
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
      LedProgram::Fixed { .. } => true,
      LedProgram::Animated { anim, .. } => anim.is_complete(),
      LedProgram::Modulated { modulators, .. } => modulators.is_complete(),
      LedProgram::Timeline { entries, .. } => entries.iter().all(|e| e.completed()),
    }
  }

  pub fn color_mut(&mut self) -> Option<&mut ColorSequence> {
    match self {
      LedProgram::Fixed { color, .. } => Some(color),
      LedProgram::Modulated { color, .. } => Some(color),
      _ => None,
    }
  }

  // -- Constructors --

  /// Keep targets the exact same ColorSequence
  pub fn fixed<T: Contextual<LedIdentifications> + 'static>(
    targets: T,
    color: ColorSequence,
  ) -> Self {
    Self::Fixed {
      ids: Box::new(targets),
      color,
    }
  }

  /// Apply a ColorSequence animation
  pub fn animated<T: Contextual<LedIdentifications> + 'static>(
    targets: T,
    animation: impl Animation<Duration, ColorSequence> + 'static,
  ) -> Self {
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
  /// LedProgram::tween(q, Duration::from_secs(1), Curve::Linear, vec![
  ///   ColorSequence::solid(Rgba::blue()),
  ///   ColorSequence::solid(Rgba::red()),
  /// ])
  ///
  /// // fade between all red to striped red
  /// LedProgram::tween(q, Duration::from_secs(1), Curve::Linear, vec![
  ///   ColorSequence::solid(Rgba::red()),
  ///   ColorSequence::tile(vec![Rgba::red(), Rgba::white()]),
  /// ])
  ///
  /// // "dancing lights" effect
  /// LedProgram::tween(q, Duration::from_secs(1), Curve::Steps(2), vec![
  ///   ColorSequence::tile(vec![Rgba::white(), Rgba::red()]),
  ///   ColorSequence::tile(vec![Rgba::red(), Rgba::white()]),
  /// ])
  /// ```
  pub fn tween<T: Contextual<LedIdentifications> + 'static>(
    targets: T,
    duration: Duration,
    curve: Curve,
    cycle: Cycle,
    colors: Vec<ColorSequence>,
  ) -> Self {
    Self::animated(targets, Tween::new(duration, curve, colors, cycle))
  }

  /// Typical on/off behavior
  pub fn flash<T: Contextual<LedIdentifications> + 'static>(
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
  pub fn breathe<T: Contextual<LedIdentifications> + 'static>(
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
  pub fn initial<T: Contextual<LedIdentifications> + 'static>(
    targets: T,
    initial: ColorSequence,
  ) -> Self {
    Self::Modulated {
      ids: Box::new(targets),
      color: initial,
      modulators: MultiModulator::new(Vec::new(), true),
    }
  }

  /// Add an additional mutation onto the modulation
  pub fn modulate<T: Clone + Send + Sync + 'static>(
    mut self,
    animation: impl Animation<Duration, T> + 'static,
    setter: impl Fn(&mut ColorSequence, T) + Send + Sync + 'static,
  ) -> Self {
    if let LedProgram::Modulated { modulators, .. } = &mut self {
      let modulator = Modulator::<ColorSequence, T, Duration>::new(animation, setter);
      modulators.add(modulator);
    }
    self
  }

  pub fn rotating<T: Contextual<LedIdentifications> + 'static>(
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
          *rotation = Extent::Absolute(angle as i16);
        }
      },
    )
  }
}

#[derive(Clone)]
pub struct TimelineAccumulator {
  target: Duration,
  elapsed: Duration,
  program: LedProgram,
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
