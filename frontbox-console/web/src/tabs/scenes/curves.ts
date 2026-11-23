import type { Curve } from '../../types/generated/Curve'

/** A curve that's a shape of its own, with nothing inside it and no number */
export type PlainCurve = Extract<Curve, string>

/** A curve made from one or two others */
export type MadeCurve = 'Reverse' | 'Reflect' | 'Repeat' | 'Hold' | 'Add' | 'Multiply' | 'Compose'

export type CurveName = PlainCurve | MadeCurve | 'Steps' | 'Constant' | 'Noise'

/** Where a curve is inside another: which piece to go into at each level, from the outside in */
export type CurvePath = number[]

export const PLAIN_CURVES: PlainCurve[] = [
  'Linear',
  'EaseIn',
  'EaseOut',
  'EaseInOut',
  'SineInOut',
  'ExponentialIn',
  'ExponentialOut',
  'ExponentialInOut',
  'Sinusoid',
  'BackIn',
  'BackOut',
  'BackInOut',
  'BounceIn',
  'BounceOut',
  'BounceInOut',
]

/** The curves set by a number, with the number each starts at. Noise is made from `seed`. */
export function numberCurves(seed: number): Curve[] {
  return [{ Steps: 4 }, { Constant: 0.5 }, { Noise: [16, seed] }]
}

export const MADE_CURVES: MadeCurve[] = ['Reverse', 'Reflect', 'Repeat', 'Hold', 'Add', 'Multiply', 'Compose']

/** 1 through the first half of the wave and 0 through the second, for keeping a curve to the first half */
const FIRST_HALF: Curve = { Reverse: { Steps: 2 } }

/**
 * A ready-made composition, which goes in as its pieces so it can be changed like any other. Some are made around
 * `inside`, the curve they replace, as a composition is.
 */
export type CurveTemplate = { name: string; description: string; curve: (seed: number, inside: Curve) => Curve }

export const CURVE_TEMPLATES: CurveTemplate[] = [
  {
    name: 'Flame',
    description: 'A flame: mostly bright, with short uneven dips and a faster flutter on top',
    curve: (seed) => ({
      Add: [
        { Multiply: [{ Reflect: { Compose: [{ Noise: [16, seed] }, 'EaseIn'] } }, { Constant: 0.85 }] },
        { Multiply: [{ Noise: [48, (seed + 1) >>> 0] }, { Constant: 0.15 }] },
      ],
    }),
  },
  {
    name: 'Glitch',
    description: 'Mostly at rest, with sudden hard jumps all the way up',
    // Noise held at each of its points, so it jumps between them, then pushed down by ExponentialIn and snapped to 0 or
    // 1, so the points in the top tenth or so jump
    curve: (seed) => ({
      Compose: [{ Compose: [{ Compose: [{ Steps: 33 }, { Noise: [32, seed] }] }, 'ExponentialIn'] }, { Steps: 2 }],
    }),
  },
  {
    name: 'Dropout',
    description: 'Mostly on, with sudden hard drops all the way down, like a failing signal',
    // Glitch upside down. Reflect also plays it backwards, which only makes it a different random pattern.
    curve: (seed, inside) => ({ Reflect: CURVE_TEMPLATES.find(({ name }) => name === 'Glitch')!.curve(seed, inside) }),
  },
  {
    name: 'Strobe',
    description: 'Evenly spaced hard flashes, eight a wave',
    curve: () => ({ Compose: [{ Repeat: ['Linear', 8] }, { Steps: 2 }] }),
  },
  {
    name: 'Twinkle',
    description: 'Mostly dark, with soft peaks at random, like a star',
    // Noise to the fourth power, which keeps it low apart from its highest points
    curve: (seed) => ({ Compose: [{ Noise: [24, seed] }, { Compose: ['EaseIn', 'EaseIn'] }] }),
  },
  {
    name: 'Heartbeat',
    description: 'A strong beat, a weaker one, then a rest',
    // four sharp bumps, kept to the first half of the wave and fading down, so the second is smaller
    curve: () => ({
      Multiply: [{ Multiply: [{ Compose: [{ Repeat: ['Sinusoid', 4] }, 'ExponentialIn'] }, FIRST_HALF] }, { Reverse: 'Linear' }],
    }),
  },
  {
    name: 'Ring',
    description: 'A hit that rings out and fades, like a struck bell',
    curve: () => ({ Multiply: [{ Repeat: ['Sinusoid', 6] }, { Reverse: 'EaseIn' }] }),
  },
  {
    name: 'Startup',
    description: 'Flickers on and off unevenly through the first half, then stays on, like a fluorescent tube',
    // 1 minus the times it's off: Noise held at each of its points and snapped to on or off, about half each, kept to
    // the first half
    curve: (seed) => ({
      Add: [
        { Constant: 1 },
        {
          Multiply: [
            { Multiply: [{ Compose: [{ Compose: [{ Steps: 25 }, { Noise: [24, seed] }] }, { Steps: 2 }] }, FIRST_HALF] },
            { Constant: -1 },
          ],
        },
      ],
    }),
  },
  {
    name: 'Skipping',
    description: 'Plays the curve with its time jittering back and forth a little, like a skipping disc',
    // Linear time, plus a random offset from -0.05 to 0.05 held at each of 16 points
    curve: (seed, inside) => ({
      Compose: [
        {
          Add: [
            'Linear',
            { Add: [{ Multiply: [{ Compose: [{ Steps: 17 }, { Noise: [16, seed] }] }, { Constant: 0.1 }] }, { Constant: -0.05 }] },
          ],
        },
        inside,
      ],
    }),
  },
]

