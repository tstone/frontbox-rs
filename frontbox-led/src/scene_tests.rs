//! Scenes played by the `SceneSystem`, as the `LedSystem` draws them

use std::sync::LazyLock;

use frontbox_core::prelude::*;
use frontbox_gfx::animation::{Curve, Cycle};
use frontbox_gfx::prelude::*;
use frontbox_gfx::scene::*;
use frontbox_scene::*;

use crate::*;

const OWNER: u64 = 42;

static PLANE: ReferencePlane = ReferencePlane::new("Plane").extent(Vec2::new(10.0, 10.0)).build();
static WIDE: ReferencePlane = ReferencePlane::new("Wide").extent(Vec2::new(20.0, 10.0)).build();
static OFFSET: LazyLock<ReferencePlane> =
  LazyLock::new(|| ReferencePlane::new("Offset").origin(Vec3::new(100.0, 0.0, 0.0)).extent(Vec2::new(10.0, 10.0)).build());

/// LEDs "led0".. at these locations, with every plane added
fn placed_leds(locations: &[Option<Vec3>]) -> (TestContext, Vec<LedAddress>) {
  let mut context = TestContext::default();
  context.insert_system(LedSystem::new());
  context.insert_system(SceneSystem::new());
  let addresses: Vec<_> = (0..locations.len()).map(|i| LedAddress::new(ExpAddress::default(), i as u8)).collect();
  for (i, address) in addresses.iter().enumerate() {
    context.insert_led(LED { name: format!("led{i}"), address: address.clone(), location: locations[i], ..Default::default() });
  }
  context.add_plane(&PLANE);
  context.add_plane(&WIDE);
  context.add_plane(&OFFSET);
  context.sys_ctx().systems.with::<LedSystem, _>(|leds| leds.on_spawn(&context.sys_ctx()));
  (context, addresses)
}

/// Three LEDs, "led0" to "led2", with no location
fn three_leds() -> (TestContext, Vec<LedAddress>) {
  placed_leds(&[None, None, None])
}

fn at(x: f32, y: f32) -> Option<Vec3> {
  Some(Vec3::new(x, y, 0.0))
}

fn play_as(context: &TestContext, owner: u64, scene: impl Into<PlayableScene>) -> PlaybackId {
  context.sys_ctx().systems.with::<SceneSystem, _>(|scenes| scenes.start(owner, &scene.into())).unwrap()
}

fn play(context: &TestContext, scene: impl Into<PlayableScene>) -> PlaybackId {
  play_as(context, OWNER, scene)
}

fn with_scenes<R>(context: &TestContext, f: impl FnOnce(&mut SceneSystem) -> R) -> R {
  context.sys_ctx().systems.with::<SceneSystem, _>(f).unwrap()
}

fn tick(context: &TestContext, delta: Duration) {
  with_scenes(context, |scenes| scenes.on_tick(delta, &context.sys_ctx()));
}

/// What each LED shows, `None` where nothing gives it a color
fn shown(context: &TestContext, addresses: &[LedAddress]) -> Vec<Option<Rgba<u8>>> {
  let resolved = context.sys_ctx().systems.with::<LedSystem, _>(|leds| leds.resolve(&context.sys_ctx())).unwrap();
  addresses.iter().map(|address| resolved.iter().find(|(led, _)| led == address).and_then(|(_, color)| *color)).collect()
}

fn played(context: &TestContext, addresses: &[LedAddress], scene: Scene) -> Vec<Option<Rgba<u8>>> {
  play(context, scene);
  shown(context, addresses)
}

fn solid(color: Rgba<u8>) -> Layer {
  Layer::new(Fill::new(FillNode::Solid(Color(color).into())))
}

fn see_through(color: Rgba<u8>) -> Layer {
  solid(Rgba([color[0], color[1], color[2], 128]))
}

fn red_to_blue() -> Layer {
  Layer::new(Fill::new(FillNode::Gradient {
    stops: vec![Stop::new(0.0, Color(Rgba::red())), Stop::new(1.0, Color(Rgba::blue()))],
    style: GradientStyle::Linear,
  }))
}

/// One layer along some LEDs
fn on(leds: LedQ, layer: Layer) -> Scene {
  Scene::new(vec![LedLayer::sequence(leds, layer).into()])
}

/// One layer across a plane
fn across(plane: &str, leds: LedQ, layer: Layer) -> Scene {
  Scene::new(vec![LedLayer::area(leds, plane, layer).into()])
}

