//! Layers drawn together, one over another

use glam::Vec2;
use image::Rgba;

use crate::animation::{Curve, Cycle};
use crate::color::RgbaColor;
use crate::layout::{Grid, Line};
use crate::scene::test_scene::Scene;
use crate::scene::*;

const SIZE: Vec2 = Vec2::new(100.0, 50.0);
const RED: Rgba<u8> = Rgba([255, 0, 0, 255]);
const BLUE: Rgba<u8> = Rgba([0, 0, 255, 255]);
const DARK: Rgba<u8> = Rgba([16, 16, 16, 255]);

fn solid(color: Rgba<u8>) -> Fill {
  Fill::new(FillNode::Solid(Color(color).into()))
}

fn red_to_blue() -> Fill {
  Fill::new(FillNode::Gradient {
    stops: vec![Stop::new(0.0, Color(RED)), Stop::new(1.0, Color(BLUE))],
    style: GradientStyle::Linear,
  })
}

fn sample(scene: &Scene, x: f32, y: f32, radius: f32) -> Rgba<u8> {
  let params = Params::new();
  let frame = LayerFrame { time: 0.0, playback_seed: 0, params: &params, assets: Assets::none(), size: SIZE, surface: Surface::Area, visits: None, timing: None, defaults: no_params() };
  scene.sample(&Sample::new(Vec2::new(x, y), radius), &frame)
}

fn no_timing(layer: &str) -> SceneError {
  SceneError::NoTiming { layer: layer.into() }
}

#[test]
fn a_scene_that_cant_play_says_which_layer_is_why() {
  let changing = || solid(RED).adjust(ColorAdjust::HueShift(Prop::Keyframes(Keyframes { keys: vec![], cycle: Cycle::Once })));
  let scene = |top: Layer| Scene::new(Surface::Area, vec![Layer::new(solid(RED)), top]);

  let error = scene(Layer::new(changing()).name("sparkle")).check().unwrap_err();
  assert_eq!(error, no_timing("layer \"sparkle\""));
  assert_eq!(error.to_string(), "layer \"sparkle\" changes over time, so the scene needs a timing");
  assert_eq!(scene(Layer::new(changing())).check(), Err(no_timing("layer 2")));
}

#[test]
fn a_hidden_layer_isnt_drawn_and_keeps_its_place() {
  let scene = Scene::new(Surface::Area, vec![Layer::new(solid(RED)), Layer::new(solid(BLUE)).hidden()]);
  assert_eq!(sample(&scene, 1.0, 1.0, 0.5), RED);

  let params = Params::new();
  let frame = LayerFrame { time: 0.0, playback_seed: 0, params: &params, assets: Assets::none(), size: SIZE, surface: Surface::Area, visits: None, timing: None, defaults: no_params() };
  assert_eq!(scene.outlines(&frame).len(), 2);
}

#[test]
fn a_layer_with_no_shape_covers_the_viewport_and_no_further() {
  let scene = Scene::new(Surface::Area, vec![Layer::new(solid(RED))]);
  assert_eq!(sample(&scene, 100.0, 50.0, 0.5), RED);
  assert_eq!(sample(&scene, 100.5, 25.0, 0.5), Rgba([0, 0, 0, 0]));
  assert_eq!(sample(&scene, 50.0, -0.5, 0.5), Rgba([0, 0, 0, 0]));
}

#[test]
fn a_shape_runs_past_the_viewport() {
  let shape = Shape::new(ShapeNode::circle(10.0)).transform(Transform::Translate(Vec2::new(150.0, 25.0).into()));
  let scene = Scene::new(Surface::Area, vec![Layer::new(solid(RED)), Layer::new(solid(BLUE)).region(shape)]);
  assert_eq!(sample(&scene, 150.0, 25.0, 0.5), BLUE);
  assert_eq!(sample(&scene, 170.0, 25.0, 0.5), Rgba([0, 0, 0, 0]));
}

fn turned(degrees: f32) -> Scene {
  Scene::new(Surface::Sequence, vec![Layer::new(red_to_blue().transform(Transform::Rotate(degrees.into())))])
}

#[test]
fn rotate_runs_a_sequence_around_a_point_at_a_time() {
  let still = line(&turned(0.0), 8);
  let mut around = still.clone();
  around.rotate_right(1);
  assert_eq!(line(&turned(45.0), 8), around);
  around.rotate_left(2);
  assert_eq!(line(&turned(-45.0), 8), around);
}

#[test]
fn a_lap_of_a_sequence_comes_back_to_the_start() {
  assert_eq!(line(&turned(360.0), 8), line(&turned(0.0), 8));
  assert_eq!(line(&turned(-720.0), 8), line(&turned(0.0), 8));
}

#[test]
fn rotate_runs_a_shape_past_the_end_of_a_sequence_to_its_start() {
  let lit = |transform: Vec<Transform>| -> Vec<bool> {
    let shape = transform.into_iter().fold(Shape::new(ShapeNode::Rect { width: 0.1.into(), height: 1.0.into() }), Shape::transform);
    line(&Scene::new(Surface::Sequence, vec![Layer::new(solid(RED)).region(shape)]), 5).iter().map(|color| color[3] > 0).collect()
  };
  let place = || Transform::Translate(Vec2::new(0.75, 0.0).into());
  let turn = |degrees: f32| Transform::Rotate(degrees.into());

  assert_eq!(lit(vec![place(), turn(72.0)]), [false, false, false, false, true]);
  assert_eq!(lit(vec![place(), turn(144.0)]), [true, false, false, false, false]);
  assert_eq!(lit(vec![turn(144.0), place()]), [true, false, false, false, false]);
}

#[test]
fn a_shape_rotating_on_an_area_turns_rather_than_runs_around() {
  let bar = Shape::new(ShapeNode::Rect { width: 40.0.into(), height: 2.0.into() })
    .transform(Transform::Rotate(90.0.into()))
    .transform(Transform::Translate(Vec2::new(50.0, 25.0).into()));
  let scene = Scene::new(Surface::Area, vec![Layer::new(solid(RED)).region(bar)]);
  assert_eq!(sample(&scene, 50.0, 10.0, 0.1), RED);
  assert_eq!(sample(&scene, 65.0, 25.0, 0.1)[3], 0);
}

#[test]
fn a_shape_moved_then_rotated_turns_where_it_is() {
  let bar = Shape::new(ShapeNode::Rect { width: 40.0.into(), height: 2.0.into() })
    .transform(Transform::Translate(Vec2::new(50.0, 25.0).into()))
    .transform(Transform::Rotate(90.0.into()));
  let scene = Scene::new(Surface::Area, vec![Layer::new(solid(RED)).region(bar)]);
  assert_eq!(sample(&scene, 50.0, 10.0, 0.1), RED);
  assert_eq!(sample(&scene, 65.0, 25.0, 0.1)[3], 0);
}

#[test]
fn a_polygon_away_from_the_origin_turns_around_its_own_middle() {
  let bar = ShapeNode::Polygon {
    points: [(10.0, 4.0), (30.0, 4.0), (30.0, 6.0), (10.0, 6.0)].map(|(x, y)| Vec2::new(x, y).into()).to_vec(),
  };
  let shape = Shape::new(bar).transform(Transform::Rotate(90.0.into()));
  let scene = Scene::new(Surface::Area, vec![Layer::new(solid(RED)).region(shape)]);
  assert_eq!(sample(&scene, 20.0, 12.0, 0.1), RED);
  assert_eq!(sample(&scene, 12.0, 5.0, 0.1)[3], 0);
}

fn line(scene: &Scene, count: usize) -> Vec<Rgba<u8>> {
  let params = Params::new();
  let frame = LayerFrame { time: 0.0, playback_seed: 0, params: &params, assets: Assets::none(), size: SIZE, surface: Surface::Area, visits: None, timing: None, defaults: no_params() };
  Line::new(count).samples().map(|s| scene.sample(&s, &frame)).collect()
}

