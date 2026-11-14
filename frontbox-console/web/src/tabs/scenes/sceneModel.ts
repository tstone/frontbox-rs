import { contentImageInfo } from '../../state/contentImages'
import type { Color } from '../../types/generated/Color'
import type { ColorAdjust } from '../../types/generated/ColorAdjust'
import type { Curve } from '../../types/generated/Curve'
import type { ImageInfo } from '../../types/generated/ImageInfo'
import type { Key } from '../../types/generated/Key'
import type { Layer } from '../../types/generated/Layer'
import type { Mask } from '../../types/generated/Mask'
import type { NodeRef } from '../../types/generated/NodeRef'
import type { Prop } from '../../types/generated/Prop'
import type { Pick } from '../../types/generated/Pick'
import type { Random } from '../../types/generated/Random'
import type { GraphMask } from '../../types/generated/GraphMask'
import type { LedLayer } from '../../types/generated/LedLayer'
import type { Scene as SceneFile } from '../../types/generated/Scene'
import type { SceneLayer as SceneFileLayer } from '../../types/generated/SceneLayer'
import type { SceneSource } from '../../types/generated/SceneSource'
import type { ShapeNode } from '../../types/generated/ShapeNode'
import type { SoundLayer } from '../../types/generated/SoundLayer'
import type { Speed } from '../../types/generated/Speed'
import type { Spread } from '../../types/generated/Spread'
import type { Sweep } from '../../types/generated/Sweep'
import type { Timing } from '../../types/generated/Timing'
import type { Track } from '../../types/generated/Track'
import type { Value } from '../../types/generated/Value'
import type { VisitResponse } from '../../types/generated/VisitResponse'
import type { Transform } from '../../types/generated/Transform'
import type { Traversal } from '../../types/generated/Traversal'
import type { TraversalAlgorithm } from '../../types/generated/TraversalAlgorithm'

export type Vec2 = [number, number]

type Shape = NonNullable<Layer['region']>

/** The size of what a layer is being edited for, in that device's units. A `Sequence` has none. */
export type Surface = Vec2 | null

/** Says what a layer is measured against, as a scene's layers can each be on something different */
export type LayerSurfaces = (layer: SceneLayer) => Surface

/** What a `Prop` can hold */
export type SceneValue = number | Vec2 | Color

/** The span a slider covers. The number box next to it can go past this. */
export type Range = {
  min: number
  max: number
  step: number
  /** Where a new random spread ends, for when that isn't `max` */
  spreadEnd?: number
  /** What double clicking the slider's handle resets it to. Without one, there's no value to go back to. */
  rest?: number
}

/** Times are kept to this, in seconds, as a key at 0.4183 is only ever an accident of the mouse */
export const TIME_STEP = 0.01

export const snap = (seconds: number) => Math.round(seconds / TIME_STEP) * TIME_STEP

/** A range for each axis. A plain number uses the first. */
export type Ranges = [Range, Range]

/** Which space a transform moves things in: a shape's is the viewport, a fill's is its unit box */
export type TransformSpace = 'shape' | 'fill'

/**
 * A place in the scene that can be read and replaced. Fields take a lens rather than a value and a callback so that
 * an editor can be handed any part of the scene and doesn't need to know where it sits.
 */
export type Lens<T> = {
  get(): T
  set(value: T): void
}

type KeysOf<T> = T extends object ? keyof T : never
type VariantOf<T, K extends PropertyKey> = T extends Record<K, infer V> ? V : never

export function field<T, K extends keyof T>(lens: Lens<T>, key: K): Lens<T[K]> {
  return {
    get: () => lens.get()[key],
    set: (value) => lens.set({ ...lens.get(), [key]: value }),
  }
}

export function item<T>(lens: Lens<T[]>, index: number): Lens<T> {
  return {
    get: () => lens.get()[index],
    set: (value) => lens.set(lens.get().map((existing, i) => (i === index ? value : existing))),
  }
}

/** A list the format leaves out when it's empty */
export function orEmpty<T>(lens: Lens<T[] | undefined>): Lens<T[]> {
  return { get: () => lens.get() ?? [], set: lens.set }
}

/** A value the format can leave out, which reads as `fallback` until it's set */
export function orElse<T>(lens: Lens<T | undefined>, fallback: T): Lens<T> {
  return { get: () => lens.get() ?? fallback, set: lens.set }
}

/** What's inside an enum variant. Only for use while the enum is that variant. */
export function variant<T, K extends KeysOf<T>>(lens: Lens<T>, key: K): Lens<VariantOf<T, K>> {
  return {
    get: () => (lens.get() as Record<K, VariantOf<T, K>>)[key],
    set: (value) => lens.set({ [key]: value } as T),
  }
}

export function removed<T>(list: T[], index: number): T[] {
  return list.filter((_, i) => i !== index)
}

export function moved<T>(list: T[], index: number, by: number): T[] {
  const next = [...list]
  next.splice(index + by, 0, ...next.splice(index, 1))
  return next
}

/** The value a param holds, as `Value` tags each kind */
export function wrapValue(value: SceneValue): Value {
  if (typeof value === 'number') return { Float: value }
  return value.length === 2 ? { Vec2: value } : { Color: value }
}

export function unwrapValue(value: Value): SceneValue {
  if ('Float' in value) return value.Float
  return 'Vec2' in value ? value.Vec2 : value.Color
}

/** Whether a param can feed a prop holding values like `like`. A param of the wrong type gives the type's default. */
export function paramFits(param: Value | undefined, like: SceneValue) {
  return param !== undefined && Object.keys(param)[0] === Object.keys(wrapValue(like))[0]
}

/** Whether a random prop holding values like `like` can pick from a spread. One of the wrong type gives the type's default. */
export function spreadFits(spread: Spread, like: SceneValue) {
  return Object.keys(spread)[0] === Object.keys(wrapValue(like))[0]
}

const midpoint = ([low, high]: [number, number]) => (low + high) / 2

