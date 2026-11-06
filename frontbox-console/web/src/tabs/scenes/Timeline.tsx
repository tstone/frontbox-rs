import { Popover } from '@ark-ui/solid/popover'
import { createMemo, createSignal, For, Index, Show, useContext } from 'solid-js'
import { Portal } from 'solid-js/web'
import type { Curve } from '../../types/generated/Curve'
import type { Cycle } from '../../types/generated/Cycle'
import type { Key } from '../../types/generated/Key'
import type { Pick as RandomPick } from '../../types/generated/Pick'
import type { Prop } from '../../types/generated/Prop'
import type { Timing } from '../../types/generated/Timing'
import type { Playback } from './playback'
import { Dropdown, FieldRow, NumberField, SceneContext, SurfaceContext, ValueField } from './SceneFields'
import { field, item, removed, repeats, sceneTracks, trackTimes, UNIT, variant, withKey, withKeyCopy, withPick, type Lens, type SceneTrack, type SceneValue } from './sceneModel'

/** Times are kept to this, in seconds, as a key at 0.4183 is only ever an accident of the mouse */
const TIME_STEP = 0.01

/** How close in pixels a click can be to a key before it's taken to mean that key */
const KEY_REACH = 12

/** How far in pixels a press on a dot has to move before it's a drag and not a click */
const DRAG_START = 3

/** How far along the view a copy of a key goes from it, given the room */
const COPY_DISTANCE = 0.05

const CURVES: Extract<Curve, string>[] = [
  'Linear',
  'EaseIn',
  'EaseOut',
  'EaseInOut',
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
  'Random',
]

/** Written for the scene file by hand, and not something this list can make */
const CUSTOM_CURVE = 'Custom'

const TICK_SPACINGS = [0.05, 0.1, 0.25, 0.5, 1, 2, 5, 10, 30, 60, 120, 300]
const MOST_TICKS = 10

const snap = (seconds: number) => Math.round(seconds / TIME_STEP) * TIME_STEP

const clamp = (value: number, min: number, max: number) => Math.min(Math.max(value, min), max)

/** Where a pointer is along an element, from 0 to 1 */
function along(e: PointerEvent, element: HTMLElement) {
  const { left, width } = element.getBoundingClientRect()
  return clamp((e.clientX - left) / width, 0, 1)
}