/// A scene a second long whose color is picked at the start of each cycle
fn random_color(cycle: Cycle) -> Scene {
  let spread = Spread::Color { hue: (0.0, 360.0), brightness: (1.0, 1.0), alpha: (1.0, 1.0) };
  let color = Prop::Random(Random { picks: vec![Pick::new(0.0)], spread, seed: 1, force_unique: true });
  Scene::new(Surface::Sequence, vec![Layer::new(Fill::new(FillNode::Solid(color)))]).timing(Timing::new(1.0, cycle))
}

fn random_color_at(scene: &Scene, time: f32, playback_seed: u32) -> Rgba<u8> {
  let params = Params::new();
  let frame = LayerFrame { time, playback_seed, params: &params, assets: Assets::none(), size: SIZE, surface: Surface::Area, visits: None, timing: None, defaults: no_params() };
  scene.sample(&Sample::new(Vec2::new(0.5, 0.0), 0.5), &frame)
}

#[test]
fn a_scene_that_repeats_has_a_cycle_either_side_of_its_middle() {
  let middle = |cycle| Scene::new(Surface::Sequence, vec![]).timing(Timing::new(1.0, cycle)).middle_cycle();
  assert_eq!(middle(Cycle::Forever).timing.unwrap().cycle, Cycle::Times(3));
  assert_eq!(middle(Cycle::Times(2)).timing.unwrap().cycle, Cycle::Times(3));
  assert_eq!(middle(Cycle::Forever).middle_cycle_time(0.5), 1.5);
}

#[test]
fn a_scene_of_one_cycle_stays_as_it_is_in_its_middle() {
  for cycle in [Cycle::Once, Cycle::Times(1)] {
    let scene = Scene::new(Surface::Sequence, vec![]).timing(Timing::new(1.0, cycle)).middle_cycle();
    assert_eq!(scene.timing.unwrap().cycle, cycle);
    assert_eq!(scene.middle_cycle_time(0.5), 0.5);
  }
}

#[test]
fn the_end_of_a_middle_cycle_is_still_that_cycle() {
  let scene = Scene::new(Surface::Sequence, vec![]).timing(Timing::new(0.1, Cycle::Forever)).middle_cycle();
  assert_eq!(scene.timing.unwrap().cycle_index(scene.middle_cycle_time(0.1)), 1);
}

#[test]
fn a_time_in_a_cycle_stays_in_that_cycle() {
  for duration in [0.1, 0.7, 2.0] {
    let timing = Timing::new(duration, Cycle::Forever);
    let scene = Scene::new(Surface::Sequence, vec![]).timing(timing);
    for cycle in 0..1000 {
      assert_eq!(timing.cycle_index(scene.cycle_time(cycle, 0.0)), cycle, "start of {cycle} of {duration}");
      assert_eq!(timing.cycle_index(scene.cycle_time(cycle, duration)), cycle, "end of {cycle} of {duration}");
    }
  }
}

#[test]
fn a_cycle_past_the_last_is_the_last() {
  let scene = Scene::new(Surface::Sequence, vec![]).timing(Timing::new(1.0, Cycle::Times(3)));
  assert_eq!(scene.cycle_time(7, 0.5), 2.5);
}

#[test]
fn a_scene_of_one_cycle_has_only_that_one() {
  let scene = Scene::new(Surface::Sequence, vec![]).timing(Timing::new(1.0, Cycle::Once));
  assert_eq!(scene.cycle_time(3, 1.0), 1.0);
}

#[test]
fn random_picks_smooth_into_the_next_cycle() {
  let smoothed = |cycle| {
    let spread = Spread::Float(0.0, 360.0);
    let pick = Pick { at: 0.0, smooth_in: 1.0, smooth_out: 0.0 };
    let shift = Prop::Random(Random { picks: vec![pick], spread, seed: 1, force_unique: true });
    Scene::new(Surface::Sequence, vec![Layer::new(solid(RED).adjust(ColorAdjust::HueShift(shift)))])
      .timing(Timing::new(1.0, cycle))
  };
  let looping = smoothed(Cycle::Forever);
  assert_ne!(random_color_at(&looping, 1.25, 0), random_color_at(&looping, 1.75, 0));
  let once = smoothed(Cycle::Once);
  assert_eq!(random_color_at(&once, 0.25, 0), random_color_at(&once, 0.75, 0));
}

#[test]
fn random_picks_change_each_cycle_and_hold_within_one() {
  let scene = random_color(Cycle::Forever);
  assert_eq!(random_color_at(&scene, 0.1, 0), random_color_at(&scene, 0.9, 0));
  assert_ne!(random_color_at(&scene, 0.9, 0), random_color_at(&scene, 1.1, 0));
  assert_eq!(random_color_at(&scene, 1.1, 0), random_color_at(&scene, 1.9, 0));
}

#[test]
fn a_take_is_the_same_each_time_and_differs_from_another() {
  let scene = random_color(Cycle::Forever);
  assert_eq!(random_color_at(&scene, 0.5, 3), random_color_at(&scene, 0.5, 3));
  assert_ne!(random_color_at(&scene, 0.5, 3), random_color_at(&scene, 0.5, 4));
}

#[test]
fn a_completed_scene_holds_its_last_pick() {
  let scene = random_color(Cycle::Times(2));
  assert_eq!(random_color_at(&scene, 1.9, 0), random_color_at(&scene, 2.0, 0));
  assert_eq!(random_color_at(&scene, 1.9, 0), random_color_at(&scene, 30.0, 0));
}

#[test]
fn random_props_need_a_timing() {
  let mut scene = random_color(Cycle::Once);
  scene.timing = None;
  assert_eq!(scene.check().unwrap_err(), no_timing("layer 1"));
}

/// A circle of diameter 40 at (50, 25) in a 100 x 50 viewport, over a dark background
fn circle(fill: Fill) -> Scene {
  Scene::new(Surface::Area, vec![
    Layer::new(solid(DARK)),
    Layer::new(fill).region(
      Shape::new(ShapeNode::circle(0.5))
        .transform(Transform::Scale(40.0.into()))
        .transform(Transform::Translate(Vec2::new(50.0, 25.0).into())),
    ),
  ])
}

fn outlines(layer: Layer) -> Vec<Vec<Vec2>> {
  let params = Params::new();
  let frame = LayerFrame { time: 0.0, playback_seed: 0, params: &params, assets: Assets::none(), size: SIZE, surface: Surface::Area, visits: None, timing: None, defaults: no_params() };
  Scene::new(Surface::Area, vec![layer]).outlines(&frame).remove(0)
}

#[test]
fn a_device_decides_the_size_of_an_area() {
  let scene = Scene::new(Surface::Area, vec![Layer::new(solid(RED))]);
  let params = Params::new();
  let at = Sample::new(Vec2::new(150.0, 25.0), 0.0);
  let color = |size: Vec2| scene.sample(&at, &LayerFrame { time: 0.0, playback_seed: 0, params: &params, assets: Assets::none(), size, surface: Surface::Area, visits: None, timing: None, defaults: no_params() });
  assert_eq!(color(Vec2::new(200.0, 50.0)), RED);
  assert_eq!(color(SIZE), Rgba([0, 0, 0, 0]));
}

#[test]
fn a_gradient_with_no_shape_stretches_to_the_device() {
  let scene = Scene::new(Surface::Area, vec![Layer::new(red_to_blue())]);
  let params = Params::new();
  let far_edge = |width: f32| {
    let frame = LayerFrame { time: 0.0, playback_seed: 0, params: &params, assets: Assets::none(), size: Vec2::new(width, 50.0), surface: Surface::Area, visits: None, timing: None, defaults: no_params() };
    scene.sample(&Sample::new(Vec2::new(width, 25.0), 0.0), &frame)
  };
  assert_eq!(far_edge(100.0), BLUE);
  assert_eq!(far_edge(300.0), BLUE);
}

#[test]
fn a_layer_with_no_shape_outlines_the_viewport() {
  let corners = [(0.0, 0.0), (100.0, 0.0), (100.0, 50.0), (0.0, 50.0)].map(|(x, y)| Vec2::new(x, y));
  assert_eq!(outlines(Layer::new(solid(RED))), [corners]);
}