/** The fully saturated color at a hue in degrees, and a brightness and an alpha from 0 to 1 */
function hsvColor(hue: number, brightness: number, alpha: number): Color {
  const channel = (n: number) => {
    const k = (n + hue / 60) % 6
    return Math.round(255 * brightness * (1 - Math.max(0, Math.min(k, 4 - k, 1))))
  }
  return [channel(5), channel(3), channel(1), Math.round(alpha * 255)]
}

/** The middle of what a spread can pick */
function spreadMiddle(spread: Spread): SceneValue {
  if ('Float' in spread) return midpoint(spread.Float)
  if ('Vec2' in spread) return [midpoint(spread.Vec2.x), midpoint(spread.Vec2.y)]
  const { hue, brightness, alpha } = spread.Color
  return hsvColor(midpoint(hue), midpoint(brightness), midpoint(alpha))
}

/** The widest spread for a prop holding values like `like`, short of making a color see-through */
export function fullSpread(like: SceneValue, [x, y]: Ranges): Spread {
  if (typeof like === 'number') return { Float: [x.min, x.spreadEnd ?? x.max] }
  if (like.length === 2) return { Vec2: { x: [x.min, x.max], y: [y.min, y.max] } }
  return { Color: { hue: [HUE.min, HUE.max], brightness: [UNIT.min, UNIT.max], alpha: [UNIT.max, UNIT.max] } }
}

/** The value a prop shows with no game running: the parameter's default, a track's first key, the middle of a spread, where a path was moved to, or a visit response at rest */
export function startingValue<T extends SceneValue>(prop: Prop<T>, scene: Scene, fallback: T): T {
  if ('Fixed' in prop) return prop.Fixed
  if ('Visit' in prop) return prop.Visit.from
  if ('Track' in prop) return prop.Track.keys[0]?.value ?? fallback
  if ('Random' in prop) return spreadFits(prop.Random.spread, fallback) ? (spreadMiddle(prop.Random.spread) as T) : fallback
  if ('Sweep' in prop) {
    const moved = prop.Sweep.path.transform?.find((transform) => 'Translate' in transform)
    return moved && 'Translate' in moved ? (startingValue(moved.Translate, scene, fallback as Vec2) as T) : fallback
  }
  const param = scene.params?.[prop.Param]
  return paramFits(param, fallback) ? (unwrapValue(param!) as T) : fallback
}

/** `base`, or `base_2` and so on if a parameter already has that name */
export function freshParamName(scene: Scene, base: string) {
  const taken = (name: string) => scene.params?.[name] !== undefined
  if (!taken(base)) return base
  let n = 2
  while (taken(`${base}_${n}`)) n++
  return `${base}_${n}`
}

/** The points of a shape made of them, which is none for the others */
export function shapePoints(base: ShapeNode): Prop<Vec2>[] {
  if ('Polygon' in base) return base.Polygon.points
  if ('Polyline' in base) return base.Polyline.points
  return 'Bezier' in base ? base.Bezier.points.map(({ point }) => point) : []
}

/** Whether a shape is a line that stops at its last point, and not an edge that comes back around to its first */
export const isOpen = (base: ShapeNode) => 'Polyline' in base || 'Bezier' in base

/** A circle around `center` for the whole of a scene's length, big enough to see in the ranges of the prop on it */
export function newSweep(center: Vec2, [x, y]: Ranges, duration: number): Sweep {
  const reach = Math.min(x.max - x.min, y.max - y.min) / 4
  const base = { Circle: { radius: { Fixed: reach } } }
  return { path: { base, transform: [{ Translate: { Fixed: center } }] }, start: 0, end: duration }
}

/** Whether a scene plays more than once, which gives its first moment something to come from and its last somewhere to go */
export function repeats(scene: Scene) {
  const cycle = scene.timing?.cycle
  return cycle === 'Forever' || (typeof cycle === 'object' && cycle.Times > 1)
}

/** How many times a scene's cycle plays */
export function cycleCount({ cycle }: Timing) {
  return cycle === 'Once' ? 1 : cycle === 'Forever' ? Infinity : cycle.Times
}

/** Whether a scene's cycles differ from each other, which only random picks can make them do */
export function changesEachCycle(scene: Scene) {
  return repeats(scene) && propUses(reading(scene)).some(({ lens }) => 'Random' in lens.get())
}

/** Scene with a timing, which it needs before it can have keyframes. A new one is `duration` long and repeats. */
export function withTiming(scene: Scene, duration = 1): Scene {
  return scene.timing ? scene : { ...scene, timing: { duration, cycle: 'Forever' } }
}

/** How long an image takes to play on the timeline, if its file says */
export const imageLength = ({ duration }: ImageInfo) => snap(duration) || undefined

/**
 * A track that plays an animated image once at the speed its file says, or as fast as it takes to end with the timeline.
 * It goes up to the frame count rather than the last frame, as that gives every frame the same time.
 */
export function framesTrack(info: ImageInfo, { duration }: Timing): Prop<number> {
  const end = Math.min(imageLength(info) ?? duration, duration)
  const keys: Key<number>[] = [
    { at: 0, value: 0, curve: 'Linear' },
    { at: end, value: info.frames, curve: 'Linear' },
  ]
  return { Track: { keys, cycle: 'Once' } }
}

/** Scene with a parameter added or changed */
export function withParam(scene: Scene, name: string, value: SceneValue): Scene {
  return { ...scene, params: { ...scene.params, [name]: wrapValue(value) } }
}

/** The plane an `Area` layer covers, which is `null` for a `Sequence` or a layer of a DMD */
export function layerPlane({ surface }: SceneLayer) {
  return surface === undefined || surface === 'Sequence' ? null : surface.Area
}