fn on_plane(leds: LedQ, layer: Layer) -> Scene {
  across("Plane", leds, layer)
}

fn timed(cycle: Cycle) -> Scene {
  on(LedQ::name("led0"), solid(Rgba::red())).timing(Timing::new(1.0, cycle))
}

#[test]
fn an_image_runs_along_the_leds() {
  let (context, addresses) = three_leds();
  let art = image::RgbaImage::from_fn(8, 1, |x, _| if x < 4 { Rgba::red() } else { Rgba::blue() });
  let mut scene = on(LedQ::names(vec!["led0", "led1", "led2"]), Layer::new(Fill::new(FillNode::image("art.png"))));
  scene.images.insert("art.png", Picture::new(art));
  play(&context, PlayableScene::new(scene));
  let shown = shown(&context, &addresses);
  assert_eq!((shown[0], shown[2]), (Some(Rgba::red()), Some(Rgba::blue())));
}

#[test]
fn gradient_runs_along_the_leds() {
  let (context, addresses) = three_leds();
  let scene = on(LedQ::names(vec!["led0", "led1", "led2"]), red_to_blue());
  assert_eq!(played(&context, &addresses, scene), vec![Some(Rgba::red()), Some(Rgba([127, 0, 128, 255])), Some(Rgba::blue())]);
}

#[test]
fn time_moves_tracks() {
  let (context, addresses) = three_leds();
  let track = Prop::Track(Track {
    keys: vec![Key::new(0.0, Color(Rgba([0, 0, 0, 255])), Curve::Linear), Key::new(1.0, Color(Rgba([200, 0, 0, 255])), Curve::Linear)],
    cycle: Cycle::Once,
  });
  let scene = on(LedQ::name("led0"), Layer::new(Fill::new(FillNode::Solid(track)))).timing(Timing::new(1.0, Cycle::Once));
  play(&context, PlayableScene::new(scene));

  tick(&context, Duration::from_millis(500));
  assert_eq!(shown(&context, &addresses)[0], Some(Rgba([100, 0, 0, 255])));
}

#[test]
fn clears_the_leds_once_complete() {
  let (context, addresses) = three_leds();
  play(&context, PlayableScene::new(timed(Cycle::Once)));

  tick(&context, Duration::from_millis(500));
  assert_eq!(shown(&context, &addresses)[0], Some(Rgba::red()));
  tick(&context, Duration::from_millis(500));
  assert_eq!(shown(&context, &addresses)[0], None);
}

#[test]
fn can_hold_its_last_moment() {
  let (context, addresses) = three_leds();
  play(&context, PlayableScene::new(on(LedQ::name("led0"), solid(Rgba::red())).timing(Timing::new(1.0, Cycle::Once).end(EndBehavior::Hold))));
  tick(&context, Duration::from_secs(2));
  assert_eq!(shown(&context, &addresses)[0], Some(Rgba::red()));
}

#[test]
fn each_play_picks_its_own_random_values() {
  let (context, addresses) = three_leds();
  let spread = Spread::Color { hue: (0.0, 360.0), brightness: (1.0, 1.0), alpha: (1.0, 1.0) };
  let color = Prop::Random(Random { picks: vec![Pick::new(0.0)], spread, seed: 0, force_unique: true });
  let scene = on(LedQ::name("led0"), Layer::new(Fill::new(FillNode::Solid(color)))).timing(Timing::new(1.0, Cycle::Once));

  let mut shots = vec![];
  for _ in 0..8 {
    let id = play(&context, PlayableScene::new(scene.clone()));
    shots.push(shown(&context, &addresses)[0]);
    with_scenes(&context, |scenes| scenes.remove(id));
  }
  assert!(shots.iter().any(|shot| *shot != shots[0]), "{shots:?}");
}

#[test]
fn a_scene_that_repeats_forever_keeps_playing() {
  let (context, addresses) = three_leds();
  play(&context, PlayableScene::new(timed(Cycle::Forever)));
  tick(&context, Duration::from_secs(100));
  assert_eq!(shown(&context, &addresses)[0], Some(Rgba::red()));
}

#[test]
fn a_static_scene_keeps_playing() {
  let (context, addresses) = three_leds();
  play(&context, PlayableScene::new(on(LedQ::name("led0"), solid(Rgba::red()))));
  tick(&context, Duration::from_secs(100));
  assert_eq!(shown(&context, &addresses)[0], Some(Rgba::red()));
}