/** A scene's timing, and a track for each keyframed prop, along the bottom of the editor */
export default function Timeline(props: { playback: Playback; rendering: boolean; error: string | null }) {
  const scene = useContext(SceneContext)!
  const surface = useContext(SurfaceContext)!
  const timing = () => scene.get().timing
  const tracks = createMemo(() => sceneTracks(scene, surface()))

  /** How much time the view shows. A key past the end is still there to be found and moved. */
  const span = createMemo(() =>
    Math.max(timing()?.duration ?? 1, ...tracks().map(({ prop }) => trackTimes(prop.get()).at(-1) ?? 0)),
  )

  const percent = (seconds: number) => `${(seconds / span()) * 100}%`

  const ticks = () => {
    const spacing = TICK_SPACINGS.find((candidate) => span() / candidate <= MOST_TICKS) ?? span()
    return Array.from({ length: Math.floor(span() / spacing) + 1 }, (_, index) => index * spacing)
  }

  const scrubTo = (e: PointerEvent & { currentTarget: HTMLElement }) => {
    props.playback.scrub(Math.min(snap(along(e, e.currentTarget) * span()), timing()!.duration))
  }

  return (
    <section class="scene-timeline" aria-label="Timeline">
      <header>
        <Show
          when={props.playback.playing()}
          fallback={
            <button type="button" aria-label="Play" disabled={!timing()} onClick={props.playback.play}>
              <svg viewBox="0 0 16 16" aria-hidden="true">
                <path d="M4 2.5v11l9-5.5z" />
              </svg>
            </button>
          }
        >
          <button type="button" aria-label="Pause" onClick={props.playback.pause}>
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <path d="M3.5 2.5h3v11h-3zM9.5 2.5h3v11h-3z" />
            </svg>
          </button>
        </Show>
        <button type="button" aria-label="Stop" disabled={!timing()} onClick={props.playback.stop}>
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path d="M3 3h10v10H3z" />
          </svg>
        </button>
        <Show when={props.rendering}>
          <span class="scene-spinner" role="status" aria-label="Rendering" />
        </Show>
        <Show when={timing()}>
          <span class="scene-timeline-time">
            {props.playback.position().toFixed(2)} / {timing()!.duration.toFixed(2)} s
          </span>
          <TimingFields lens={field(scene, 'timing') as Lens<Timing>} />
        </Show>
        <Show when={props.error}>
          <span class="scene-note scene-note-warn">{props.error}</span>
        </Show>
      </header>
      <Show
        when={timing()}
        fallback={<p class="scene-note">Set any value to Track to animate this scene.</p>}
      >
        <div class="scene-timeline-tracks">
          <div class="scene-timeline-lanes" style={{ '--played-fraction': props.playback.position() / span(), '--end-fraction': timing()!.duration / span() }}>
            <div class="scene-timeline-row scene-timeline-times">
              <span />
              <div
                class="scene-timeline-ruler"
                onPointerDown={(e) => {
                  e.currentTarget.setPointerCapture(e.pointerId)
                  scrubTo(e)
                }}
                onPointerMove={(e) => {
                  if (e.currentTarget.hasPointerCapture(e.pointerId)) scrubTo(e)
                }}
              >
                <For each={ticks()}>
                  {(tick) => (
                    <span class="scene-timeline-tick" style={{ left: percent(tick) }}>
                      {Number(tick.toFixed(2))}
                    </span>
                  )}
                </For>
              </div>
            </div>
            {/* by position, so an edit doesn't tear down the editor it was made in */}
            <Index each={tracks()} fallback={<p class="scene-note">Set any value to Track to give it a track here.</p>}>
              {(track) => <TrackRow track={track()} percent={percent} span={span()} />}
            </Index>
            <div class="scene-timeline-playhead" />
          </div>
        </div>
      </Show>
    </section>
  )
}

/** The length of a cycle, and how many times it plays */
function TimingFields(props: { lens: Lens<Timing> }) {
  const cycle = field(props.lens, 'cycle')
  const kind = () => {
    const value = cycle.get()
    return typeof value === 'string' ? value : 'Times'
  }

  return (
    <>
      <label>
        Length
        <input
          type="number"
          min={TIME_STEP}
          step="0.1"
          value={props.lens.get().duration}
          onInput={(e) => {
            const duration = e.currentTarget.valueAsNumber
            if (duration > 0) field(props.lens, 'duration').set(duration)
          }}
        />
      </label>
      <Dropdown<'Once' | 'Times' | 'Forever'>
        label="Repeat"
        options={[
          { value: 'Once', label: 'Once' },
          { value: 'Times', label: 'Times' },
          { value: 'Forever', label: 'Forever' },
        ]}
        value={kind()}
        onChange={(next) => {
          if (next !== kind()) cycle.set((next === 'Times' ? { Times: 2 } : next) as Cycle)
        }}
      />
      <Show when={kind() === 'Times'}>
        <input
          type="number"
          min="1"
          step="1"
          aria-label="Times to play"
          value={(cycle.get() as { Times: number }).Times}
          onInput={(e) => {
            const times = e.currentTarget.valueAsNumber
            if (Number.isInteger(times) && times > 0) cycle.set({ Times: times })
          }}
        />
      </Show>
    </>
  )
}

