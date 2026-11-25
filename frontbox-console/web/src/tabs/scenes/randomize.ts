import { batch } from 'solid-js'
import type { Color } from '../../types/generated/Color'
import type { Curve } from '../../types/generated/Curve'
import type { Prop } from '../../types/generated/Prop'
import type { SweepEnds } from '../../types/generated/SweepEnds'
import type { ConfigNode } from './configNodes'
import { fullSpread, newModulator, newPick, newSweep, snap, sweepPaths, withLayerNamed, withNewModulator, withTiming, type Lens, type Range, type Ranges, type Scene, type SceneValue } from './sceneModel'

/** The ways a randomized prop can take its value, with how likely each is. A sweep is only for a position. */
const SOURCES: { kind: 'Fixed' | 'Keyframes' | 'Random' | 'Modulated' | 'Sweep'; weight: number }[] = [
  { kind: 'Fixed', weight: 50 },
  { kind: 'Keyframes', weight: 20 },
  { kind: 'Random', weight: 10 },
  { kind: 'Modulated', weight: 15 },
  { kind: 'Sweep', weight: 5 },
]

/** What a random sweep does past its ends */
const SWEEP_ENDS: SweepEnds[] = ['Wrap', 'Hold', 'PingPong']

/** The curves a random keyframe eases in by */
const KEY_CURVES: Curve[] = ['Linear', 'EaseIn', 'EaseOut', 'EaseInOut']

/** Props that are left as they are: whether a layer shows at all */
const KEPT = new Set(['Visible'])

/**
 * Scrambles every prop under a node of the config tree, the node itself included: each gets a value picked at random
 * within its ranges, and a random way of taking it. That's fixed, keyframed, picked at random, moved in a wave, or, for a
 * position, swept along another layer's region where there's one to follow, each with random settings of its own. A prop read from a variable stays as it is, as
 * does whether a layer is visible. It's a single edit, made once, not anything the scene does as it plays.
 */
export function randomizeNode(node: ConfigNode, scene: Lens<Scene>) {
  const props = propsUnder(node)
  batch(() => {
    for (const prop of props) {
      const current = prop.lens.get()
      if ('Variable' in current) continue
      const isPosition = Array.isArray(prop.fallback) && prop.fallback.length === 2
      const paths = sweepPaths(scene.get(), layerOf(prop.id))
      const { kind } = weighted(SOURCES.filter(({ kind }) => kind !== 'Sweep' || (isPosition && paths.length > 0)))
      // a fixed value has nothing to time, and a scene with none of the others is left without a timing
      if (kind !== 'Fixed') scene.set(withTiming(scene.get()))
      const duration = scene.get().timing?.duration ?? 1
      const path = () => {
        const [named, name] = withLayerNamed(scene.get(), paths[Math.floor(Math.random() * paths.length)])
        scene.set(named)
        return name
      }
      prop.lens.set(randomProp(kind, prop.fallback, prop.ranges, duration, () => randomModulator(scene), path))
    }
  })
}

/** Whether there's anything under a node to randomize */
export const canRandomize = (node: ConfigNode) => propsUnder(node).length > 0

type PropNode = Extract<ConfigNode, { kind: 'prop' }>

/** The props under a node, not counting those under a prop, which go with it */
function propsUnder(node: ConfigNode): PropNode[] {
  if (node.kind === 'prop') return KEPT.has(node.label) ? [] : [node as PropNode]
  return (node.children ?? []).flatMap(propsUnder)
}

/** Where a node is among the scene's drawn layers, from its id, or `null` for one that isn't in one */
const layerOf = (id: string) => {
  const match = /^layers\/(\d+)(\/|$)/.exec(id)
  return match ? Number(match[1]) : null
}

/** `modulator` gives the name of the modulator a wave moves by, and `path` the layer a sweep follows */
function randomProp(kind: (typeof SOURCES)[number]['kind'], like: SceneValue, ranges: Ranges, duration: number, modulator: () => string, path: () => string): Prop<SceneValue> {
  const value = () => randomValue(like, ranges)
  if (kind === 'Keyframes' && typeof like !== 'boolean') {
    // the start, the end, and one or two between, at times of their own
    const between = Array.from({ length: 1 + Math.floor(Math.random() * 2) }, () => snap(Math.random() * duration))
    const times = [...new Set([0, ...between, snap(duration)])].sort((a, b) => a - b)
    const keys = times.map((at) => ({ at, value: value(), curve: KEY_CURVES[Math.floor(Math.random() * KEY_CURVES.length)] }))
    return { Keyframes: { keys, cycle: 'Once' } }
  }
  if (kind === 'Random') {
    return { Random: { picks: [newPick(0)], spread: fullSpread(like, ranges), seed: Math.floor(Math.random() * 2 ** 32) } }
  }
  if (kind === 'Modulated' && typeof like !== 'boolean') {
    return { Modulated: { modulator: modulator(), spread: fullSpread(like, ranges), phase: { Fixed: Math.round(Math.random() * 100) / 100 } } }
  }
  if (kind === 'Sweep' && Array.isArray(like) && like.length === 2) return { Sweep: { ...newSweep(path(), duration), ends: SWEEP_ENDS[Math.floor(Math.random() * SWEEP_ENDS.length)] } }
  return { Fixed: value() }
}

/**
 * One of the scene's modulators for a randomized wave to move by: an existing one half the time, so that waves come to
 * share them, or else a new one at a rate of its own
 */
function randomModulator(scene: Lens<Scene>): string {
  const names = Object.keys(scene.get().modulators ?? {})
  if (names.length > 0 && Math.random() < 0.5) return names[Math.floor(Math.random() * names.length)]
  const rate = Math.round((1 + Math.random()) * 100) / 100
  const [next, name] = withNewModulator(scene.get(), { ...newModulator(), rate })
  scene.set(next)
  return name
}

/** A value like `like`, picked at random within its ranges */
function randomValue(like: SceneValue, [x, y]: Ranges): SceneValue {
  if (typeof like === 'boolean') return Math.random() < 0.5
  if (typeof like === 'number') return within(x)
  if (like.length === 2) return [within(x), within(y)]
  const channel = () => Math.floor(Math.random() * 256)
  return [channel(), channel(), channel(), 255] as Color
}

/** A number in a range, on its steps */
function within({ min, max, step }: Range) {
  const value = min + Math.random() * (max - min)
  return step > 0 ? Math.round(Math.min(max, Math.max(min, Math.round((value - min) / step) * step + min)) * 1000) / 1000 : value
}

/** One of `options`, as likely as its weight */
function weighted<T extends { weight: number }>(options: T[]): T {
  let left = Math.random() * options.reduce((sum, { weight }) => sum + weight, 0)
  return options.find(({ weight }) => (left -= weight) <= 0) ?? options.at(-1)!
}