#[test]
fn an_outline_follows_a_shapes_transforms_in_order() {
  let square = ShapeNode::Polygon {
    points: [(0.0, 0.0), (1.0, 0.0), (1.0, 2.0), (0.0, 2.0)].map(|(x, y)| Vec2::new(x, y).into()).to_vec(),
  };
  let shape = Shape::new(square)
    .transform(Transform::Scale(2.0.into()))
    .transform(Transform::Translate(Vec2::new(10.0, 20.0).into()));
  let corners = [(10.0, 20.0), (12.0, 20.0), (12.0, 24.0), (10.0, 24.0)].map(|(x, y)| Vec2::new(x, y));
  assert_eq!(outlines(Layer::new(solid(RED)).region(shape)), [corners]);
}

#[test]
fn an_ellipse_outline_runs_around_its_edge() {
  let shape = Shape::new(ShapeNode::Ellipse { rx: 0.5.into(), ry: 0.25.into() })
    .transform(Transform::Scale(40.0.into()))
    .transform(Transform::Translate(Vec2::new(50.0, 25.0).into()));
  let outline = outlines(Layer::new(solid(RED)).region(shape)).remove(0);
  assert!(outline.len() > 16);
  for point in outline {
    let offset = (point - Vec2::new(50.0, 25.0)) / Vec2::new(20.0, 10.0);
    assert!((offset.length() - 1.0).abs() < 1e-4, "{point}");
  }
}

#[test]
fn a_shape_that_draws_nothing_has_no_outline() {
  let collapsed = Shape::new(ShapeNode::circle(0.5)).transform(Transform::Scale(0.0.into()));
  assert!(outlines(Layer::new(solid(RED)).region(collapsed)).is_empty());
  let line = Shape::new(ShapeNode::Polygon { points: vec![Vec2::ZERO.into(), Vec2::ONE.into()] });
  assert!(outlines(Layer::new(solid(RED)).region(line)).is_empty());
}

#[test]
fn solid_on_a_line() {
  let scene = Scene::new(Surface::Sequence, vec![Layer::new(solid(RED))]);
  assert_eq!(line(&scene, 4), vec![RED; 4]);
}

#[test]
fn a_gradient_on_a_line_runs_from_the_first_led_to_the_last() {
  use crate::prelude::*;

  for count in [1, 2, 5, 16, 33] {
    let scene = Scene::new(Surface::Sequence, vec![Layer::new(red_to_blue())]);
    let expected = (0..count).map(|i| RED.mix_with(BLUE, i as f32 / (count - 1).max(1) as f32));
    let actual = line(&scene, count);
    for (i, (a, e)) in actual.iter().zip(expected).enumerate() {
      let close = (0..4).all(|c| (a[c] as i16 - e[c] as i16).abs() <= 1);
      assert!(close, "{count} LEDs, LED {i}: {a:?} vs {e:?}");
    }
  }
}

#[test]
fn shapes_on_a_grid() {
  let scene = circle(solid(RED));
  // a 100 x 50 DMD, one sample per pixel
  let lit = (0..50)
    .flat_map(|y| (0..100).map(move |x| (x, y)))
    .map(|(x, y)| sample(&scene, x as f32 + 0.5, y as f32 + 0.5, 0.5))
    .filter(|c| *c == RED)
    .count();
  // close to the area of the circle, less the partly covered edge
  let area = std::f32::consts::PI * 20.0 * 20.0;
  assert!((lit as f32 - area).abs() < 2.0 * std::f32::consts::PI * 20.0, "{lit} vs {area}");
  assert_eq!(sample(&scene, 50.0, 25.0, 0.5), RED);
  assert_eq!(sample(&scene, 5.0, 5.0, 0.5), DARK);
}

/// A circle of diameter 40 at (50, 25)
fn middle_circle() -> Shape {
  Shape::new(ShapeNode::circle(0.5))
    .transform(Transform::Scale(40.0.into()))
    .transform(Transform::Translate(Vec2::new(50.0, 25.0).into()))
}

/// A red layer over a dark one, with a mask on the red
fn masked(mask: Mask) -> Scene {
  Scene::new(Surface::Area, vec![Layer::new(solid(DARK)), Layer::new(solid(RED)).mask(mask)])
}

#[test]
fn a_mask_shows_its_layer_only_inside_it_and_leaves_the_layer_below() {
  let scene = masked(Mask::new(solid(BLUE)).region(middle_circle()));
  assert_eq!(sample(&scene, 50.0, 25.0, 0.5), RED);
  assert_eq!(sample(&scene, 5.0, 5.0, 0.5), DARK);
}

#[test]
fn an_inverted_mask_shows_its_layer_only_outside_it() {
  let scene = masked(Mask::new(solid(BLUE)).region(middle_circle()).inverted());
  assert_eq!(sample(&scene, 50.0, 25.0, 0.5), DARK);
  assert_eq!(sample(&scene, 5.0, 5.0, 0.5), RED);
}

#[test]
fn a_mask_with_no_shape_covers_the_viewport() {
  assert_eq!(sample(&masked(Mask::new(solid(BLUE))), 5.0, 5.0, 0.5), RED);
  assert_eq!(sample(&masked(Mask::new(solid(BLUE)).inverted()), 5.0, 5.0, 0.5), DARK);
}

#[test]
fn a_mask_fades_its_layer_by_its_own_opacity() {
  let half = solid(BLUE).adjust(ColorAdjust::Opacity(0.5.into()));
  let scene = Scene::new(Surface::Area, vec![Layer::new(solid(RED)).mask(Mask::new(half))]);
  let alpha = sample(&scene, 50.0, 25.0, 0.5)[3];
  assert!((alpha as i16 - 128).abs() <= 1, "{alpha}");
}

#[test]
fn a_mask_stays_put_as_its_layer_moves() {
  let moved = middle_circle().transform(Transform::Translate(Vec2::new(20.0, 0.0).into()));
  let mask = Mask::new(solid(BLUE)).region(middle_circle());
  let scene = Scene::new(Surface::Area, vec![Layer::new(solid(RED)).region(moved).mask(mask)]);
  // in both, in the layer alone, and in the mask alone
  assert_eq!(sample(&scene, 60.0, 25.0, 0.5), RED);
  assert_eq!(sample(&scene, 85.0, 25.0, 0.5)[3], 0);
  assert_eq!(sample(&scene, 35.0, 25.0, 0.5)[3], 0);
}

#[test]
fn an_animated_mask_needs_a_timing() {
  let changing = solid(RED).adjust(ColorAdjust::HueShift(Prop::Keyframes(Keyframes { keys: vec![], cycle: Cycle::Once })));
  assert_eq!(masked(Mask::new(changing)).check(), Err(no_timing("layer 2")));
}

#[test]
fn a_masks_image_is_one_the_scene_draws() {
  let image = Fill::new(FillNode::Image { name: "stencil.png".into(), frame: None });
  let scene = masked(Mask::new(image));
  assert_eq!(scene.image_names().collect::<Vec<_>>(), ["stencil.png"]);
}

#[test]
fn a_masks_outline_and_pose_line_up_with_the_layers() {
  let scene = masked(Mask::new(solid(BLUE)).region(middle_circle()));
  let params = Params::new();
  let frame = LayerFrame { time: 0.0, playback_seed: 0, params: &params, assets: Assets::none(), size: SIZE, surface: Surface::Area, visits: None, timing: None, defaults: no_params() };
  assert!(scene.mask_outlines(&frame)[0].is_empty());
  assert!(!scene.mask_outlines(&frame)[1].is_empty());
  assert!(scene.mask_poses(&frame)[0].is_none());
  assert!(scene.mask_poses(&frame)[1].is_some());
}