/** Where an algorithm that goes to a node is going, which is `null` for the others */
export function traversalTarget(algorithm: Lens<TraversalAlgorithm>): Lens<NodeRef> | null {
  const current = algorithm.get()
  if (typeof current === 'string') return null
  const [[kind, settings]] = Object.entries(current)
  // the wandering algorithms have settings too, but nowhere to go
  if (!('to' in settings)) return null
  const now = () => Object.values(algorithm.get())[0] as { to: NodeRef }
  return {
    get: () => now().to,
    set: (to) => algorithm.set({ [kind]: { ...now(), to } } as TraversalAlgorithm),
  }
}

/** The names of a layer's traversals, which is what a visit response or a fit refers to them by */
export const traversalNames = (layer: SceneLayer | undefined) => layer?.graph?.traversals.map(({ name }) => name) ?? []

/** `base`, or `base_2` and so on if one of the layer's traversals already has that name */
export function freshTraversalName(layer: SceneLayer, base: string) {
  const taken = new Set(traversalNames(layer))
  if (!taken.has(base)) return base
  let n = 2
  while (taken.has(`${base}_${n}`)) n++
  return `${base}_${n}`
}

/** Where a new layer goes: where the one under it is, so that stacking is the easy thing to do */
export function newLayerPlace(scene: Scene): Omit<SceneLayer, keyof Layer> {
  const under = scene.layers.at(-1)
  if (scene.kind === 'Dmd') return { display: under?.display ?? 'Dmd', z: under?.z }
  return { leds: under?.leds ?? 'Every', z: under?.z, surface: under?.surface ?? 'Sequence' }
}

/** What code can do with a scene's LEDs: nothing as it can't play yet, replace them all, or leave them as they are */
export function ledsCase(scene: Scene) {
  const layers = scene.layers.filter((layer) => layer.display === undefined)
  if (layers.some((layer) => layer.leds === undefined)) return 'None'
  const leds = new Set(layers.map((layer) => JSON.stringify(layer.leds)))
  return leds.size <= 1 ? 'Replaceable' : 'Fixed'
}

type PropLens = Lens<Prop<SceneValue>>

/** The layer open in the config, and whether it's the layer's mask being edited rather than the layer */
export type Selection = { layer: number; mask: boolean }

/** The region of what's selected, if it has one */
export function selectedShape(scene: Scene | null | undefined, { layer, mask }: Selection) {
  const current = scene?.layers[layer]
  return (mask ? current?.mask : current)?.region
}

/** Where to change the region of what's selected, which has to have one */
export function selectedShapeLens(scene: Lens<Scene>, { layer, mask }: Selection) {
  const lens = item(field(scene, 'layers'), layer)
  return (mask ? field(field(lens, 'mask') as Lens<Mask>, 'region') : field(lens, 'region')) as Lens<Shape>
}