/** A prop's name and its keys or picks along the timeline, with a button that follows the mouse for adding one */
function TrackRow(props: { track: SceneTrack; percent: (seconds: number) => string; span: number }) {
  // read through `props`, as a row goes to whichever prop comes to be at its position
  const prop: Lens<Prop<SceneValue>> = { get: () => props.track.prop.get(), set: (next) => props.track.prop.set(next) }
  const times = () => trackTimes(prop.get())
  const [hovering, setHovering] = createSignal<number | null>(null)

  const hover = (e: PointerEvent & { currentTarget: HTMLElement }) => {
    const { width } = e.currentTarget.getBoundingClientRect()
    const at = along(e, e.currentTarget) * props.span
    const onKey = times().some((time) => (Math.abs(time - at) / props.span) * width < KEY_REACH)
    setHovering(onKey ? null : snap(at))
  }

  const add = (at: number) => {
    const current = prop.get()
    if ('Track' in current) prop.set({ Track: withKey(current.Track, at) })
    if ('Random' in current) prop.set({ Random: withPick(current.Random, at) })
  }

  return (
    <div class="scene-timeline-row">
      <span class="scene-timeline-label" title={props.track.label}>
        {props.track.label}
      </span>
      <div class="scene-timeline-lane" onPointerMove={hover} onPointerLeave={() => setHovering(null)}>
        <Show when={hovering() !== null}>
          <button
            type="button"
            class="scene-key-add"
            aria-label={`Add a key to ${props.track.label}`}
            style={{ left: props.percent(hovering()!) }}
            onClick={() => {
              add(hovering()!)
              setHovering(null)
            }}
          >
            +
          </button>
        </Show>
        <Index each={times()}>
          {(time, index) => (
            <Show
              when={'Track' in prop.get()}
              fallback={
                <PickDot
                  label={props.track.label}
                  picks={field(variant(prop, 'Random'), 'picks')}
                  index={index}
                  left={props.percent(time())}
                  times={times()}
                  span={props.span}
                />
              }
            >
              <KeyDot
                label={props.track.label}
                ranges={props.track}
                keys={field(variant(prop, 'Track'), 'keys')}
                index={index}
                left={props.percent(time())}
                times={times()}
                span={props.span}
              />
            </Show>
          )}
        </Index>
      </div>
    </div>
  )
}

/** Where a dot sits among the others on its track */
type DotPlace = { index: number; left: string; times: number[]; span: number }

/** A free time just after a dot, or just before it if it has no room after. There's none if it's hemmed in both ways. */
function roomNear({ index, times, span }: DotPlace) {
  const toward = (end: number) => {
    const room = end - times[index]
    return snap(times[index] + Math.sign(room) * Math.min(span * COPY_DISTANCE, Math.abs(room) / 2))
  }
  const candidates = [toward(times[index + 1] ?? span), toward(times[index - 1] ?? 0)]
  return candidates.find((candidate) => times.every((time) => Math.abs(time - candidate) >= TIME_STEP / 2))
}

/** A dot on a track. Clicking it opens what can be changed about it, and dragging it moves it between its neighbors. */
function Dot(props: DotPlace & { label: string; move: (at: number) => void; pick?: boolean; background?: string; children: any }) {
  const [open, setOpen] = createSignal(false)
  // kept from the press, as the view's length can change with where the dot is
  let press: { x: number; min: number; max: number; span: number } | null = null
  // until the next press, as the click that ends a drag comes after the pointer is let go
  let dragged = false

  return (
    <Popover.Root
      lazyMount
      unmountOnExit
      positioning={{ placement: 'top' }}
      open={open()}
      onOpenChange={(details) => {
        if (!dragged) setOpen(details.open)
      }}
    >
      <Popover.Trigger
        class="scene-key"
        classList={{ 'scene-key-value': !props.background, 'scene-key-pick': props.pick }}
        aria-label={props.label}
        style={{ left: props.left, background: props.background }}
        onPointerDown={(e) => {
          e.currentTarget.setPointerCapture(e.pointerId)
          dragged = false
          const before = props.times[props.index - 1]
          const after = props.times[props.index + 1]
          press = {
            x: e.clientX,
            min: before === undefined ? 0 : before + TIME_STEP,
            max: after === undefined ? props.span : after - TIME_STEP,
            span: props.span,
          }
        }}
        onPointerMove={(e) => {
          if (!press || (!dragged && Math.abs(e.clientX - press.x) < DRAG_START)) return
          dragged = true
          props.move(clamp(snap(along(e, e.currentTarget.parentElement!) * press.span), press.min, press.max))
        }}
        onPointerUp={() => (press = null)}
        onPointerCancel={() => (press = null)}
      />
      <Portal>
        <Popover.Positioner>
          <Popover.Content class="scene-key-editor">{props.children}</Popover.Content>
        </Popover.Positioner>
      </Portal>
    </Popover.Root>
  )
}

