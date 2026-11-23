import { Dialog } from '@ark-ui/solid/dialog'
import { createSignal, For, Show, type JSX } from 'solid-js'
import ScenePortal from './ScenePortal'
import NumberBox from './NumberBox'
import { Dices, X } from '../../components/icons'
import type { Curve } from '../../types/generated/Curve'
import {
  CURVE_DESCRIPTIONS,
  CURVE_TEMPLATES,
  curveAt,
  curveHeight,
  curveName,
  curvePieces,
  MADE_CURVES,
  madeCurve,
  numberCurves,
  pathIn,
  PLAIN_CURVES,
  randomSeed,
  withCurveAt,
  type CurvePath,
  type CurveTemplate,
  type MadeCurve,
} from './curves'

/** How many points along each curve are drawn */
const SAMPLES = 120

/** How far below 0 and above 1 a box shows, as the Back curves go past both */
const OVERSHOOT = 0.2

/**
 * A button that opens every curve drawn from 0 to 1, for picking `label`'s curve by how it looks. It shows `children`
 * in place of its own icon when given them.
 *
 * A curve made from others shows as a tree of its pieces above the list. Clicking a piece selects it, and the list
 * then sets that piece. A curve in the list that's made from others is made from the selected piece, so it's drawn
 * that way too.
 */
export default function CurvePicker(props: {
  label: string
  value: Curve | undefined
  onChange: (curve: Curve) => void
  class?: string
  children?: JSX.Element
}) {
  const [open, setOpen] = createSignal(false)
  const [chosenPath, setChosenPath] = createSignal<CurvePath>([])
  // the seed the Noise and templates in the list are drawn with, and made with when picked. A new one after each is
  // picked, so no two wobble the same way.
  const [seed, setSeed] = createSignal(randomSeed())
  // a path left over from before a piece was taken out stops where the curve does now
  const path = () => (props.value === undefined ? [] : pathIn(props.value, chosenPath()))
  const selected = () => (props.value === undefined ? undefined : curveAt(props.value, path()))

  const set = (curve: Curve) => props.onChange(props.value === undefined ? curve : withCurveAt(props.value, path(), curve))
  const pickPlain = (curve: Curve) => {
    // picking the same kind keeps its number
    if (selected() === undefined || curveName(selected()!) !== curveName(curve)) {
      set(curve)
      if (curveName(curve) === 'Noise') setSeed(randomSeed())
    }
    if (path().length === 0 && typeof curve === 'string') setOpen(false)
  }
  const pickTemplate = (template: CurveTemplate) => {
    set(template.curve(seed(), selected() ?? 'Linear'))
    setSeed(randomSeed())
  }
  const pickMade = (name: MadeCurve) => {
    set(madeCurve(name, selected() ?? 'Linear', seed()))
    if (name === 'Hold') setSeed(randomSeed())
    // the first piece is the curve it was made from, so what's left to choose is the second. What's added is scaled
    // down by a Constant, so choosing from the list changes its shape and keeps its strength.
    if (name === 'Add') setChosenPath([...path(), 1, 0])
    else if (name === 'Multiply' || name === 'Compose') setChosenPath([...path(), 1])
    else setChosenPath(path())
  }

  return (
    <Dialog.Root
      lazyMount
      unmountOnExit
      open={open()}
      onOpenChange={(details) => {
        if (details.open) setChosenPath([])
        setOpen(details.open)
      }}
    >
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
          <Dialog.Content class="scene-dialog scene-curve-dialog" classList={{ 'scene-curve-composing': typeof props.value === 'object' }}>
            <header>
              <Dialog.Title>{props.label}</Dialog.Title>
              <Dialog.CloseTrigger aria-label="Close">×</Dialog.CloseTrigger>
            </header>
            {/* a curve made from others is built in a column beside the list, as its pieces grow downwards */}
            <div class="scene-curve-body">
            <Show when={typeof props.value === 'object'}>
              <div class="scene-curve-editing">
                <figure class="scene-curve-whole">
                  <CurveGraph height={(phase) => curveHeight(props.value!, phase)} />
                  <figcaption>Final</figcaption>
                </figure>
                <div class="scene-curve-tree">
                  <CurvePiece root={props.value!} path={[]} selected={path()} onSelect={setChosenPath} onChange={props.onChange} />
                </div>
              </div>
            </Show>
            <div class="scene-curve-choices">
              <div class="scene-curve-list">
                <For each={[...PLAIN_CURVES, ...numberCurves(seed())]}>
                  {(curve) => <CurveBox curve={curve} chosen={selected() !== undefined && curveName(selected()!) === curveName(curve)} onClick={() => pickPlain(curve)} />}
                </For>
              </div>
              <h3>Composition</h3>
              <div class="scene-curve-list">
                <For each={MADE_CURVES}>
                  {(name) => (
                    <button type="button" class="scene-curve scene-curve-named" title={CURVE_DESCRIPTIONS[name]} onClick={() => pickMade(name)}>
                      <span>{name}</span>
                    </button>
                  )}
                </For>
              </div>
              <h3>Templates</h3>
              <div class="scene-curve-list">
                <For each={CURVE_TEMPLATES}>
                  {(template) => (
                    <CurveBox curve={template.curve(seed(), selected() ?? 'Linear')} name={template.name} title={template.description} chosen={false} onClick={() => pickTemplate(template)} />
                  )}
                </For>
              </div>
            </div>
            </div>
          </Dialog.Content>
        </Dialog.Positioner>
      </ScenePortal>
    </Dialog.Root>
  )
}