/** Every prop in a scene: where it is, which layer it's in and whether that's in its mask, what to call it, and the ranges its sliders use */
function propUses(scene: Lens<Scene>, surfaces: LayerSurfaces = () => null) {
  const uses: { lens: PropLens; layer: number | null; mask: boolean; label: string; ranges: Ranges }[] = []
  // `null` for a sound layer, which isn't among the drawn layers
  let layerIndex: number | null = 0
  let inMask = false
  let surface: Surface = null
  const use = (lens: unknown, label: string, ranges: Ranges) => {
    uses.push({ lens: lens as PropLens, layer: layerIndex, mask: inMask, label, ranges })
    if ('Sweep' in (lens as PropLens).get()) {
      const path = `${label} · Path`
      shape(field(variant(lens as PropLens, 'Sweep'), 'path'), path, path)
    }
  }

  const transforms = (transform: Lens<Transform[]>, label: string, space: TransformSpace) => {
    transform.get().forEach((w, index) => {
      const lens = item(transform, index)
      if ('Translate' in w) use(variant(lens, 'Translate'), `${label} · Offset`, translateRanges(surface, space))
      else if ('Scale' in w) use(variant(lens, 'Scale'), `${label} · Scale`, scaleRanges())
      else if ('Rotate' in w) use(variant(lens, 'Rotate'), `${label} · Rotation`, both(ROTATION))
    })
  }

  const shape = (lens: Lens<Shape>, label: string, transformLabel: string) => {
    transforms(orEmpty(field(lens, 'transform')), transformLabel, 'shape')
    const base = field(lens, 'base')
    const current = base.get()
    const [across, down] = sizeRanges(surface)
    if ('Circle' in current) {
      use(field(variant(base, 'Circle'), 'radius'), `${label} · Radius`, both(roundRange(surface)))
    } else if ('Ring' in current) {
      const ring = variant(base, 'Ring')
      use(field(ring, 'inner_radius'), `${label} · Inner radius`, both(roundRange(surface)))
      use(field(ring, 'thickness'), `${label} · Thickness`, both(roundRange(surface)))
    } else if ('Ellipse' in current) {
      const ellipse = variant(base, 'Ellipse')
      use(field(ellipse, 'rx'), `${label} · RX`, both(radius(across)))
      use(field(ellipse, 'ry'), `${label} · RY`, both(radius(down)))
    } else if ('Rect' in current) {
      const rect = variant(base, 'Rect')
      use(field(rect, 'width'), `${label} · Width`, both(across))
      use(field(rect, 'height'), `${label} · Height`, both(down))
    } else if ('Pinwheel' in current) {
      const pinwheel = variant(base, 'Pinwheel')
      use(field(pinwheel, 'blades'), `${label} · Blades`, both(BLADES))
      use(field(pinwheel, 'blade_fraction'), `${label} · Blade fraction`, both(BLADE_FRACTION))
    } else if ('Bezier' in current) {
      const points = field(variant(base, 'Bezier'), 'points')
      points.get().forEach((_, i) => {
        use(field(item(points, i), 'point'), `${label} · Point ${i + 1}`, pointRanges(surface))
        use(field(item(points, i), 'handle'), `${label} · Point ${i + 1} · Handle`, pointRanges(surface))
      })
    } else {
      const points = 'Polygon' in current ? field(variant(base, 'Polygon'), 'points') : field(variant(base, 'Polyline'), 'points')
      points.get().forEach((_, i) => use(item(points, i), `${label} · Point ${i + 1}`, pointRanges(surface)))
    }
  }

  const fill = (lens: Lens<Layer['fill']>, label: string) => {
    transforms(orEmpty(field(lens, 'transform')), `${label} · Fill`, 'fill')
    const base = field(lens, 'base')
    const current = base.get()
    if ('Solid' in current) {
      use(variant(base, 'Solid'), `${label} · Color`, both(UNIT))
    } else if ('Gradient' in current) {
      const stops = field(variant(base, 'Gradient'), 'stops')
      stops.get().forEach((_, stop) => use(field(item(stops, stop), 'color'), `${label} · Stop ${stop + 1}`, both(UNIT)))
    } else if ('Image' in current && current.Image.frame) {
      const frames = contentImageInfo(current.Image.name)?.frames
      use(field(variant(base, 'Image'), 'frame'), `${label} · Frame`, frameRanges(frames))
    }
    const adjust = orEmpty(field(lens, 'adjust'))
    adjust.get().forEach((current, i) => {
      const { label: name, range } = ADJUSTS[adjustKind(current)]
      use(adjustAmount(item(adjust, i)), `${label} · ${name}`, both(range))
    })
  }

  const graph = (lens: Lens<GraphMask>, label: string) => {
    const traversals = field(lens, 'traversals')
    traversals.get().forEach(({ name, from }, i) => {
      const traversal = item(traversals, i)
      const named = `${label} · ${name}`
      const speed = field(traversal, 'speed')
      if ('Rate' in speed.get()) use(variant(speed as Lens<Extract<Speed, { Rate: unknown }>>, 'Rate'), `${named} · Rate`, both(RATE))
      if ('Nearest' in from) use(variant(field(traversal, 'from'), 'Nearest'), `${named} · From`, translateRanges(surface, 'shape'))
      const algorithm = field(traversal, 'algorithm')
      const current = algorithm.get()
      if (typeof current !== 'string') {
        const [kind, settings] = Object.entries(current)[0]
        const wandering = algorithm as unknown as Lens<Record<string, { wander: Prop<number> }>>
        if ('wander' in settings) use(field(variant(wandering, kind), 'wander'), `${named} · Wander`, both(WANDER))
        if ('FanOut' in current) {
          const fan = variant(algorithm as Lens<Extract<TraversalAlgorithm, { FanOut: unknown }>>, 'FanOut')
          use(field(fan, 'angle'), `${named} · Arc angle`, both(FAN_ANGLE))
          use(field(fan, 'count'), `${named} · Additional paths`, both(FAN_COUNT))
        }
        if ('Growth' in current) {
          const growth = variant(algorithm as Lens<Extract<TraversalAlgorithm, { Growth: unknown }>>, 'Growth')
          use(field(growth, 'reach'), `${named} · Reach`, both(reachRange(surface)))
          use(field(growth, 'density'), `${named} · Density`, both(GROWTH_DENSITY))
          use(field(growth, 'branching'), `${named} · Branching`, both(BRANCHING))
        }
      }
      const target = traversalTarget(algorithm)
      const to = target?.get()
      if (target && to && 'Nearest' in to) use(variant(target, 'Nearest'), `${named} · To`, translateRanges(surface, 'shape'))
    })
  }

  const layers = field(scene, 'layers')
  layers.get().forEach(({ region, mask, graph: layerGraph }, index) => {
    layerIndex = index
    inMask = false
    surface = surfaces(layers.get()[index])
    const layer = item(layers, index)
    const label = layerLabel(layers.get()[index], index)

    fill(field(layer, 'fill'), label)
    if (region) shape(field(layer, 'region') as Lens<Shape>, label, `${label} · Region`)
    if (layerGraph) graph(field(layer, 'graph') as Lens<GraphMask>, `${label} · Graph`)

    if (mask) {
      inMask = true
      const lens = field(layer, 'mask') as Lens<Mask>
      const maskLabel = `${label} · Mask`
      fill(field(lens, 'fill'), maskLabel)
      if (mask.region) shape(field(lens, 'region') as Lens<Shape>, maskLabel, `${maskLabel} · Region`)
    }
  })

  const sounds = field(scene, 'sounds')
  layerIndex = null
  inMask = false
  sounds.get().forEach((layer, index) => use(soundVolume(item(sounds, index)), `${soundLabel(layer, index)} · Volume`, both(VOLUME)))
  return uses
}

/** A sound layer's volume, which is 1 when the file leaves it out */
export function soundVolume(layer: Lens<SoundLayer>): Lens<Prop<number>> {
  const volume = field(layer, 'volume')
  return { get: () => volume.get() ?? { Fixed: 1 }, set: volume.set }
}

/** What to call a layer, which is its place from the bottom until it's given a name */
export const layerLabel = (layer: SceneLayer, index: number) => layer.name ?? `Layer ${index + 1}`

const reading = (scene: Scene): Lens<Scene> => ({ get: () => scene, set: () => {} })

/** The names of the parameters a scene's props read */
function referencedParams(scene: Scene) {
  const names = new Set<string>()
  for (const { lens } of propUses(reading(scene))) {
    const prop = lens.get()
    if ('Param' in prop) names.add(prop.Param)
  }
  return names
}

/** A prop on the timeline. `layer` is `null` for a sound layer's. */
export type SceneTrack = { prop: PropLens; layer: number | null; mask: boolean; label: string; ranges: Ranges }

/** A traversal on the timeline: its layer, and its place among the layer's traversals */
export type SceneTraversal = { traversal: Lens<Traversal>; layer: number; index: number; label: string }