#[test]
fn shapes_at_sparse_points() {
  let scene = circle(solid(RED));
  // inserts with a large footprint, one well inside, one well outside, one straddling the edge
  assert_eq!(sample(&scene, 50.0, 25.0, 3.0), RED);
  assert_eq!(sample(&scene, 90.0, 25.0, 3.0), DARK);
  let edge = sample(&scene, 70.0, 25.0, 3.0);
  assert!(edge[0] > DARK[0] && edge[0] < RED[0], "{edge:?}");
}

#[test]
fn gradient_fills_its_shape() {
  let scene = circle(red_to_blue());
  assert_eq!(sample(&scene, 30.5, 25.0, 0.0), Rgba([252, 0, 3, 255]));
  assert_eq!(sample(&scene, 69.5, 25.0, 0.0), Rgba([3, 0, 252, 255]));
}

#[test]
fn gradient_turns_with_its_shape() {
  let mut scene = circle(red_to_blue());
  let Some(shape) = &mut scene.layers[1].region else { unreachable!() };
  shape.transform.insert(1, Transform::Rotate(90.0.into()));
  // the red end is now at the top, and the blue end at the bottom
  assert_eq!(sample(&scene, 50.0, 5.5, 0.0), Rgba([252, 0, 3, 255]));
  assert_eq!(sample(&scene, 50.0, 44.5, 0.0), Rgba([3, 0, 252, 255]));
}

#[test]
fn shape_on_a_line_is_measured_along_it() {
  // on 5 LEDs, a circle most of the way across centered on the middle lights all but the end LEDs
  let scene = Scene::new(Surface::Sequence, vec![Layer::new(solid(RED)).region(
    Shape::new(ShapeNode::circle(0.5))
      .transform(Transform::Scale(0.75.into()))
      .transform(Transform::Translate(Vec2::new(0.5, 0.0).into())),
  )]);
  let transparent = Rgba([0, 0, 0, 0]);
  assert_eq!(line(&scene, 5), vec![transparent, RED, RED, RED, transparent]);
}

#[test]
fn a_sequence_plays_on_any_number_of_leds() {
  let scene = Scene::new(Surface::Sequence, vec![Layer::new(red_to_blue())]);
  for count in [2, 6, 8, 60] {
    let colors = line(&scene, count);
    assert_eq!(colors[0], RED, "{count} LEDs");
    assert_eq!(colors[count - 1], BLUE, "{count} LEDs");
  }
}

#[test]
fn a_shape_covers_the_same_part_of_any_strip() {
  let scene = Scene::new(Surface::Sequence, vec![Layer::new(solid(RED)).region(
    Shape::new(ShapeNode::circle(0.5))
      .transform(Transform::Scale(0.5.into()))
      .transform(Transform::Translate(Vec2::new(0.5, 0.0).into())),
  )]);
  let lit = |count| line(&scene, count).iter().filter(|c| **c == RED).count();
  // the middle half, less the LEDs its edges pass through
  assert_eq!(lit(9), 3);
  assert_eq!(lit(101), 49);
}

#[test]
fn nothing_is_seen_outside_the_viewport() {
  let scene = circle(solid(RED));
  let transparent = Rgba([0, 0, 0, 0]);
  assert_eq!(sample(&scene, -1.0, 25.0, 0.5), transparent);
  assert_eq!(sample(&scene, 101.0, 25.0, 0.5), transparent);
  assert_eq!(sample(&scene, 50.0, 51.0, 0.5), transparent);
}

#[test]
fn a_shape_larger_than_the_viewport_can_move_under_it() {
  // a 300 wide gradient, moved so its middle is in view
  let scene = Scene::new(Surface::Area, vec![Layer::new(red_to_blue()).region(
    Shape::new(ShapeNode::Polygon {
      points: [(0.0, 0.0), (6.0, 0.0), (6.0, 1.0), (0.0, 1.0)].map(|(x, y)| Vec2::new(x, y).into()).to_vec(),
    })
    .transform(Transform::Scale(50.0.into()))
    .transform(Transform::Translate(Vec2::new(-100.0, 0.0).into())),
  )]);
  assert_eq!(sample(&scene, 50.0, 25.0, 0.0), Rgba([127, 0, 128, 255]));
}

#[test]
fn a_grid_of_another_shape_does_not_squash_the_scene() {
  // 100 x 50 on a 128 x 32 grid: as wide as it is tall, with bars at the sides
  let scene = circle(solid(RED));
  let params = Params::new();
  let frame = LayerFrame { time: 0.0, playback_seed: 0, params: &params, assets: Assets::none(), size: SIZE, surface: Surface::Area, visits: None, timing: None, defaults: no_params() };
  let pixels: Vec<_> =
    Grid::new(128, 32, Vec2::new(100.0, 50.0)).samples().map(|s| scene.sample(&s, &frame)).collect();
  let row = |y: usize| pixels[y * 128..(y + 1) * 128].iter().filter(|c| **c == RED).count();
  let column = |x: usize| (0..32).filter(|y| pixels[y * 128 + x] == RED).count();
  assert!(row(16).abs_diff(column(64)) <= 2, "{} across, {} down", row(16), column(64));
  assert_eq!(pixels[0], Rgba([0, 0, 0, 0]));
}

/// A level that rises from 0 to 100 over a second, and stays there. Read back with `level_at`.
fn rising(timing: Option<Timing>) -> Scene {
  let track = Prop::Keyframes(Keyframes {
    keys: vec![
      Key::new(0.0, Color(Rgba([0, 0, 0, 255])), Curve::Linear),
      Key::new(1.0, Color(Rgba([100, 0, 0, 255])), Curve::Linear),
    ],
    cycle: Cycle::Once,
  });
  let scene = Scene::new(Surface::Sequence, vec![Layer::new(Fill::new(FillNode::Solid(track)))]);
  match timing {
    Some(timing) => scene.timing(timing),
    None => scene,
  }
}

fn level_at(scene: &Scene, time: f32) -> u8 {
  let params = Params::new();
  scene.sample(&Sample::new(Vec2::new(0.5, 0.0), 0.5), &LayerFrame { time, playback_seed: 0, params: &params, assets: Assets::none(), size: SIZE, surface: Surface::Area, visits: None, timing: None, defaults: no_params() })[0]
}

#[test]
fn a_scene_that_plays_once_rests_at_its_end() {
  let scene = rising(Some(Timing::new(1.0, Cycle::Once)));
  assert_eq!(level_at(&scene, 0.5), 50);
  assert!(!scene.is_complete(0.5));
  assert_eq!(level_at(&scene, 1.0), 100);
  assert!(scene.is_complete(1.0));
  assert_eq!(level_at(&scene, 9.0), 100);
}

#[test]
fn a_scene_repeats_as_many_times_as_it_says() {
  let scene = rising(Some(Timing::new(1.0, Cycle::Times(2))));
  assert_eq!(level_at(&scene, 1.5), 50);
  assert!(!scene.is_complete(1.5));
  assert!(scene.is_complete(2.0));
  assert_eq!(level_at(&scene, 2.5), 100);
}

#[test]
fn a_scene_can_repeat_forever() {
  let scene = rising(Some(Timing::new(1.0, Cycle::Forever)));
  assert_eq!(level_at(&scene, 10.25), 25);
  assert!(!scene.is_complete(1000.0));
}

#[test]
fn timing_cuts_a_track_that_runs_longer() {
  let scene = rising(Some(Timing::new(0.5, Cycle::Forever)));
  assert_eq!(level_at(&scene, 0.75), 25);
}

#[test]
fn a_static_scene_has_no_time() {
  let scene = rising(None);
  assert_eq!(level_at(&scene, 0.5), 0);
  assert!(!scene.is_complete(1000.0));
}

#[test]
fn keyframes_need_a_timing() {
  assert_eq!(rising(None).check(), Err(no_timing("layer 1")));
  assert_eq!(rising(Some(Timing::new(1.0, Cycle::Once))).check(), Ok(()));
  assert_eq!(Scene::new(Surface::Sequence, vec![Layer::new(solid(RED))]).check(), Ok(()));
}

