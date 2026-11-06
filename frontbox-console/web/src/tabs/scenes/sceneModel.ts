import type { Color } from '../../types/generated/Color'
import type { Prop } from '../../types/generated/Prop'
import type { Scene } from '../../types/generated/Scene'
import type { Value } from '../../types/generated/Value'
import type { Viewport } from '../../types/generated/Viewport'

export type Vec2 = [number, number]

/** What a `Prop` can hold */
export type SceneValue = number | Vec2 | Color

/** The span a slider covers. The number box next to it can go past this. */
export type Range = { min: number; max: number; step: number }

/** A range for each axis. A plain number uses the first. */
export type Ranges = [Range, Range]

/** Which space a warp moves things in: a shape's is the viewport, a paint's is its unit box */
export type WarpSpace = 'shape' | 'paint'

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

/** The value a prop shows with no game running: the parameter's default, or a track's first key */
export function startingValue<T extends SceneValue>(prop: Prop<T>, scene: Scene, fallback: T): T {
  if ('Fixed' in prop) return prop.Fixed
  if ('Track' in prop) return prop.Track.keys[0]?.value ?? fallback
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

/** Scene with a parameter added or changed */
export function withParam(scene: Scene, name: string, value: SceneValue): Scene {
  return { ...scene, params: { ...scene.params, [name]: wrapValue(value) } }
}

/** Every prop in a scene, with the ranges its sliders use */
function propUses(scene: Scene) {
  const uses: { prop: Prop<SceneValue>; ranges: Ranges }[] = []
  const warps = (warp: Scene['layers'][number]['paint']['warp'], space: WarpSpace) => {
    for (const w of warp ?? []) {
      if ('Translate' in w) uses.push({ prop: w.Translate, ranges: translateRanges(scene.viewport, space) })
      else if ('Scale' in w) uses.push({ prop: w.Scale, ranges: scaleRanges(scene.viewport, space) })
      else uses.push({ prop: w.Rotate, ranges: both(ROTATION) })
    }
  }

  for (const { paint, shape } of scene.layers) {
    warps(paint.warp, 'paint')
    if ('Solid' in paint.base) uses.push({ prop: paint.base.Solid, ranges: both(UNIT) })
    else for (const stop of paint.base.Gradient.stops) uses.push({ prop: stop.color, ranges: both(UNIT) })
    for (const adjust of paint.adjust ?? []) uses.push({ prop: adjust.HueShift, ranges: both(HUE) })

    if (shape) {
      warps(shape.warp, 'shape')
      if (shape.base !== 'Ellipse') {
        for (const point of shape.base.Polygon.points) uses.push({ prop: point, ranges: both(POLYGON_POINT) })
      }
    }
  }
  return uses
}

/** The names of the parameters a scene's props read */
function referencedParams(scene: Scene) {
  const names = new Set<string>()
  for (const { prop } of propUses(scene)) if ('Param' in prop) names.add(prop.Param)
  return names
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
export function paramRanges(scene: Scene, name: string): Ranges {
  const use = propUses(scene).find(({ prop }) => 'Param' in prop && prop.Param === name)
  return use?.ranges ?? both(UNREFERENCED)
}

/** The same range on both axes */
export const both = (range: Range): Ranges => [range, range]

export const UNIT: Range = { min: 0, max: 1, step: 0.01 }
export const ROTATION: Range = { min: -180, max: 180, step: 1 }
export const HUE: Range = { min: 0, max: 360, step: 1 }
/** A polygon's points are in the shape's own space, before its warps size it */
export const POLYGON_POINT: Range = { min: -1, max: 1, step: 0.01 }
export const VIEWPORT_SIDE: Range = { min: 1, max: 512, step: 1 }
const UNREFERENCED: Range = { min: -100, max: 100, step: 0.1 }
const PAINT_MOVE: Range = { min: -1, max: 1, step: 0.01 }
const PAINT_SCALE: Range = { min: 0, max: 4, step: 0.01 }

export function translateRanges(viewport: Viewport, space: WarpSpace): Ranges {
  if (space === 'paint') return both(PAINT_MOVE)
  if (viewport === 'Sequence') return [UNIT, { min: -0.5, max: 0.5, step: 0.01 }]
  return [span(0, viewport.Area[0]), span(0, viewport.Area[1])]
}

export function scaleRanges(viewport: Viewport, space: WarpSpace): Ranges {
  if (space === 'paint') return both(PAINT_SCALE)
  if (viewport === 'Sequence') return both({ min: 0, max: 2, step: 0.01 })
  return [span(0, viewport.Area[0] * 2), span(0, viewport.Area[1] * 2)]
}

/** A range with a round step of about a 200th of its width */
function span(min: number, max: number): Range {
  const rough = (max - min) / 200
  const magnitude = 10 ** Math.floor(Math.log10(rough))
  const step = [1, 2, 5, 10].map((n) => n * magnitude).find((candidate) => candidate >= rough) ?? rough
  return { min, max, step }
}