/** Every traversal in a scene, as each plays out along the timeline */
export function sceneTraversals(scene: Lens<Scene>): SceneTraversal[] {
  const layers = field(scene, 'layers')
  return layers.get().flatMap((layer, layerIndex) => {
    if (!layer.graph) return []
    const traversals = field(field(item(layers, layerIndex), 'graph') as Lens<GraphMask>, 'traversals')
    const label = `${layerLabel(layer, layerIndex)} · Graph`
    return layer.graph.traversals.map(({ name }, index) => ({ traversal: item(traversals, index), layer: layerIndex, index, label: `${label} · ${name}` }))
  })
}

/**
 * Seconds a traversal takes from its start: its time to finish in, or for one with a rate, `measured` by the machine
 * from the graph. `null` with nothing measured, as for one that visits nothing.
 */
export function traversalLength(traversal: Traversal, scene: Scene, measured: number | null): number | null {
  const { speed, start } = traversal
  if ('Rate' in speed) return measured
  return speed.Duration ?? Math.max((scene.timing?.duration ?? 0) - start, 0)
}

/** Every prop in a scene that changes along the timeline */
export function sceneTracks(scene: Lens<Scene>, surfaces?: LayerSurfaces): SceneTrack[] {
  return propUses(scene, surfaces).flatMap(({ lens, layer, mask, label, ranges }) => {
    const prop = lens.get()
    return 'Track' in prop || 'Random' in prop || 'Sweep' in prop ? [{ prop: lens, layer, mask, label, ranges }] : []
  })
}

/** A time no scene gets to, which a sweep waits for at the start of its path */
const NEVER = 1e30

/**
 * Whether a track is set to stay put. That's only for the editing and isn't part of the scene: it's on the track so that
 * it goes where the track goes, the machine pays it no attention, and a scene read back from RON doesn't have it.
 */
export const isHidden = (prop: Prop<SceneValue>) => (Object.values(prop)[0] as { hidden?: boolean }).hidden === true

/** `prop`, a track, shown or hidden */
export function withHidden(prop: Prop<SceneValue>, hidden: boolean) {
  const [[kind, track]] = Object.entries(prop)
  return { [kind]: { ...track, hidden: hidden || undefined } } as Prop<SceneValue>
}

/** A spread that can only pick its middle */
function middleOf(spread: Spread): Spread {
  const middle = (range: [number, number]): [number, number] => [midpoint(range), midpoint(range)]
  if ('Float' in spread) return { Float: middle(spread.Float) }
  if ('Vec2' in spread) return { Vec2: { x: middle(spread.Vec2.x), y: middle(spread.Vec2.y) } }
  const { hue, brightness, alpha } = spread.Color
  return { Color: { hue: middle(hue), brightness: middle(brightness), alpha: middle(alpha) } }
}

/**
 * A track that stays on its first value. Only the machine knows what a random prop's first pick comes to, so that stays
 * on the middle of its spread, as it does for anything else that needs a value for it here.
 */
function held(prop: Prop<SceneValue>): Prop<SceneValue> {
  if ('Sweep' in prop) return { Sweep: { ...prop.Sweep, start: NEVER, end: 2 * NEVER } }
  if ('Random' in prop) return { Random: { ...prop.Random, spread: middleOf(prop.Random.spread), force_unique: false } }
  return 'Track' in prop && prop.Track.keys.length > 0 ? { Fixed: prop.Track.keys[0].value } : prop
}

/**
 * A layer as the editor works on it, with what it's drawn on alongside what it draws: LEDs, with `leds`, `surface`, and
 * `z`, or a display, named by `display`.
 */
export type SceneLayer = Layer & Partial<Omit<LedLayer, 'layer'>> & { display?: string }

/**
 * A scene as the editor works on it, with everything about a layer in one place. `kind` is what the editor shows it on:
 * one with display layers is edited as a display's, and one without as the LEDs'. Its sound layers are apart from the
 * drawn ones in `layers`, as nothing about them is drawn, and go after them in the file.
 */
export type Scene = Omit<SceneFile, 'layers'> & { kind: 'Led' | 'Dmd'; layers: SceneLayer[]; sounds: SoundLayer[] }

export function editedScene({ layers, ...scene }: SceneFile): Scene {
  const edited = layers.flatMap((on): SceneLayer[] => {
    if ('Sound' in on) return []
    if ('Display' in on) {
      const { layer, ...place } = on.Display
      return [{ ...layer, ...place }]
    }
    const { layer, ...place } = on.Leds
    return [{ ...layer, ...place }]
  })
  const sounds = layers.flatMap((on) => ('Sound' in on ? [on.Sound] : []))
  return { ...scene, kind: layers.some((on) => 'Display' in on) ? 'Dmd' : 'Led', layers: edited, sounds }
}

function sceneFile({ kind, layers, sounds, ...scene }: Scene): SceneFile {
  const placed = layers.map(({ leds, z, surface, display, ...layer }): SceneFileLayer =>
    display !== undefined ? { Display: { display, z, layer } } : { Leds: { leds, z, surface: surface ?? 'Sequence', layer } },
  )
  return { ...scene, layers: [...placed, ...sounds.map((sound): SceneFileLayer => ({ Sound: sound }))] }
}

/** What to call a sound layer: its name, or the file it plays without the folder and extension */
export const soundLabel = (layer: SoundLayer, index: number) =>
  layer.name ?? (layer.sound.split('/').at(-1)!.replace(/\.[^.]+$/, '') || `Sound ${index + 1}`)

/** Where on the timeline a sound layer's sound would end if it weren't cut off, if `length` says how long the sound is */
export const soundNaturalEnd = (layer: SoundLayer, length: number | null | undefined) => (length == null ? undefined : layer.start + length - (layer.skip ?? 0))

/** Where a sound layer plays to: where it's cut off, or else the end of its sound, if `length` says how long that is */
export const soundEnd = (layer: SoundLayer, length: number | null | undefined) => layer.end ?? soundNaturalEnd(layer, length)

/** `scene` with each of its sound layers changed by `edit` */
function withSounds(scene: Scene, edit: (layer: SoundLayer) => SoundLayer): Scene {
  return { ...scene, sounds: scene.sounds.map(edit) }
}