#[test]
fn keyframes_are_found_wherever_a_scene_has_them() {
  fn track<T>() -> Keyframes<T> {
    Keyframes { keys: vec![], cycle: Cycle::Once }
  }
  let in_transform = Layer::new(solid(RED)).region(Shape::new(ShapeNode::circle(0.5)).transform(Transform::Rotate(Prop::Keyframes(track()))));
  let in_point = Layer::new(solid(RED)).region(Shape::new(ShapeNode::Polygon { points: vec![Prop::Keyframes(track())] }));
  let in_stop = Layer::new(Fill::new(FillNode::Gradient {
    stops: vec![Stop::new(0.0, Prop::Keyframes(track()))],
    style: GradientStyle::Linear,
  }));
  let in_adjust = Layer::new(solid(RED).adjust(ColorAdjust::HueShift(Prop::Keyframes(track()))));
  for layer in [in_transform, in_point, in_stop, in_adjust] {
    assert_eq!(Scene::new(Surface::Area, vec![layer]).check(), Err(no_timing("layer 1")));
  }
}

#[test]
fn a_timing_needs_a_length() {
  for duration in [0.0, -1.0, f32::NAN, f32::INFINITY] {
    let scene = Scene::new(Surface::Area, vec![]).timing(Timing::new(duration, Cycle::Once));
    assert_eq!(scene.check(), Err(SceneError::NoDuration), "{duration}");
  }
}

#[test]
fn params_override_defaults() {
  use crate::scene::Value;

  let mut scene = Scene::new(
    Surface::Sequence,
    vec![Layer::new(Fill::new(FillNode::Solid(Prop::Param("color".into()))))],
  );
  scene.params.insert("color".into(), Value::Color(Color(RED)));
  assert_eq!(line(&scene, 1), vec![RED]);

  let params = Params::from([("color".into(), Value::Color(Color(BLUE)))]);
  let frame = LayerFrame { time: 0.0, playback_seed: 0, params: &params, assets: Assets::none(), size: SIZE, surface: Surface::Area, visits: None, timing: None, defaults: no_params() };
  assert_eq!(scene.sample(&Sample::new(Vec2::ZERO, 0.5), &frame), BLUE);
}

/// Red on the left half and blue on the right, with a green row across the middle
fn image_fill() -> Fill {
  Fill::new(FillNode::image("art.png"))
}

fn images() -> Assets {
  const GREEN: Rgba<u8> = Rgba([0, 255, 0, 255]);
  let art = image::RgbaImage::from_fn(4, 3, |x, y| match (x, y) {
    (_, 1) => GREEN,
    (0 | 1, _) => RED,
    _ => BLUE,
  });
  let mut assets = Assets::new();
  assets.insert_picture("art.png", Picture::new(art));
  assets
}

fn drawn(scene: &Scene, x: f32, y: f32, radius: f32) -> Rgba<u8> {
  let (params, assets) = (Params::new(), images());
  let frame = LayerFrame { time: 0.0, playback_seed: 0, params: &params, assets: &assets, size: Vec2::new(4.0, 3.0), surface: Surface::Area, visits: None, timing: None, defaults: no_params() };
  scene.sample(&Sample::new(Vec2::new(x, y), radius), &frame)
}

#[test]
fn an_image_shows_its_pixels_on_a_grid_of_the_same_size() {
  let scene = Scene::new(Surface::Area, vec![Layer::new(image_fill())]);
  let (params, assets) = (Params::new(), images());
  let size = Vec2::new(4.0, 3.0);
  let frame = LayerFrame { time: 0.0, playback_seed: 0, params: &params, assets: &assets, size, surface: Surface::Area, visits: None, timing: None, defaults: no_params() };
  let colors: Vec<_> = Grid::new(4, 3, size).samples().map(|at| scene.sample(&at, &frame)).collect();

  let green = Rgba([0, 255, 0, 255]);
  assert_eq!(colors, [RED, RED, BLUE, BLUE, green, green, green, green, RED, RED, BLUE, BLUE]);
}

#[test]
fn an_image_stretches_over_a_shape_and_moves_with_it() {
  let shape = Shape::new(ShapeNode::circle(0.5)).transform(Transform::Scale(2.0.into())).transform(Transform::Translate(Vec2::new(2.0, 1.5).into()));
  let scene = Scene::new(Surface::Area, vec![Layer::new(image_fill()).region(shape)]);
  assert_eq!(drawn(&scene, 1.5, 0.7, 0.01), RED);
  assert_eq!(drawn(&scene, 2.5, 0.7, 0.01), BLUE);
  assert_eq!(drawn(&scene, 0.1, 0.1, 0.01)[3], 0);
}

#[test]
fn a_strip_runs_along_the_middle_of_an_image() {
  let scene = Scene::new(Surface::Sequence, vec![Layer::new(image_fill())]);
  let (params, assets) = (Params::new(), images());
  let frame = LayerFrame { time: 0.0, playback_seed: 0, params: &params, assets: &assets, size: SIZE, surface: Surface::Area, visits: None, timing: None, defaults: no_params() };
  for at in Line::new(5).samples() {
    assert_eq!(scene.sample(&Sample::new(at.pos, 0.0), &frame), Rgba([0, 255, 0, 255]));
  }
}

#[test]
fn a_shape_scaled_down_covers_more_of_its_image_with_each_sample() {
  let checkers = image::RgbaImage::from_fn(64, 64, |x, y| if (x + y) % 2 == 0 { RED } else { BLUE });
  let mut assets = Assets::new();
  assets.insert_picture("art.png", Picture::new(checkers));
  let (params, size) = (Params::new(), Vec2::new(64.0, 64.0));
  let frame = LayerFrame { time: 0.0, playback_seed: 0, params: &params, assets: &assets, size, surface: Surface::Area, visits: None, timing: None, defaults: no_params() };

  // a whole viewport of it shows each pixel, and the same image in a shape of 4 units has 16 pixels to each unit
  let at = Sample::new(Vec2::new(32.0, 32.0), 0.5);
  let whole = Scene::new(Surface::Area, vec![Layer::new(image_fill())]);
  assert!([RED, BLUE].contains(&whole.sample(&at, &frame)));

  let shape = Shape::new(ShapeNode::circle(0.5)).transform(Transform::Scale(4.0.into())).transform(Transform::Translate(Vec2::splat(32.0).into()));
  let small = Scene::new(Surface::Area, vec![Layer::new(image_fill()).region(shape)]);
  let color = small.sample(&at, &frame);
  assert!((100..=155).contains(&color[0]) && (100..=155).contains(&color[2]), "{color:?}");
}

#[test]
fn transforms_and_adjusts_work_on_an_image_like_any_fill() {
  let flipped = image_fill().transform(Transform::Rotate(180.0.into()));
  let scene = Scene::new(Surface::Area, vec![Layer::new(flipped)]);
  assert_eq!(drawn(&scene, 0.5, 0.5, 0.01), BLUE);
  assert_eq!(drawn(&scene, 3.5, 0.5, 0.01), RED);

  let tinted = image_fill().adjust(ColorAdjust::HueShift(120.0.into()));
  let scene = Scene::new(Surface::Area, vec![Layer::new(tinted)]);
  assert_ne!(drawn(&scene, 0.5, 0.5, 0.01), RED);
}

#[test]
fn an_image_shrunk_by_a_transform_leaves_the_rest_clear() {
  let shrunk = image_fill().transform(Transform::Scale(0.5.into()));
  let scene = Scene::new(Surface::Area, vec![Layer::new(shrunk), ]);
  assert_eq!(drawn(&scene, 0.2, 0.2, 0.01)[3], 0);
  assert_eq!(drawn(&scene, 1.5, 1.0, 0.01), RED);
}

#[test]
fn a_scene_lists_the_images_it_draws() {
  let scene = Scene::new(Surface::Area, vec![Layer::new(solid(RED)), Layer::new(image_fill())]);
  assert_eq!(scene.image_names().collect::<Vec<_>>(), ["art.png"]);
}