#[test]
fn stop_clears_the_leds() {
  let (context, addresses) = three_leds();
  let id = play(&context, PlayableScene::new(on(LedQ::name("led0"), solid(Rgba::red()))));
  assert_eq!(shown(&context, &addresses)[0], Some(Rgba::red()));

  with_scenes(&context, |scenes| scenes.remove(id));
  assert_eq!(shown(&context, &addresses)[0], None);
}

#[test]
fn params_set_by_the_game() {
  let (context, addresses) = three_leds();
  let scene = on(LedQ::name("led0"), Layer::new(Fill::new(FillNode::Solid(Prop::Param("color".into())))));
  let id = play(&context, PlayableScene::new(scene).param("color", Rgba::red()));
  assert_eq!(shown(&context, &addresses)[0], Some(Rgba::red()));

  with_scenes(&context, |scenes| scenes.playback(id).unwrap().set_param("color", Rgba::green()));
  assert_eq!(shown(&context, &addresses)[0], Some(Rgba::green()));
}

#[test]
fn a_scene_takes_its_size_from_the_plane() {
  // the same scene on a plane twice as wide: halfway across its gradient is further along
  let (context, addresses) = placed_leds(&[at(20.0, 5.0)]);
  assert_eq!(played(&context, &addresses, across("Wide", LedQ::every(), red_to_blue())), vec![Some(Rgba::blue())]);
}

#[test]
fn colors_come_from_where_leds_are_not_their_order() {
  // out of order and unevenly spread: along a list these would be red, purple, blue
  let (context, addresses) = placed_leds(&[at(10.0, 5.0), at(0.0, 5.0), at(5.0, 5.0)]);
  assert_eq!(
    played(&context, &addresses, on_plane(LedQ::every(), red_to_blue())),
    vec![Some(Rgba::blue()), Some(Rgba::red()), Some(Rgba([127, 0, 128, 255]))]
  );
}

#[test]
fn only_leds_matching_the_query_take_part() {
  let (context, addresses) = placed_leds(&[at(1.0, 1.0), at(2.0, 2.0)]);
  assert_eq!(played(&context, &addresses, on_plane(LedQ::name("led1"), solid(Rgba::red()))), vec![None, Some(Rgba::red())]);
}

#[test]
fn leds_without_a_location_dont_take_part() {
  let (context, addresses) = placed_leds(&[None, at(2.0, 2.0)]);
  assert_eq!(played(&context, &addresses, on_plane(LedQ::every(), solid(Rgba::red()))), vec![None, Some(Rgba::red())]);
}

#[test]
fn a_layer_with_no_shape_stops_at_the_edge_of_the_plane() {
  let (context, addresses) = placed_leds(&[at(2.0, 2.0), at(11.0, 2.0)]);
  assert_eq!(played(&context, &addresses, on_plane(LedQ::every(), solid(Rgba::red()))), vec![Some(Rgba::red()), None]);
}

/// A circle of radius 2 around this position
fn circle(at: Prop<Vec2>) -> Shape {
  Shape::new(ShapeNode::circle(0.5)).transform(Transform::Scale(4.0.into())).transform(Transform::Translate(at))
}

#[test]
fn a_shape_reaches_leds_past_the_edge_of_the_plane() {
  let (context, addresses) = placed_leds(&[at(11.0, 5.0), at(20.0, 5.0)]);
  let scene = on_plane(LedQ::every(), solid(Rgba::red()).region(circle(Vec2::new(11.0, 5.0).into())));
  assert_eq!(played(&context, &addresses, scene), vec![Some(Rgba::red()), None]);
}

#[test]
fn an_led_a_shape_moves_off_is_given_up() {
  let (context, addresses) = placed_leds(&[at(5.0, 5.0)]);
  let path = Track {
    keys: vec![Key::new(0.0, Vec2::new(5.0, 5.0), Curve::Linear), Key::new(1.0, Vec2::new(50.0, 5.0), Curve::Linear)],
    cycle: Cycle::Once,
  };
  let scene = on_plane(LedQ::every(), solid(Rgba::red()).region(circle(Prop::Track(path)))).timing(Timing::new(1.0, Cycle::Once).end(EndBehavior::Hold));
  play(&context, PlayableScene::new(scene));
  assert_eq!(shown(&context, &addresses)[0], Some(Rgba::red()));

  tick(&context, Duration::from_secs(1));
  assert_eq!(shown(&context, &addresses)[0], None);
}