/** A moment on a random prop's track when a new value is picked */
function PickDot(props: DotPlace & { label: string; picks: Lens<RandomPick[]> }) {
  const scene = useContext(SceneContext)!
  const pick = () => item(props.picks, props.index)
  const last = () => props.index === props.picks.get().length - 1

  const smoothing = (label: string, name: 'smooth_in' | 'smooth_out') => (
    <FieldRow label={label}>
      <NumberField label={label} range={UNIT} value={pick().get()[name]} onChange={field(pick(), name).set} />
    </FieldRow>
  )

  return (
    <Dot {...props} label={`${props.label} pick at ${pick().get().at.toFixed(2)} seconds`} move={field(pick(), 'at').set} pick>
      {/* the first moment of a scene has nothing to smooth from, nor the last anything to smooth to */}
      <Show when={props.index > 0 || repeats(scene.get())}>{smoothing('Smooth in', 'smooth_in')}</Show>
      <Show when={!last() || repeats(scene.get())}>{smoothing('Smooth out', 'smooth_out')}</Show>
      {/* with no picks there'd be nothing to show, and leaving it is a matter of switching it away from Random */}
      <button
        type="button"
        class="scene-key-remove"
        disabled={props.picks.get().length === 1}
        onClick={() => props.picks.set(removed(props.picks.get(), props.index))}
      >
        Remove pick
      </button>
    </Dot>
  )
}

/** A key on a track */
function KeyDot(props: DotPlace & { label: string; ranges: Pick<SceneTrack, 'ranges'>; keys: Lens<Key<SceneValue>[]> }) {
  const key = () => item(props.keys, props.index)
  const at = () => key().get().at
  const room = () => roomNear(props)

  const curve = () => {
    const value = key().get().curve
    return typeof value === 'string' ? value : CUSTOM_CURVE
  }

  const background = () => {
    const value = key().get().value
    return Array.isArray(value) && value.length === 4 ? `rgb(${value.slice(0, 3).join(' ')})` : undefined
  }

  return (
    <Dot {...props} label={`${props.label} key at ${at().toFixed(2)} seconds`} move={field(key(), 'at').set} background={background()}>
      <FieldRow label="Value">
        <ValueField label="Value" lens={field(key(), 'value')} ranges={props.ranges.ranges} />
      </FieldRow>
      <Show when={props.index > 0}>
        <FieldRow label="Curve into this key">
          <Dropdown
            label="Curve into this key"
            options={[...CURVES, ...(curve() === CUSTOM_CURVE ? [CUSTOM_CURVE] : [])].map((value) => ({ value, label: value }))}
            value={curve()}
            onChange={(next) => {
              if (next !== CUSTOM_CURVE) field(key(), 'curve').set(next as Curve)
            }}
          />
        </FieldRow>
      </Show>
      <div class="scene-key-actions">
        <button
          type="button"
          class="scene-key-duplicate"
          disabled={room() === undefined}
          onClick={() => props.keys.set(withKeyCopy(props.keys.get(), props.index, room()!))}
        >
          Duplicate
        </button>
        {/* a track with no keys reads as zero, and leaving it is a matter of switching it away from Track */}
        <button
          type="button"
          class="scene-key-remove"
          disabled={props.keys.get().length === 1}
          onClick={() => props.keys.set(removed(props.keys.get(), props.index))}
        >
          Remove keyframe
        </button>
      </div>
    </Dot>
  )
}