/// A scene of an animated image, one pixel for each frame number
fn playing(frame: Prop<f32>) -> Scene {
  let image = FillNode::Image { name: "a.gif".into(), frame: Some(frame) };
  Scene::new(Surface::Area, vec![Layer::new(Fill::new(image))]).timing(Timing::new(1.0, Cycle::Forever))
}

/// Which of 4 frames is drawn. Each frame's red is its number.
fn frame_at(scene: &Scene, time: f32, playback_seed: u32, params: &Params) -> u8 {
  let art = (0..4).map(|n| image::RgbaImage::from_pixel(1, 1, Rgba([n, 0, 0, 255])));
  let mut assets = Assets::new();
  assets.insert_picture("a.gif", Picture::animated(art, 0.4));
  let frame = LayerFrame { time, playback_seed, params, assets: &assets, size: Vec2::ONE, surface: Surface::Area, visits: None, timing: None, defaults: no_params() };
  scene.sample(&Sample::new(Vec2::splat(0.5), 0.1), &frame)[0]
}

#[test]
fn keys_going_from_0_to_the_frame_count_spend_the_same_time_on_each() {
  let track = Keyframes { keys: vec![Key::new(0.0, 0.0, Curve::Linear), Key::new(1.0, 4.0, Curve::Linear)], cycle: Cycle::Once };
  let scene = playing(Prop::Keyframes(track));
  let frames = [0.1, 0.3, 0.6, 0.9].map(|time| frame_at(&scene, time, 0, &Params::new()));
  assert_eq!(frames, [0, 1, 2, 3]);
}

#[test]
fn a_param_chooses_a_frame() {
  let scene = playing(Prop::Param("frame".into()));
  let params = Params::from([("frame".into(), crate::scene::Value::Float(2.0))]);
  assert_eq!(frame_at(&scene, 0.0, 0, &params), 2);
}

#[test]
fn a_spread_up_to_the_frame_count_picks_each_frame_about_as_often() {
  let random = Random { picks: vec![Pick::new(0.0)], spread: Spread::Float(0.0, 4.0), seed: 1, force_unique: true };
  let scene = playing(Prop::Random(random));
  let mut times = [0; 4];
  for playback_seed in 0..4000 {
    times[frame_at(&scene, 0.5, playback_seed, &Params::new()) as usize] += 1;
  }
  assert!(times.iter().all(|times| (900..1100).contains(times)), "{times:?}");
}

#[test]
fn a_random_frame_is_a_different_frame_at_every_pick() {
  let random = Random { picks: vec![Pick::new(0.0)], spread: Spread::Float(0.0, 4.0), seed: 1, force_unique: true };
  let scene = playing(Prop::Random(random));
  let frames: Vec<u8> = (0..300).map(|cycle| frame_at(&scene, cycle as f32 + 0.5, 0, &Params::new())).collect();
  assert!(frames.windows(2).all(|pair| pair[0] != pair[1]), "{frames:?}");
}

#[test]
fn a_frame_that_changes_needs_timing() {
  let track = Keyframes { keys: vec![Key::new(0.0, 0.0, Curve::Linear)], cycle: Cycle::Once };
  let image = FillNode::Image { name: "a.gif".into(), frame: Some(Prop::Keyframes(track)) };
  let scene = Scene::new(Surface::Area, vec![Layer::new(Fill::new(image))]);
  assert!(matches!(scene.check(), Err(SceneError::NoTiming { .. })));
}

#[test]
fn an_image_yet_to_be_chosen_is_not_listed() {
  let scene = Scene::new(Surface::Area, vec![Layer::new(Fill::new(FillNode::image("")))]);
  assert_eq!(scene.image_names().count(), 0);
  assert_eq!(sample(&scene, 1.0, 1.0, 0.5)[3], 0);
}

#[test]
fn an_image_with_nothing_to_draw_it_is_clear_and_needs_no_timing() {
  let scene = Scene::new(Surface::Area, vec![Layer::new(image_fill())]);
  assert!(scene.check().is_ok());
  assert_eq!(sample(&scene, 1.0, 1.0, 0.5)[3], 0);
}

/// A red layer on a graph, its opacity responding where traversal "wave" has just been: out from the first node one
/// edge a second, each node fading out over one step
fn wave() -> Scene {
  let shown = Prop::Visit(VisitResponse { smooth_out: Some(1.0), ..VisitResponse::new("wave", 0.0, 1.0) });
  let wave = Traversal::new("wave", NodeRef::name("0"), TraversalAlgorithm::BreadthFirst { wander: 0.0.into() }, 1.0);
  let layer = Layer::new(solid(RED).adjust(ColorAdjust::Opacity(shown))).graph(GraphMask::new(vec![wave]));
  Scene::new(Surface::Sequence, vec![layer]).timing(Timing::new(10.0, Cycle::Once))
}

/// Alpha along a line of 5, with the samples numbered as graph nodes
fn graph_alphas(scene: &Scene, time: f32) -> Vec<u8> {
  let params = Params::new();
  let samples: Vec<Sample> = Line::new(5).samples().enumerate().map(|(i, sample)| sample.at_node(i)).collect();
  let nodes = samples.iter().enumerate().map(|(i, sample)| GraphNode::new(i.to_string(), sample.pos)).collect();
  let graph = Graph::build(nodes);
  let frame = LayerFrame { time, playback_seed: 0, params: &params, assets: Assets::none(), size: SIZE, surface: Surface::Sequence, visits: None, timing: None, defaults: no_params() };
  let visits = scene.visits(&graph, &frame);
  samples.iter().map(|sample| scene.sample_visited(sample, &frame, &visits)[3]).collect()
}

#[test]
fn visits_set_off_responses_at_their_nodes() {
  assert_eq!(graph_alphas(&wave(), 0.0), [255, 0, 0, 0, 0]);
  assert_eq!(graph_alphas(&wave(), 2.0), [0, 0, 255, 0, 0]);
  assert_eq!(graph_alphas(&wave(), 2.5), [0, 0, 128, 0, 0]);
}


/// A red layer on a graph, its opacity responding to traversal "line": from the first node to the last within the
/// 10 second cycle, reaching the last at 8 seconds
fn revealed(smooth_out: Option<f32>) -> Scene {
  let line = Traversal::new("line", NodeRef::name("0"), TraversalAlgorithm::ShortestPath { to: NodeRef::name("4"), wander: 0.0.into() }, Speed::Duration(None));
  let shown = Prop::Visit(VisitResponse { smooth_out, ..VisitResponse::new("line", 0.0, 1.0) });
  let layer = Layer::new(solid(RED).adjust(ColorAdjust::Opacity(shown))).graph(GraphMask::new(vec![line]));
  Scene::new(Surface::Sequence, vec![layer]).timing(Timing::new(10.0, Cycle::Once))
}

#[test]
fn an_opacity_responding_to_visits_shows_the_path_as_it_grows() {
  let scene = revealed(None);
  assert_eq!(graph_alphas(&scene, 0.0), [255, 0, 0, 0, 0]);
  assert_eq!(graph_alphas(&scene, 5.0), [255, 255, 255, 0, 0]);
  assert_eq!(graph_alphas(&scene, 10.0), [255; 5]);
}

#[test]
fn an_opacity_with_a_smooth_out_leaves_a_trail() {
  // nodes 0.25 apart, one every 2 seconds, each fading over 0.5: two nodes behind the newest
  let scene = revealed(Some(0.5));
  assert_eq!(graph_alphas(&scene, 5.0), [0, 64, 191, 0, 0]);
}
#[test]
fn a_graph_layer_leaves_out_samples_that_arent_nodes() {
  let mut scene = wave();
  scene.layers.insert(0, Layer::new(solid(BLUE)));
  scene.layers[1].fill.adjust.clear();
  let params = Params::new();
  let frame = LayerFrame { time: 0.0, playback_seed: 0, params: &params, assets: Assets::none(), size: SIZE, surface: Surface::Sequence, visits: None, timing: None, defaults: no_params() };
  assert_eq!(scene.sample(&Sample::new(Vec2::ZERO, 0.1), &frame), BLUE);
  assert_eq!(scene.sample(&Sample::new(Vec2::ZERO, 0.1).at_node(0), &frame), RED);
}