/** A seed for Noise, so each one added wobbles its own way */
export function randomSeed(): number {
  return Math.floor(Math.random() * 0x7fffffff)
}

export const CURVE_DESCRIPTIONS: Partial<Record<CurveName, string>> = {
  Steps: 'Jumps between this many levels from 0 to 1 instead of moving smoothly',
  Constant: 'Stays at one value the whole way',
  Noise: 'Wobbles smoothly at random through this many points, and ends where it starts so it repeats without a jump',
  Reverse: 'Plays the curve backwards, so one that rises from 0 to 1 falls from 1 to 0',
  Reflect: 'Plays the curve backwards and turns it upside down, so it still goes from 0 to 1, and an ease-in becomes an ease-out',
  Repeat: 'Plays the curve this many times over, each time from its start to its end',
  Hold: 'Splits the curve into this many slots and freezes it through about half of them at random, then catches up, like video that hangs',
  Add: 'Adds the two curves together, such as a Linear plus a smaller Sinusoid for a rise with a bump on it',
  Multiply: 'Multiplies the two curves together, such as a Sinusoid times an EaseIn for a pulse that grows',
  Compose: 'Feeds the first curve into the second, such as an EaseIn into Steps to make it jump between levels',
}

/** Overshoot amount for the Back curves */
const BACK_C1 = 1.70158
const BACK_C2 = BACK_C1 * 1.525
const BACK_C3 = BACK_C1 + 1

/** Where a curve is at a phase from 0 to 1, as `Curve::sample` in frontbox-gfx has it */
export function curveHeight(curve: Curve, phase: number): number {
  if (typeof curve === 'string') return plainHeight(curve, phase)
  if ('Steps' in curve) {
    const levels = Math.max(1, curve.Steps - 1)
    return Math.round(phase * levels) / levels
  }
  if ('Constant' in curve) return curve.Constant
  if ('Noise' in curve) return noiseHeight(curve.Noise[0], curve.Noise[1], phase)
  if ('Reverse' in curve) return curveHeight(curve.Reverse, 1 - phase)
  if ('Reflect' in curve) return 1 - curveHeight(curve.Reflect, 1 - phase)
  if ('Repeat' in curve) return curveHeight(curve.Repeat[0], repeatPhase(curve.Repeat[1], phase))
  if ('Hold' in curve) return curveHeight(curve.Hold[0], holdPhase(curve.Hold[1], curve.Hold[2], phase))
  if ('Add' in curve) return curveHeight(curve.Add[0], phase) + curveHeight(curve.Add[1], phase)
  if ('Multiply' in curve) return curveHeight(curve.Multiply[0], phase) * curveHeight(curve.Multiply[1], phase)
  return curveHeight(curve.Compose[1], curveHeight(curve.Compose[0], phase))
}