/** How to ask the machine for a scene: as it stands, or if any of its tracks are hidden, as it looks without them moving */
export function sceneSource(scene: Scene): SceneSource {
  let shown = scene
  const tracks = sceneTracks({ get: () => shown, set: (edited) => (shown = edited) })
  for (const { prop } of tracks) {
    if (isHidden(prop.get())) prop.set(held(prop.get()))
  }
  const definition = sceneFile(scene)
  return shown === scene ? { Definition: definition } : { Preview: { scene: definition, shown: sceneFile(shown) } }
}

/** When a prop changes, in order: a track's keys, the moments a value is picked, or the two ends of a sweep */
export function trackTimes(prop: Prop<SceneValue>) {
  if ('Track' in prop) return prop.Track.keys.map((key) => key.at)
  if ('Sweep' in prop) return [prop.Sweep.start, prop.Sweep.end]
  return 'Random' in prop ? prop.Random.picks.map((pick) => pick.at) : []
}

/** `scene` with new times for what's on its timeline: the keys and picks of a prop, and the two ends of a sweep */
function withTimes(
  scene: Scene,
  moments: <T extends { at: number }>(list: T[]) => T[],
  ends: (sweep: Sweep) => [number, number],
): Scene {
  let next = scene
  const tracks = sceneTracks({ get: () => next, set: (edited) => (next = edited) })
  for (const { prop } of tracks) {
    const current = prop.get()
    if ('Track' in current) {
      prop.set({ Track: { ...current.Track, keys: moments(current.Track.keys) } })
    } else if ('Random' in current) {
      prop.set({ Random: { ...current.Random, picks: moments(current.Random.picks) } })
    } else if ('Sweep' in current) {
      const [start, end] = ends(current.Sweep)
      prop.set({ Sweep: { ...current.Sweep, start, end } })
    }
  }
  return next
}

/** `scene` with each traversal changed by `edit` */
function withTraversals(scene: Scene, edit: (traversal: Traversal) => Traversal): Scene {
  let next = scene
  for (const { traversal } of sceneTraversals({ get: () => next, set: (edited) => (next = edited) })) traversal.set(edit(traversal.get()))
  return next
}

/** `moments` with the last of them at `length` if any are past it, and those it comes up against kept a step apart */
function pulledBack<T extends { at: number }>(moments: T[], length: number): T[] {
  if (moments.every(({ at }) => at <= length)) return moments
  const pulled = [...moments]
  let latest = length
  for (let index = pulled.length - 1; index >= 0; index--) {
    latest = Math.max(Math.min(pulled[index].at, latest), 0)
    pulled[index] = { ...pulled[index], at: latest }
    latest -= TIME_STEP
  }
  return pulled
}

/** `scene` with a timeline `duration` long, as if it were typed in as the length */
export function withDuration(scene: Scene, duration: number): Scene {
  const timed = withTiming(scene, duration)
  return withTimesInLength({ ...timed, timing: { ...timed.timing!, duration } })
}

/** `scene` with what's past the end of its timeline brought back to the end */
export function withTimesInLength(scene: Scene): Scene {
  const length = scene.timing?.duration
  if (length === undefined) return scene
  const timed = withTimes(
    scene,
    (moments) => pulledBack(moments, length),
    ({ start, end }) => (end <= length ? [start, end] : [Math.min(start, Math.max(length - TIME_STEP, 0)), length]),
  )
  // one that runs past the end is left to show that it does
  const traversed = withTraversals(timed, (traversal) => (traversal.start < length ? traversal : { ...traversal, start: Math.max(length - TIME_STEP, 0) }))
  // a sound has to start before the end, as one at the end is the start of the next cycle
  return withSounds(traversed, (layer) => ({ ...layer, start: Math.min(layer.start, Math.max(length - TIME_STEP, 0)) }))
}

/** `scene` with its length and everything on its timeline scaled by `by`. What's past the end stays past a longer one. */
export function withTimesScaled(scene: Scene, by: number): Scene {
  const timed = withTimes(
    scene,
    (moments) => moments.map((moment) => ({ ...moment, at: moment.at * by })),
    ({ start, end }) => [start * by, end * by],
  )
  // a rate is left as it is, as it's in steps and not seconds
  const scaled = withTraversals(timed, ({ start, speed, ...traversal }) => ({
    ...traversal,
    start: start * by,
    speed: 'Duration' in speed && speed.Duration !== null ? { Duration: speed.Duration * by } : speed,
  }))
  const soundsScaled = withSounds(scaled, ({ start, end, ...layer }) => ({ ...layer, start: start * by, ...(end !== undefined && { end: end * by }) }))
  const next = { ...soundsScaled, timing: soundsScaled.timing && { ...soundsScaled.timing, duration: soundsScaled.timing.duration * by } }
  return by < 1 ? withTimesInLength(next) : next
}

/**
 * `next` without its timing once `current`'s last track is gone, as the timing was only there for them and would
 * otherwise end a scene that's meant to stay up. A sound starting after the start keeps it, as it needs one to start in.
 */
export function withoutDroppedTiming(current: Scene, next: Scene): Scene {
  const tracks = (scene: Scene) => sceneTracks(reading(scene)).length
  const startsLater = next.sounds.some(({ start }) => start > 0)
  if (!next.timing || tracks(current) === 0 || tracks(next) > 0 || traverses(next) || startsLater) return next
  const { timing: _, ...rest } = next
  return rest
}

/** Whether a scene has traversals or responds to them, which plays out over time as keyframes do */
function traverses(scene: Scene) {
  return scene.layers.some((layer) => traversalNames(layer).length > 0) || propUses(reading(scene)).some(({ lens }) => 'Visit' in lens.get())
}