#[test]
fn positions_are_relative_to_the_plane() {
  // the plane starts at x = 100, so x = 110 is its far edge and x = 5 is off it
  let (context, addresses) = placed_leds(&[at(110.0, 5.0), at(5.0, 5.0)]);
  assert_eq!(played(&context, &addresses, across("Offset", LedQ::every(), red_to_blue())), vec![Some(Rgba::blue()), None]);
}

#[test]
fn leds_off_the_surface_project_straight_onto_it() {
  let (context, addresses) = placed_leds(&[Some(Vec3::new(0.0, 5.0, 1.5))]);
  assert_eq!(played(&context, &addresses, on_plane(LedQ::every(), red_to_blue())), vec![Some(Rgba::red())]);
}

#[test]
fn led_radius_softens_shape_edges() {
  // how much of a circle's edge shows on an LED just inside it
  let coverage = |radius: f32| {
    let (mut context, addresses) = placed_leds(&[at(5.0, 6.95)]);
    context.set_led_radius(radius);
    let scene = on_plane(LedQ::every(), solid(Rgba::red()).region(circle(Vec2::new(5.0, 5.0).into())));
    played(&context, &addresses, scene)[0].unwrap()[3]
  };

  let default = coverage(DEFAULT_LED_RADIUS);
  let soft = coverage(2.0);
  assert!(soft < default && soft > 127, "{soft} vs {default}");
}

#[test]
fn a_layer_on_a_plane_the_app_doesnt_have_shows_nothing() {
  let (context, addresses) = placed_leds(&[at(5.0, 5.0)]);
  assert_eq!(played(&context, &addresses, across("Nowhere", LedQ::every(), solid(Rgba::red()))), vec![None]);
}

#[test]
fn a_higher_z_is_drawn_over_a_lower_one() {
  let (context, addresses) = three_leds();
  play(&context, PlayableScene::new(on(LedQ::name("led0"), solid(Rgba::red()))));
  let over = play(&context, PlayableScene::new(Scene::new(vec![LedLayer::sequence(LedQ::name("led0"), solid(Rgba::blue())).at_z(1).into()])));
  assert_eq!(shown(&context, &addresses)[0], Some(Rgba::blue()));

  with_scenes(&context, |scenes| scenes.remove(over));
  assert_eq!(shown(&context, &addresses)[0], Some(Rgba::red()));
}

fn two_layers(under: &str, over: &str) -> Scene {
  Scene::new(vec![LedLayer::sequence(LedQ::name(under), solid(Rgba::red())).into(), LedLayer::sequence(LedQ::name(over), solid(Rgba::blue())).at_z(2).into()])
    .timing(Timing::new(1.0, Cycle::Once))
}

#[test]
fn layers_play_on_their_own_leds() {
  let (context, addresses) = three_leds();
  assert_eq!(played(&context, &addresses, two_layers("led0", "led2")), vec![Some(Rgba::red()), None, Some(Rgba::blue())]);
}

#[test]
fn layers_run_on_one_clock_and_clear_together() {
  let (context, addresses) = three_leds();
  play(&context, PlayableScene::new(two_layers("led0", "led1")));
  tick(&context, Duration::from_secs(1));
  assert_eq!(shown(&context, &addresses), vec![None, None, None]);
}

#[test]
fn each_layer_spreads_over_its_own_leds() {
  let (context, addresses) = three_leds();
  let scene = Scene::new(vec![
    LedLayer::sequence(LedQ::names(vec!["led0", "led1"]), red_to_blue()).into(),
    LedLayer::sequence(LedQ::name("led2"), red_to_blue()).into(),
  ]);
  // on its own, the third LED is the start of its layer
  assert_eq!(played(&context, &addresses, scene), vec![Some(Rgba::red()), Some(Rgba::blue()), Some(Rgba::red())]);
}

#[test]
fn each_layer_is_measured_against_its_own_plane() {
  // x = 10 is halfway along Wide, and x = 110 is the far edge of Offset, which starts at x = 100
  let (context, addresses) = placed_leds(&[at(10.0, 5.0), at(110.0, 5.0)]);
  let gradient = |leds: &str, plane: &str| LedLayer::area(LedQ::name(leds), plane, red_to_blue()).into();
  let scene = Scene::new(vec![gradient("led0", "Wide"), gradient("led1", "Offset")]);
  assert_eq!(played(&context, &addresses, scene), vec![Some(Rgba([127, 0, 128, 255])), Some(Rgba::blue())]);
}