export function curveName(curve: Curve): CurveName {
  return typeof curve === 'string' ? curve : (Object.keys(curve)[0] as CurveName)
}

/** The whole curve written out as the scene file has it, such as `Compose(BackOut, Steps(4))` */
export function curveText(curve: Curve): string {
  if (typeof curve === 'string') return curve
  if ('Steps' in curve) return `Steps(${curve.Steps})`
  if ('Constant' in curve) return `Constant(${curve.Constant})`
  if ('Noise' in curve) return `Noise(${curve.Noise[0]}, ${curve.Noise[1]})`
  if ('Repeat' in curve) return `Repeat(${curveText(curve.Repeat[0])}, ${curve.Repeat[1]})`
  if ('Hold' in curve) return `Hold(${curveText(curve.Hold[0])}, ${curve.Hold[1]}, ${curve.Hold[2]})`
  return `${curveName(curve)}(${curvePieces(curve).map(curveText).join(', ')})`
}

/** The curves this one is made from, in order */
export function curvePieces(curve: Curve): Curve[] {
  if (typeof curve === 'string') return []
  if ('Reverse' in curve) return [curve.Reverse]
  if ('Reflect' in curve) return [curve.Reflect]
  if ('Repeat' in curve) return [curve.Repeat[0]]
  if ('Hold' in curve) return [curve.Hold[0]]
  if ('Add' in curve) return curve.Add
  if ('Multiply' in curve) return curve.Multiply
  if ('Compose' in curve) return curve.Compose
  return []
}

/** `curve` made from `pieces` instead of its own */
function withPieces(curve: Curve, pieces: Curve[]): Curve {
  if (typeof curve === 'string') return curve
  if ('Reverse' in curve) return { Reverse: pieces[0] }
  if ('Reflect' in curve) return { Reflect: pieces[0] }
  if ('Repeat' in curve) return { Repeat: [pieces[0], curve.Repeat[1]] }
  if ('Hold' in curve) return { Hold: [pieces[0], curve.Hold[1], curve.Hold[2]] }
  if ('Add' in curve) return { Add: [pieces[0], pieces[1]] }
  if ('Multiply' in curve) return { Multiply: [pieces[0], pieces[1]] }
  if ('Compose' in curve) return { Compose: [pieces[0], pieces[1]] }
  return curve
}

export function curveAt(curve: Curve, path: CurvePath): Curve | undefined {
  return path.reduce<Curve | undefined>((at, index) => at && curvePieces(at)[index], curve)
}

export function withCurveAt(curve: Curve, path: CurvePath, replacement: Curve): Curve {
  if (path.length === 0) return replacement
  const [index, ...rest] = path
  return withPieces(
    curve,
    curvePieces(curve).map((piece, at) => (at === index ? withCurveAt(piece, rest, replacement) : piece)),
  )
}

/** The longest start of `path` that's still in `curve`, for a path that was in it before a change */
export function pathIn(curve: Curve, path: CurvePath): CurvePath {
  let at = curve
  let length = 0
  for (const index of path) {
    const piece = curvePieces(at)[index]
    if (piece === undefined) break
    at = piece
    length++
  }
  return path.slice(0, length)
}

/**
 * A `name` curve made from `inside`. One made from two gets a second to start with that changes how `inside` looks,
 * so the change shows straight away.
 */
export function madeCurve(name: MadeCurve, inside: Curve, seed: number): Curve {
  switch (name) {
    case 'Reverse':
      return { Reverse: inside }
    case 'Reflect':
      return { Reflect: inside }
    case 'Repeat':
      return { Repeat: [inside, 2] }
    case 'Hold':
      return { Hold: [inside, 8, seed] }
    // the second one scaled down, as adding is usually a smaller change on top of the first
    case 'Add':
      return { Add: [inside, { Multiply: ['Sinusoid', { Constant: 0.25 }] }] }
    case 'Multiply':
      return { Multiply: [inside, 'Sinusoid'] }
    case 'Compose':
      return { Compose: [inside, { Steps: 4 }] }
  }
}