function lerp(a: SceneValue, b: SceneValue, by: number): SceneValue {
  if (typeof a === 'number') return a + ((b as number) - a) * by
  const mixed = a.map((channel, i) => (lerp(channel, (b as number[])[i], by) as number))
  return (a.length === 4 ? mixed.map(Math.round) : mixed) as SceneValue
}

/**
 * `track` with a key added at a time, so that the scene looks as it did. Between two keys that's the value partway
 * from one to the next as if by a straight line, which is close enough for a curve to start from.
 */
export function withKey(track: Track<SceneValue>, at: number): Track<SceneValue> {
  const { keys } = track
  const after = keys.findIndex((key) => key.at > at)
  const index = after === -1 ? keys.length : after
  const [before, next] = [keys[index - 1], keys[index]]
  const value = !before ? next.value : !next ? before.value : lerp(before.value, next.value, (at - before.at) / (next.at - before.at))
  // the stretch up to the new key is still the one `next` was curving
  const curve: Curve = next?.curve ?? 'Linear'
  return { ...track, keys: [...keys.slice(0, index), { at, value, curve }, ...keys.slice(index)] }
}

/**
 * Moves the moment at `index` to a time, passing any others on the way as the format has them in order, and says where
 * it's ended up. A time another is already at is skipped over, as two dots there couldn't be told apart.
 */
export function moveMoment<T extends { at: number }>(moments: Lens<T[]>, index: number, at: number) {
  const list = moments.get()
  const others = removed(list, index)
  if (others.some((other) => Math.abs(other.at - at) < TIME_STEP / 2)) return index
  const place = others.filter((other) => other.at < at).length
  moments.set([...others.slice(0, place), { ...list[index], at }, ...others.slice(place)])
  return place
}

/** `keys` with a copy of the one at `index` at another time */
export function withKeyCopy(keys: Key<SceneValue>[], index: number, at: number): Key<SceneValue>[] {
  return [...keys, { ...keys[index], at }].sort((a, b) => a.at - b.at)
}

/**
 * A response to `traversal` where a number rests at 0 and goes to the top of its range on each visit, taking a step to
 * get there, a step there, and a step back. A position or a color rests at `current`, and goes nowhere until it's given
 * somewhere to go.
 */
export function newVisit(traversal: string, current: SceneValue, [range]: Ranges): VisitResponse<SceneValue> {
  const [from, to] = typeof current === 'number' ? [0, range.max] : [current, current]
  return { traversal, from, to, smooth_in: 1, hold: 1, smooth_out: 1, curve_in: 'Linear', curve_out: 'Linear', overlap: 'Max' }
}

/**
 * `layer` with every visit response and fit that refers to a traversal by `from` referring to it by `to` instead, for
 * when a traversal is renamed
 */
export function withTraversalRenamed(layer: SceneLayer, from: string, to: string): SceneLayer {
  const renamed = (value: unknown): unknown => {
    if (Array.isArray(value)) return value.map(renamed)
    if (value === null || typeof value !== 'object') return value
    const entries = Object.entries(value).map(([key, inner]) => {
      if (key === 'Fit' && inner === from) return [key, to]
      if (key === 'Visit' && (inner as VisitResponse<SceneValue>).traversal === from) return [key, { ...(inner as object), traversal: to }]
      return [key, renamed(inner)]
    })
    return Object.fromEntries(entries)
  }
  const { graph, ...rest } = layer
  const traversals = graph?.traversals.map((traversal) => (traversal.name === from ? { ...traversal, name: to } : traversal))
  return { ...(renamed(rest) as SceneLayer), ...(graph && { graph: { ...graph, traversals: traversals! } }) }
}

/** The traversals something in `layer` responds to or fits to */
export function usedTraversals(layer: SceneLayer): Set<string> {
  const used = new Set<string>()
  const visit = (value: unknown) => {
    if (Array.isArray(value)) return value.forEach(visit)
    if (value === null || typeof value !== 'object') return
    for (const [key, inner] of Object.entries(value)) {
      if (key === 'Fit' && typeof inner === 'string') used.add(inner)
      if (key === 'Visit') used.add((inner as VisitResponse<SceneValue>).traversal)
      visit(inner)
    }
  }
  const { graph: _, ...rest } = layer
  visit(rest)
  return used
}

/** A moment to pick a value, held until the next one */
export function newPick(at: number): Pick {
  return { at, smooth_in: 0, smooth_out: 0 }
}

/** `random` with one more moment to pick a value */
export function withPick(random: Random, at: number): Random {
  return { ...random, picks: [...random.picks, newPick(at)].sort((a, b) => a.at - b.at) }
}

/**
 * `next` without the parameters that `current` read and it doesn't, so a parameter goes away with the last prop using
 * it. One that nothing has read yet stays, as whoever wrote it may be about to use it.
 */
export function withoutDroppedParams(current: Scene, next: Scene): Scene {
  const stillRead = referencedParams(next)
  const dropped = [...referencedParams(current)].filter((name) => !stillRead.has(name))
  if (dropped.length === 0 || !next.params) return next
  const params = { ...next.params }
  for (const name of dropped) delete params[name]
  return { ...next, params }
}

/** The ranges for a parameter's sliders, from the first prop that reads it */
export function paramRanges(scene: Scene, name: string, surfaces: LayerSurfaces): Ranges {
  const use = propUses(reading(scene), surfaces).find(({ lens }) => {
    const prop = lens.get()
    return 'Param' in prop && prop.Param === name
  })
  return use?.ranges ?? both(UNREFERENCED)
}

/** The same range on both axes */
export const both = (range: Range): Ranges => [range, range]