#[test]
fn a_scene_can_mix_a_sequence_with_an_area() {
  let (context, addresses) = placed_leds(&[at(5.0, 5.0), None]);
  let scene = Scene::new(vec![
    LedLayer::area(LedQ::name("led0"), "Plane", solid(Rgba::red())).into(),
    LedLayer::sequence(LedQ::name("led1"), solid(Rgba::blue())).into(),
  ]);
  assert_eq!(played(&context, &addresses, scene), vec![Some(Rgba::red()), Some(Rgba::blue())]);
}

#[test]
fn layers_on_one_z_blend_rather_than_replace_each_other() {
  let (context, addresses) = three_leds();
  let scene = Scene::new(vec![LedLayer::sequence(LedQ::name("led0"), solid(Rgba::blue())).into(), LedLayer::sequence(LedQ::name("led0"), see_through(Rgba::red())).into()]);
  let shown = played(&context, &addresses, scene)[0].unwrap();
  assert!(shown[0] > 100 && shown[2] > 100, "{shown:?}");
  assert_eq!(shown[3], 255);
}

#[test]
fn a_layer_that_misses_an_led_leaves_it_to_the_others_on_its_z() {
  let (context, addresses) = placed_leds(&[at(5.0, 5.0)]);
  let far_away = Shape::new(ShapeNode::circle(1.0)).transform(Transform::Translate(Vec2::new(50.0, 50.0).into()));
  let scene = Scene::new(vec![
    LedLayer::area(LedQ::every(), "Plane", solid(Rgba::red())).into(),
    LedLayer::area(LedQ::every(), "Plane", solid(Rgba::blue()).region(far_away)).into(),
  ]);
  assert_eq!(played(&context, &addresses, scene), vec![Some(Rgba::red())]);
}

#[test]
fn two_playbacks_of_one_system_on_one_z_are_drawn_in_the_order_they_started() {
  let (context, addresses) = placed_leds(&[at(2.0, 5.0), at(8.0, 5.0)]);
  let left_half = Shape::new(ShapeNode::Rect { width: 5.0.into(), height: 10.0.into() }).transform(Transform::Translate(Vec2::new(2.5, 5.0).into()));
  play(&context, PlayableScene::new(on_plane(LedQ::every(), solid(Rgba::blue()))));
  play(&context, PlayableScene::new(on_plane(LedQ::every(), see_through(Rgba::red()).region(left_half))));

  let shown = shown(&context, &addresses);
  let blended = shown[0].unwrap();
  assert!(blended[0] > 100 && blended[2] > 100, "{blended:?}");
  // where the second misses, the first still shows
  assert_eq!(shown[1], Some(Rgba::blue()));
}

#[test]
fn a_playback_is_drawn_over_declarations_on_its_z() {
  let (context, addresses) = three_leds();
  context.sys_ctx().systems.with::<LedSystem, _>(|leds| leds.declare(OWNER + 1, (&addresses[0], Rgba::blue())));
  play(&context, PlayableScene::new(on(LedQ::name("led0"), see_through(Rgba::red()))));
  let shown = shown(&context, &addresses)[0].unwrap();
  assert!(shown[0] > 100 && shown[2] > 100, "{shown:?}");
}

#[test]
fn an_inactive_owners_playback_gives_its_leds_back() {
  let (context, addresses) = three_leds();
  play(&context, PlayableScene::new(on(LedQ::name("led0"), solid(Rgba::red()))));
  with_scenes(&context, |scenes| scenes.on_event(&SystemDeactivated::new(OWNER, ROOT_GROUP), &context.sys_ctx()));
  assert_eq!(shown(&context, &addresses)[0], None);

  with_scenes(&context, |scenes| scenes.on_event(&SystemReactivated::new(OWNER, ROOT_GROUP), &context.sys_ctx()));
  assert_eq!(shown(&context, &addresses)[0], Some(Rgba::red()));
}

#[test]
fn a_moved_scene_plays_on_the_new_leds_keeping_its_zs() {
  let (context, addresses) = three_leds();
  let scene = Scene::new(vec![
    LedLayer::sequence(LedQ::name("led0"), solid(Rgba::red())).into(),
    LedLayer::sequence(LedQ::name("led0"), solid(Rgba::blue())).at_z(3).into(),
  ]);
  play(&context, PlayableScene::new(scene).on_leds(LedQ::name("led1")).unwrap());
  let shown = shown(&context, &addresses);
  assert_eq!(shown, vec![None, Some(Rgba::blue()), None]);
}