/** Where in its current repeat `phase` is, 0 to 1, as `sample_repeat` in frontbox-gfx has it */
function repeatPhase(times: number, phase: number) {
  const through = phase * Math.max(1, times)
  return through <= 0 ? through : through - (Math.ceil(through) - 1)
}

/** The phase to sample a Hold at, as `sample_hold` in frontbox-gfx has it */
function holdPhase(slots: number, seed: number, phase: number) {
  const count = Math.max(1, slots)
  const slot = Math.floor(phase * count)
  const frozen = noisePoint(((slot % count) + count) % count, seed) < 0.5
  return frozen ? slot / count : phase
}

/** Value noise, as `sample_noise` in frontbox-gfx has it */
function noiseHeight(points: number, seed: number, phase: number) {
  const count = Math.max(1, points)
  const through = phase * count
  const index = Math.floor(through)
  const t = through - index
  const wrap = (at: number) => ((at % count) + count) % count
  const from = noisePoint(wrap(index), seed)
  const to = noisePoint(wrap(index + 1), seed)
  return from + (to - from) * t * t * (3 - 2 * t)
}

/** A random height from 0 to 1 for point `index`, by the same integer hash as `noise_height` in frontbox-gfx */
function noisePoint(index: number, seed: number) {
  let x = (Math.imul(seed, 0x9e3779b9) + index) >>> 0
  x ^= x >>> 16
  x = Math.imul(x, 0x7feb352d) >>> 0
  x ^= x >>> 15
  x = Math.imul(x, 0x846ca68b) >>> 0
  x ^= x >>> 16
  return (x >>> 8) / 0xffffff
}

function plainHeight(curve: PlainCurve, phase: number): number {
  switch (curve) {
    case 'Linear':
      return phase
    case 'EaseIn':
      return phase ** 2
    case 'EaseOut':
      return 1 - (1 - phase) ** 2
    case 'EaseInOut':
      return phase < 0.5 ? 2 * phase ** 2 : 1 - (-2 * phase + 2) ** 2 / 2
    case 'SineInOut':
      return (1 - Math.cos(phase * Math.PI)) / 2
    case 'ExponentialIn':
      return 2 ** (10 * phase - 10)
    case 'ExponentialOut':
      return 1 - 2 ** (-10 * phase)
    case 'ExponentialInOut':
      return phase < 0.5 ? 2 ** (20 * phase - 10) / 2 : (2 - 2 ** (-20 * phase + 10)) / 2
    case 'Sinusoid':
      return 1 - (Math.cos(phase * 2 * Math.PI) + 1) / 2
    case 'BackIn':
      return BACK_C3 * phase ** 3 - BACK_C1 * phase ** 2
    case 'BackOut':
      return 1 + BACK_C3 * (phase - 1) ** 3 + BACK_C1 * (phase - 1) ** 2
    case 'BackInOut':
      return phase < 0.5
        ? ((2 * phase) ** 2 * ((BACK_C2 + 1) * 2 * phase - BACK_C2)) / 2
        : ((2 * phase - 2) ** 2 * ((BACK_C2 + 1) * (2 * phase - 2) + BACK_C2) + 2) / 2
    case 'BounceIn':
      return 1 - bounceOut(1 - phase)
    case 'BounceOut':
      return bounceOut(phase)
    case 'BounceInOut':
      return phase < 0.5 ? (1 - bounceOut(1 - 2 * phase)) / 2 : (1 + bounceOut(2 * phase - 1)) / 2
  }
}

function bounceOut(phase: number) {
  const n1 = 7.5625
  const d1 = 2.75
  if (phase < 1 / d1) return n1 * phase * phase
  if (phase < 2 / d1) return n1 * (phase - 1.5 / d1) ** 2 + 0.75
  if (phase < 2.5 / d1) return n1 * (phase - 2.25 / d1) ** 2 + 0.9375
  return n1 * (phase - 2.625 / d1) ** 2 + 0.984375
}