export const UNIT: Range = { min: 0, max: 1, step: 0.01, rest: 0 }
/** A sound's volume: 1 is as recorded, and above it louder */
export const VOLUME: Range = { min: 0, max: 2, step: 0.01, rest: 1 }
export const ROTATION: Range = { min: -180, max: 180, step: 1, rest: 0 }
export const HUE: Range = { min: 0, max: 360, step: 1, rest: 0 }
const OPACITY: Range = { ...UNIT, rest: 1 }
/** A multiplier that can go either way, and leaves things alone at 1 */
const FACTOR: Range = { min: 0, max: 2, step: 0.01, rest: 1 }
const UNREFERENCED: Range = { min: -100, max: 100, step: 0.1, rest: 0 }
const FILL_MOVE: Range = { min: -1, max: 1, step: 0.01, rest: 0 }
const SCALE: Range = { min: 0, max: 16, step: 0.01, rest: 1 }
/** Steps a second, or for a traversal that goes by distance, scene units a second */
export const RATE: Range = { min: 0.1, max: 60, step: 0.1, rest: 10 }
/** How much longer a traversal's edges can be made at random, as a fraction of each */
export const WANDER: Range = { min: 0, max: 5, step: 0.01, rest: 0 }
/** Degrees a fan out's arc spans */
export const FAN_ANGLE: Range = { min: 0, max: 360, step: 1, rest: 80 }
/** How many nodes a fan out goes to besides its target */
export const FAN_COUNT: Range = { min: 0, max: 16, step: 1, rest: 3 }
/** How many offshoots a growth sends out from its path */
export const GROWTH_DENSITY: Range = { min: 0, max: 32, step: 1, rest: 4 }
/** The chance a growth's offshoot sends out one of its own */
export const BRANCHING: Range = { ...UNIT, rest: 0.3 }
/** Steps of a visit response's timing */
export const STEPS: Range = { min: 0, max: 10, step: 0.1, rest: 0 }
/** A fraction of a blade is dropped, so a spread goes one past the last, as it does for `frameRanges` */
export const BLADES: Range = { min: 1, max: 32, step: 1, spreadEnd: 33, rest: 4 }
/** How much of each of a pinwheel's shares is blade. Past these ends, its blades or its gaps are too thin to make out. */
export const BLADE_FRACTION: Range = { min: 0.1, max: 0.9, step: 0.01, rest: 0.5 }

export type AdjustKind = KeysOf<ColorAdjust>

/** What to call each kind of adjust, what to call its amount, and its range, which rests where it changes nothing */
export const ADJUSTS: Record<AdjustKind, { label: string; amount: string; paramName: string; range: Range }> = {
  HueShift: { label: 'Hue shift', amount: 'Degrees', paramName: 'hue_shift', range: HUE },
  Opacity: { label: 'Opacity', amount: 'Opacity', paramName: 'opacity', range: OPACITY },
  Brightness: { label: 'Brightness', amount: 'Brightness', paramName: 'brightness', range: FACTOR },
  Contrast: { label: 'Contrast', amount: 'Contrast', paramName: 'contrast', range: FACTOR },
}

export const adjustKind = (adjust: ColorAdjust) => Object.keys(adjust)[0] as AdjustKind

/** The amount of an adjust of any kind */
export function adjustAmount(lens: Lens<ColorAdjust>): Lens<Prop<number>> {
  return {
    get: () => Object.values(lens.get())[0],
    set: (value) => lens.set({ [adjustKind(lens.get())]: value } as ColorAdjust),
  }
}

/** An adjust that changes nothing, until it's changed */
export function newAdjust(kind: AdjustKind) {
  return { [kind]: { Fixed: ADJUSTS[kind].range.rest } } as ColorAdjust
}

/**
 * Any of an animated image's frames, or a good many until it's known how many there are. A fraction of a frame is
 * dropped, so a spread goes one past the last, which gives the last the same odds as any other.
 */
export function frameRanges(frames = 100): Ranges {
  return both({ min: 0, max: Math.max(frames - 1, 1), step: 1, spreadEnd: Math.max(frames, 2), rest: 0 })
}

export function translateRanges(surface: Surface, space: TransformSpace): Ranges {
  if (space === 'fill') return both(FILL_MOVE)
  if (!surface) return [UNIT, { min: -0.5, max: 0.5, step: 0.01, rest: 0 }]
  return [{ ...span(0, surface[0]), rest: 0 }, { ...span(0, surface[1]), rest: 0 }]
}

export function scaleRanges(): Ranges {
  return both(SCALE)
}

/** How wide and tall a new shape is: half of what it's on, and a sequence runs from 0 to 1 */
export function newShapeSize(surface: Surface) {
  const [width, height] = surface ?? [1, 0]
  return (height > 0 ? Math.min(width, height) : width) / 2
}

/** A shape's width and height, which can go up to the size of what it's on */
export function sizeRanges(surface: Surface): Ranges {
  const rest = newShapeSize(surface)
  if (!surface) return both({ ...UNIT, rest })
  return [{ ...span(0, surface[0]), rest }, { ...span(0, surface[1]), rest }]
}

/** An ellipse's radius along an axis of `sizeRanges`, which comes to the same size of shape */
export function radius(size: Range): Range {
  return { ...size, rest: size.rest! / 2 }
}

/** A size of a shape that's the same every way around, which can use the wider of `sizeRanges` */
export function roundRange(surface: Surface): Range {
  const [across, down] = sizeRanges(surface)
  return radius(across.max > down.max ? across : down)
}

/** How far a growth's offshoots can end from its path, which can be as far as `roundRange` goes */
export function reachRange(surface: Surface): Range {
  const range = roundRange(surface)
  return { ...range, rest: Math.round(newShapeSize(surface) / 4 / range.step) * range.step }
}

/** A polygon's points, in the shape's own space around its center */
export function pointRanges(surface: Surface): Ranges {
  if (!surface) return both({ min: -1, max: 1, step: 0.01 })
  return [span(-surface[0], surface[0]), span(-surface[1], surface[1])]
}

/** A range with a round step of about a 200th of its width */
function span(min: number, max: number): Range {
  const rough = (max - min) / 200
  const magnitude = 10 ** Math.floor(Math.log10(rough))
  const step = [1, 2, 5, 10].map((n) => n * magnitude).find((candidate) => candidate >= rough) ?? rough
  return { min, max, step }
}
