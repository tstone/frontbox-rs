import { Dialog } from '@ark-ui/solid/dialog'
import { createSignal, For, Show, type JSX } from 'solid-js'
import ScenePortal from './ScenePortal'
import type { Curve } from '../../types/generated/Curve'

/** Written for the scene file by hand, and not something this list can make */
export const CUSTOM_CURVE = 'Custom'

export const CURVES: Extract<Curve, string>[] = [
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

/** How many points along each curve are drawn */
const SAMPLES = 120

/** How far below 0 and above 1 a box shows, as the Back curves go past both */
const OVERSHOOT = 0.2

/** Overshoot amount for the Back curves */
const BACK_C1 = 1.70158
const BACK_C2 = BACK_C1 * 1.525
const BACK_C3 = BACK_C1 + 1

/**
 * A button that opens every curve drawn from 0 to 1, for picking `label`'s curve by how it looks. It shows `children`
 * in place of its own icon when given them, and the list has a button for `onRemove` when there's one.
 */
export default function CurvePicker(props: {
  label: string
  value: Curve | undefined
  onPick: (curve: Extract<Curve, string>) => void
  onRemove?: () => void
  class?: string
  children?: JSX.Element
}) {
  const [open, setOpen] = createSignal(false)
  const pick = (curve: Extract<Curve, string>) => {
    props.onPick(curve)
    setOpen(false)
  }

  return (
    <Dialog.Root lazyMount unmountOnExit open={open()} onOpenChange={(details) => setOpen(details.open)}>
      <Dialog.Trigger class={props.class ?? 'scene-eye'} aria-label={`Pick ${props.label} from the curves drawn out`} title="Show the curves">
        {props.children ?? (
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path d="M2 13.5C8 13.5 8 2.5 14 2.5" />
          </svg>
        )}
      </Dialog.Trigger>
      <ScenePortal>
        <Dialog.Backdrop class="scene-dialog-backdrop" />
        <Dialog.Positioner class="scene-dialog-positioner">
          <Dialog.Content class="scene-dialog">
            <header>
              <Dialog.Title>{props.label}</Dialog.Title>
              <Dialog.CloseTrigger aria-label="Close">×</Dialog.CloseTrigger>
            </header>
            <div class="scene-curve-list">
              <For each={CURVES}>{(curve) => <CurveBox curve={curve} chosen={props.value === curve} onClick={() => pick(curve)} />}</For>
            </div>
            <Show when={props.onRemove}>
              {(remove) => (
                <footer>
                  <button
                    type="button"
                    class="scene-button"
                    onClick={() => {
                      remove()()
                      setOpen(false)
                    }}
                  >
                    Remove
                  </button>
                </footer>
              )}
            </Show>
          </Dialog.Content>
        </Dialog.Positioner>
      </ScenePortal>
    </Dialog.Root>
  )
}

/** One curve, with lines where it starts from and goes to */
function CurveBox(props: { curve: Extract<Curve, string>; chosen: boolean; onClick: () => void }) {
  return (
    <button type="button" class="scene-curve" aria-pressed={props.chosen} onClick={props.onClick}>
      <CurveGraph height={(phase) => sample(props.curve, phase)} />
      <span>{props.curve}</span>
    </button>
  )
}

/** A curve's height at each phase from 0 to 1, drawn with lines at 0 and 1 */
export function CurveGraph(props: { height: (phase: number) => number }) {
  const y = (value: number) => 1 + OVERSHOOT - value
  const points = () =>
    Array.from({ length: SAMPLES + 1 }, (_, index) => {
      const phase = index / SAMPLES
      return `${phase},${y(props.height(phase))}`
    }).join(' ')

  return (
    <svg viewBox={`0 0 1 ${1 + 2 * OVERSHOOT}`} preserveAspectRatio="none" aria-hidden="true">
      <line class="scene-curve-guide" x1="0" x2="1" y1={y(0)} y2={y(0)} />
      <line class="scene-curve-guide" x1="0" x2="1" y1={y(1)} y2={y(1)} />
      <polyline class="scene-curve-line" points={points()} />
    </svg>
  )
}

/** Where a curve is at a phase from 0 to 1, as `Curve::sample` in frontbox-gfx has it */
export function sample(curve: Extract<Curve, string>, phase: number): number {
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