#[test]
fn without_visits_a_graph_layer_rests() {
  let scene = wave();
  let params = Params::new();
  let frame = LayerFrame { time: 2.0, playback_seed: 0, params: &params, assets: Assets::none(), size: SIZE, surface: Surface::Sequence, visits: None, timing: None, defaults: no_params() };
  assert_eq!(scene.sample(&Sample::new(Vec2::ZERO, 0.1).at_node(0), &frame)[3], 0);
}

#[test]
fn traversals_need_their_own_names_and_a_rate() {
  let traversal = |name: &str, rate: f32| Traversal::new(name, NodeRef::name("0"), TraversalAlgorithm::BreadthFirst { wander: 0.0.into() }, rate);
  let scene = |traversals| Scene::new(Surface::Area, vec![Layer::new(solid(RED)).graph(GraphMask::new(traversals))]).timing(Timing::new(1.0, Cycle::Once));
  assert!(scene(vec![traversal("a", 1.0), traversal("b", 1.0)]).check().is_ok());
  assert_eq!(
    scene(vec![traversal("a", 1.0), traversal("a", 1.0)]).check(),
    Err(SceneError::SameTraversalName { layer: "layer 1".into(), traversal: "a".into() })
  );
  assert_eq!(scene(vec![traversal("a", 0.0)]).check(), Err(SceneError::NoSpeed { layer: "layer 1".into(), traversal: "a".into() }));
}

/// Colors at 5 nodes on a diagonal from (10, 10) to (40, 40), with a fill fit to the shortest path between two of them
fn fit_colors(fill: Fill, shape: Option<Shape>, from: &str, to: &str) -> Vec<Rgba<u8>> {
  let bolt = Traversal::new("bolt", NodeRef::name(from), TraversalAlgorithm::ShortestPath { to: NodeRef::name(to), wander: 0.0.into() }, 10.0);
  let mut layer = Layer::new(fill).graph(GraphMask::new(vec![bolt]));
  layer.region = shape;
  let scene = Scene::new(Surface::Area, vec![layer]).timing(Timing::new(10.0, Cycle::Once));

  let samples: Vec<Sample> = (0..5).map(|i| Sample::new(Vec2::splat(10.0 + 7.5 * i as f32), 0.5).at_node(i)).collect();
  let nodes = samples.iter().enumerate().map(|(i, sample)| GraphNode::new(i.to_string(), sample.pos)).collect();
  let graph = Graph::build(nodes);
  let params = Params::new();
  let frame = LayerFrame { time: 0.0, playback_seed: 0, params: &params, assets: Assets::none(), size: SIZE, surface: Surface::Area, visits: None, timing: None, defaults: no_params() };
  let visits = scene.visits(&graph, &frame);
  samples.iter().map(|sample| scene.sample_visited(sample, &frame, &visits)).collect()
}

fn is_between_red_and_blue(color: Rgba<u8>) -> bool {
  (100..=155).contains(&color[0]) && (100..=155).contains(&color[2])
}

#[test]
fn fit_runs_a_gradient_from_where_a_traversal_starts_to_where_it_ends() {
  let colors = fit_colors(red_to_blue().transform(Transform::Fit("bolt".into())), None, "0", "4");
  assert_eq!((colors[0], colors[4]), (RED, BLUE));
  assert!(is_between_red_and_blue(colors[2]), "{:?}", colors[2]);

  let backwards = fit_colors(red_to_blue().transform(Transform::Fit("bolt".into())), None, "4", "0");
  assert_eq!((backwards[0], backwards[4]), (BLUE, RED));
}

#[test]
fn transforms_before_fit_move_the_fill_along_the_traversal() {
  let radial = Fill::new(FillNode::Gradient { stops: vec![Stop::new(0.0, Color(RED)), Stop::new(1.0, Color(BLUE))], style: GradientStyle::Radial })
    .transform(Transform::Scale(2.0.into()))
    .transform(Transform::Translate(Vec2::new(-0.5, 0.0).into()))
    .transform(Transform::Fit("bolt".into()));
  let colors = fit_colors(radial, None, "0", "4");
  assert_eq!((colors[0], colors[4]), (RED, BLUE));
  assert!(is_between_red_and_blue(colors[2]), "{:?}", colors[2]);
}

#[test]
fn fit_works_inside_a_shape() {
  let rect = Shape::new(ShapeNode::Rect { width: 60.0.into(), height: 60.0.into() }).transform(Transform::Translate(Vec2::new(25.0, 25.0).into()));
  let colors = fit_colors(red_to_blue().transform(Transform::Fit("bolt".into())), Some(rect), "0", "4");
  assert_eq!((colors[0], colors[4]), (RED, BLUE));
  assert!(is_between_red_and_blue(colors[2]), "{:?}", colors[2]);
}

#[test]
fn fit_changes_nothing_without_a_traversal_to_follow() {
  let fitted = fit_colors(red_to_blue().transform(Transform::Fit("nope".into())), None, "0", "4");
  assert_eq!(fitted, fit_colors(red_to_blue(), None, "0", "4"));
}

#[test]
fn a_graph_with_traversals_needs_a_timing() {
  let scene = Scene::new(Surface::Area, vec![wave().composition.layers[0].clone()]);
  assert!(matches!(scene.check(), Err(SceneError::NoTiming { .. })));
}

#[test]
fn a_grid_is_drawn_over_what_is_already_in_it() {
  let layer = Layer::new(solid(Rgba([255, 0, 0, 128]))).region(Shape::new(ShapeNode::circle(5.0)).transform(Transform::Translate(Vec2::new(16.0, 8.0).into())));
  let grid = Grid::new(32, 16, Vec2::new(32.0, 16.0));
  let params = Params::new();
  let frame = LayerFrame { time: 0.0, playback_seed: 0, params: &params, assets: Assets::none(), size: grid.size, surface: Surface::Area, visits: None, timing: None, defaults: no_params() };
  let mut pixels = vec![BLUE; 32 * 16];
  layer.sample_grid(&grid, &frame, &mut pixels);
  let alone: Vec<_> = grid.samples().map(|at| layer.sample(&at, &frame).composite_over(BLUE)).collect();
  assert_eq!(pixels, alone);
  assert_ne!(pixels[8 * 32 + 16], BLUE);
  assert_eq!(pixels[0], BLUE);
}

#[test]
fn a_prepared_layer_draws_samples_of_another_radius_as_they_are_drawn_alone() {
  // on a Sequence, how far a Rotate turns depends on the radius of the samples
  let scene = turned(45.0);
  let params = Params::new();
  let frame = LayerFrame { time: 0.0, playback_seed: 0, params: &params, assets: Assets::none(), size: SIZE, surface: Surface::Sequence, visits: None, timing: None, defaults: no_params() };
  let prepared = scene.layers[0].prepare(&frame, 0.25);
  for at in Line::new(8).samples() {
    assert_eq!(prepared.sample(&at), scene.layers[0].sample(&at, &frame));
  }
}

#[test]
fn a_colored_stroke_is_drawn_over_the_fill_and_one_without_a_color_takes_it() {
  let circle = || Shape::new(ShapeNode::circle(10.0)).transform(Transform::Translate(Vec2::new(50.0, 25.0).into()));
  let outlined = circle().adjust(ShapeAdjust::Stroke { width: 2.0.into(), align: StrokeAlign::Inside, color: Some(Color(BLUE).into()) });
  let scene = Scene::new(Surface::Area, vec![Layer::new(solid(RED)).region(outlined)]);
  assert_eq!(sample(&scene, 50.0, 25.0, 0.1), RED);
  assert_eq!(sample(&scene, 59.0, 25.0, 0.1), BLUE);
  assert_eq!(sample(&scene, 61.0, 25.0, 0.1), Rgba([0, 0, 0, 0]));

  let ring = circle().adjust(ShapeAdjust::Stroke { width: 2.0.into(), align: StrokeAlign::Inside, color: None });
  let scene = Scene::new(Surface::Area, vec![Layer::new(solid(RED)).region(ring)]);
  assert_eq!(sample(&scene, 50.0, 25.0, 0.1), Rgba([0, 0, 0, 0]));
  assert_eq!(sample(&scene, 59.0, 25.0, 0.1), RED);
}