/**
 * The piece of `root` at `path`, with the pieces it's made from below it. One set by a number has a box for it, and
 * one made from others has an X that puts its first piece in its place.
 */
function CurvePiece(props: { root: Curve; path: CurvePath; selected: CurvePath; onSelect: (path: CurvePath) => void; onChange: (curve: Curve) => void }) {
  const curve = () => curveAt(props.root, props.path)!
  const name = () => curveName(curve())
  const pieces = () => curvePieces(curve())
  const isSelected = () => props.selected.length === props.path.length && props.selected.every((index, at) => index === props.path[at])
  const set = (next: Curve) => props.onChange(withCurveAt(props.root, props.path, next))
  const number = () => {
    const value = curve()
    if (typeof value === 'string') return undefined
    if ('Steps' in value) return { label: 'Levels', value: value.Steps, min: 2, step: 1, set: (n: number) => set({ Steps: Math.max(2, Math.round(n)) }) }
    if ('Constant' in value) return { label: 'Value', value: value.Constant, step: 0.05, set: (n: number) => set({ Constant: n }) }
    if ('Repeat' in value) return { label: 'Times', value: value.Repeat[1], min: 1, step: 1, set: (n: number) => set({ Repeat: [value.Repeat[0], Math.max(1, Math.round(n))] }) }
    if ('Hold' in value) {
      const [inside, slots] = value.Hold
      return {
        label: 'Slots',
        value: slots,
        min: 2,
        step: 1,
        set: (n: number) => set({ Hold: [inside, Math.max(2, Math.round(n)), value.Hold[2]] }),
        reroll: () => set({ Hold: [inside, slots, randomSeed()] }),
      }
    }
    if ('Noise' in value) {
      const [points, seed] = value.Noise
      return {
        label: 'Points',
        value: points,
        min: 2,
        step: 1,
        set: (n: number) => set({ Noise: [Math.max(2, Math.round(n)), seed] }),
        reroll: () => set({ Noise: [points, randomSeed()] }),
      }
    }
    return undefined
  }

  return (
    <div class="scene-curve-node">
      <div class="scene-curve-node-row">
        <div class="scene-curve-removable">
          <button
            type="button"
            class="scene-curve scene-curve-piece"
            classList={{ 'scene-curve-named': pieces().length > 0 }}
            aria-pressed={isSelected()}
            title={CURVE_DESCRIPTIONS[name()]}
            onClick={() => props.onSelect(props.path)}
          >
            {/* a composition is drawn by its pieces below it, and all together as the final curve */}
            <Show when={pieces().length === 0}>
              <CurveGraph height={(phase) => curveHeight(curve(), phase)} />
            </Show>
            <span>{name()}</span>
          </button>
          <Show when={pieces().length > 0}>
            <button type="button" class="scene-curve-remove" aria-label={`Take out ${name()}, keeping its first curve`} title={`Take out ${name()}, keeping its first curve`} onClick={() => set(pieces()[0])}>
              <X aria-hidden="true" />
            </button>
          </Show>
        </div>
        <Show when={number()}>
          {(number) => (
            <>
              <label class="scene-curve-number">
                {number().label}
                <NumberBox class="scene-number-box" value={number().value} min={number().min} step={number().step} onNumber={number().set} />
              </label>
              {/* the seed is kept out of the way: a new random one is all that's usually wanted */}
              <Show when={number().reroll}>
                {(reroll) => (
                  <button type="button" class="scene-curve-reroll" aria-label="Pick again at random" title="Pick again at random" onClick={reroll()}>
                    <Dices aria-hidden="true" />
                  </button>
                )}
              </Show>
            </>
          )}
        </Show>
      </div>
      <Show when={pieces().length > 0}>
        <div class="scene-curve-pieces">
          <For each={pieces()}>{(_, index) => <CurvePiece root={props.root} path={[...props.path, index()]} selected={props.selected} onSelect={props.onSelect} onChange={props.onChange} />}</For>
        </div>
      </Show>
    </div>
  )
}

/** One curve to pick, with lines where it starts from and goes to */
function CurveBox(props: { curve: Curve; name?: string; title?: string; chosen: boolean; onClick: () => void }) {
  const name = () => props.name ?? curveName(props.curve)
  return (
    <button type="button" class="scene-curve" aria-pressed={props.chosen} title={props.title ?? CURVE_DESCRIPTIONS[curveName(props.curve)]} onClick={props.onClick}>
      <CurveGraph height={(phase) => curveHeight(props.curve, phase)} />
      <span>{name()}</span>
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