#[test]
fn a_tag_finds_the_leds_that_have_it() {
  let (mut context, addresses) = three_leds();
  let address = LedAddress::new(ExpAddress::default(), 9);
  context.insert_led(LED { name: "tagged".into(), address: address.clone(), tags: vec![Box::new(frontbox_core::tags::Playfield)], ..Default::default() });
  context.sys_ctx().systems.with::<LedSystem, _>(|leds| leds.on_spawn(&context.sys_ctx()));

  play(&context, PlayableScene::new(on(LedQ::tag::<frontbox_core::tags::Playfield>(), solid(Rgba::red()))));
  assert_eq!(shown(&context, &[address])[0], Some(Rgba::red()));
  assert_eq!(shown(&context, &addresses)[0], None);
}

/// Red where traversal "wave" has just been: one edge a second out from `from`, fading over a step
fn wave(from: &str, surface: LedSurface) -> Scene {
  let shown = Prop::Visit(VisitResponse { smooth_out: Some(1.0), ..VisitResponse::new("wave", 0.0, 1.0) });
  let wave = Traversal::new("wave", NodeRef::name(from), TraversalAlgorithm::BreadthFirst { wander: 0.0.into() }, 1.0);
  let fill = Fill::new(FillNode::Solid(Color(Rgba::red()).into())).adjust(ColorAdjust::Opacity(shown));
  let layer = LedLayer { leds: Some(LedQ::every()), z: 0, surface, layer: Layer::new(fill).graph(GraphMask::new(vec![wave])) };
  Scene::new(vec![layer.into()]).timing(Timing::new(10.0, Cycle::Once))
}

fn lit(context: &TestContext, addresses: &[LedAddress]) -> Vec<bool> {
  shown(context, addresses).iter().map(|color| color.is_some_and(|color| color[3] > 127)).collect()
}

/// Four LEDs in a row on the plane, one apart
fn row() -> (TestContext, Vec<LedAddress>) {
  placed_leds(&(0..4).map(|x| at(x as f32 + 1.0, 5.0)).collect::<Vec<_>>())
}

#[test]
fn a_traversal_moves_across_the_leds_it_reaches() {
  let (context, addresses) = row();
  play(&context, PlayableScene::new(wave("led0", LedSurface::Area("Plane".into()))));
  assert_eq!(lit(&context, &addresses), [true, false, false, false]);
  tick(&context, Duration::from_secs(2));
  assert_eq!(lit(&context, &addresses), [false, false, true, false]);
}

#[test]
fn leds_off_the_plane_are_left_out_of_its_graph() {
  // the third is raised above the plane, so the wave can't go through it to the fourth
  let (context, addresses) = placed_leds(&[Some(Vec3::new(1.0, 5.0, 0.0)), Some(Vec3::new(2.0, 5.0, 0.4)), Some(Vec3::new(3.0, 5.0, 2.0)), Some(Vec3::new(4.0, 5.0, 0.0))]);
  play(&context, PlayableScene::new(wave("led0", LedSurface::Area("Plane".into()))));
  tick(&context, Duration::from_secs(1));
  assert_eq!(lit(&context, &addresses), [false, true, false, false]);
  tick(&context, Duration::from_secs(1));
  assert_eq!(lit(&context, &addresses), [false, false, false, true]);
}

#[test]
fn a_sequence_is_a_chain_in_its_order() {
  let (context, addresses) = row();
  play(&context, PlayableScene::new(wave("led3", LedSurface::Sequence)));
  tick(&context, Duration::from_secs(1));
  assert_eq!(lit(&context, &addresses), [false, false, true, false]);
}

#[test]
fn a_playback_from_another_owner_composites_the_same() {
  let (context, addresses) = three_leds();
  play_as(&context, OWNER + 1, PlayableScene::new(on(LedQ::name("led0"), solid(Rgba::blue()))));
  play(&context, PlayableScene::new(on(LedQ::name("led0"), see_through(Rgba::red()))));
  let shown = shown(&context, &addresses)[0].unwrap();
  assert!(shown[0] > 100 && shown[2] > 100, "{shown:?}");
}