fn text(text: &str, size: f32) -> Shape {
  Shape::new(ShapeNode::text(text, "pixel.ttf", size, Vec2::splat(20.0)))
}

/// `text` in a box `frame` wide and tall, with its top left corner at `corner`
fn text_box(text: &str, size: f32, corner: Vec2, frame: Vec2, align: TextAlign, vertical_align: VerticalAlign) -> Shape {
  let base = ShapeNode::Text { text: text.into(), font: "pixel.ttf".into(), size: size.into(), width: frame.x.into(), height: frame.y.into(), align, vertical_align };
  Shape::new(base).transform(Transform::Translate((corner + frame / 2.0).into()))
}

fn text_pixels(shape: Shape, width: usize, height: usize) -> Vec<String> {
  pixels(&Scene::new(Surface::Area, vec![Layer::new(solid(RED)).region(shape)]), width, height)
}

/// The scene drawn on a grid with hard edges, a row of characters for each row of pixels: `#` where it's drawn, `.`
/// where it isn't, and `+` where it's partly drawn
fn pixels(scene: &Scene, width: usize, height: usize) -> Vec<String> {
  let mut assets = Assets::new();
  assets.insert_font("pixel.ttf", crate::scene::font::tests::pixel_font());
  let params = Params::new();
  let size = Vec2::new(width as f32, height as f32);
  let frame = LayerFrame { time: 0.0, playback_seed: 0, params: &params, assets: &assets, size, surface: Surface::Area, visits: None, timing: None, defaults: no_params() };
  let alpha = |x: usize, y: usize| scene.sample(&Sample::new(Vec2::new(x as f32 + 0.5, y as f32 + 0.5), 0.0), &frame)[3];
  let mark = |alpha: u8| match alpha {
    0 => '.',
    255 => '#',
    _ => '+',
  };
  (0..height).map(|y| (0..width).map(|x| mark(alpha(x, y))).collect()).collect()
}

#[test]
fn a_pixel_font_at_its_design_size_draws_its_pixels() {
  // the font's ascent puts the baseline 11 down from the top of the box, at row 8
  let shape = text_box("HI", 16.0, Vec2::new(1.0, -3.0), Vec2::new(20.0, 20.0), TextAlign::Left, VerticalAlign::Top);
  let scene = Scene::new(Surface::Area, vec![Layer::new(solid(RED)).region(shape)]);
  let expected = [
    "............",
    ".#...#.###..",
    ".#...#..#...",
    ".#...#..#...",
    ".#####..#...",
    ".#...#..#...",
    ".#...#..#...",
    ".#...#.###..",
    "............",
  ];
  assert_eq!(pixels(&scene, 12, 9), expected);
}

#[test]
fn text_takes_a_stroke_like_any_shape() {
  let outlined = text_box("I", 16.0, Vec2::new(2.0, -2.0), Vec2::new(20.0, 20.0), TextAlign::Left, VerticalAlign::Top)
    .adjust(ShapeAdjust::Stroke { width: 1.0.into(), align: StrokeAlign::Outside, color: None });
  let scene = Scene::new(Surface::Area, vec![Layer::new(solid(RED)).region(outlined)]);
  // the stroke goes around the "I" and in under its bars, leaving the inside out
  let expected = [
    ".......",
    ".#####.",
    ".#...#.",
    ".##.##.",
    "..#.#..",
    "..#.#..",
    "..#.#..",
    ".##.##.",
    ".#...#.",
    ".#####.",
    ".......",
  ];
  assert_eq!(pixels(&scene, 7, 11), expected);
}

#[test]
fn text_in_a_font_thats_not_loaded_draws_nothing() {
  let shape = Shape::new(ShapeNode::text("HI", "missing.ttf", 16.0, Vec2::new(12.0, 7.0))).transform(Transform::Translate(Vec2::new(6.0, 3.5).into()));
  let scene = Scene::new(Surface::Area, vec![Layer::new(solid(RED)).region(shape)]);
  assert!(pixels(&scene, 12, 7).iter().all(|row| !row.contains(['#', '+'])));
}

#[test]
fn a_layer_names_the_fonts_its_region_and_its_masks_use() {
  let layer = Layer::new(solid(RED)).region(text("A", 8.0)).mask(Mask::new(solid(RED)).region(
    Shape::new(ShapeNode::text("B", "other.ttf", 8.0, Vec2::splat(8.0))),
  ));
  assert_eq!(layer.font_names().collect::<Vec<_>>(), ["pixel.ttf", "other.ttf"]);
}

#[test]
fn each_line_is_aligned_across_the_box() {
  let row = |align| text_pixels(text_box("I", 16.0, Vec2::ZERO, Vec2::new(16.0, 14.0), align, VerticalAlign::Top), 16, 14)[4].clone();
  assert_eq!(row(TextAlign::Left), "###.............");
  assert_eq!(row(TextAlign::Center), "......###.......");
  assert_eq!(row(TextAlign::Right), "............###.");
}

/// The first row with any of the text in it
fn first_row(rows: &[String]) -> Option<usize> {
  rows.iter().position(|row| row.contains('#'))
}

#[test]
fn the_lines_are_aligned_up_and_down_the_box_on_whole_pixels() {
  let top = |vertical| first_row(&text_pixels(text_box("I", 16.0, Vec2::ZERO, Vec2::new(6.0, 24.0), TextAlign::Center, vertical), 6, 24));
  // a line is 13.66 from the font's ascent to its descent, and a capital is 7 tall with its top 3.66 below the ascent
  assert_eq!(top(VerticalAlign::Top), Some(4));
  assert_eq!(top(VerticalAlign::Middle), Some(9));
  assert_eq!(top(VerticalAlign::Bottom), Some(14));
}

#[test]
fn text_breaks_into_lines_a_whole_line_height_apart() {
  // "I I" is wider than the box, so it breaks between the words
  let rows = text_pixels(text_box("I I\nI", 16.0, Vec2::ZERO, Vec2::new(8.0, 48.0), TextAlign::Left, VerticalAlign::Top), 8, 48);
  let tops: Vec<usize> = (0..rows.len()).filter(|y| rows[*y].contains('#') && (*y == 0 || !rows[y - 1].contains('#'))).collect();
  assert_eq!(tops, [4, 18, 32]);
}

#[test]
fn text_past_its_box_is_cut_off() {
  let shape = text_box("HI", 16.0, Vec2::ONE, Vec2::new(5.0, 4.0), TextAlign::Left, VerticalAlign::Middle);
  let expected = ["........", ".#...#..", ".#...#..", ".#####..", ".#...#..", "........", "........", "........"];
  assert_eq!(text_pixels(shape, 8, 8), expected);
}

#[test]
fn text_is_on_whole_pixels_wherever_its_box_is() {
  // a box at half a unit, and one an odd width across, so its middle is half a unit in
  for (corner, frame) in [(Vec2::splat(0.5), Vec2::new(16.0, 14.0)), (Vec2::ZERO, Vec2::new(15.0, 13.0)), (Vec2::new(1.3, 0.7), Vec2::new(14.2, 13.9))] {
    for align in [TextAlign::Left, TextAlign::Center, TextAlign::Right] {
      let rows = text_pixels(text_box("HI", 16.0, corner, frame, align, VerticalAlign::Middle), 18, 16);
      assert!(rows.iter().any(|row| row.contains('#')), "{corner} {frame} {align:?}");
      assert!(rows.iter().all(|row| !row.contains('+')), "{corner} {frame} {align:?}: {rows:#?}");
    }
  }
}
