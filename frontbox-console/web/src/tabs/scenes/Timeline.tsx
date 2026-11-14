import { Popover } from '@ark-ui/solid/popover'
import { batch, createMemo, createSignal, For, Index, onCleanup, Show, useContext } from 'solid-js'
import ScenePortal, { LayerColorContext, layerColorClass, type LayerColor } from './ScenePortal'
import type { Cycle } from '../../types/generated/Cycle'
import type { Key } from '../../types/generated/Key'
import type { Modulated } from '../../types/generated/Modulated'
import type { Pick as RandomPick } from '../../types/generated/Pick'
import type { Prop } from '../../types/generated/Prop'
import type { RenderedGraph } from '../../types/generated/RenderedGraph'
import type { Timing } from '../../types/generated/Timing'
import type { Traversal } from '../../types/generated/Traversal'
import type { Playback } from './playback'
import { sample } from './CurvePicker'
import { CurveField, Dropdown, EyeButton, FieldRow, LayerSurfacesContext, NumberBox, NumberField, SceneContext, ValueField } from './SceneFields'
import { contentSoundLength } from '../../state/contentSounds'
import type { SoundLayer } from '../../types/generated/SoundLayer'
import { field, isHidden, item, moveMoment, removed, repeats, sceneTracks, soundEnd, soundLabel, soundNaturalEnd, sceneTraversals, snap, startingValue, TIME_STEP, trackTimes, traversalLength, UNIT, variant, waveHeight, wavePosition, withKey, withKeyCopy, withPick, withHidden, withTimesInLength, withTimesScaled, type Lens, type Scene, type SceneTrack, type SceneTraversal, type SceneValue } from './sceneModel'

/** How close in pixels a click can be to a key before it's taken to mean that key */
const KEY_REACH = 12

/** How far in pixels a press on a dot has to move before it's a drag and not a click */
const DRAG_START = 3

/** How far along the view a copy of a key goes from it, given the room */
const COPY_DISTANCE = 0.05

/** How many milliseconds the pointer rests on the button for the time scale menu to open */
const SCALE_MENU_OPEN_DELAY = 400

/** How many milliseconds the time scale menu stays open once the pointer has left it */
const SCALE_MENU_CLOSE_DELAY = 1000

type TimeScale = { label: string; by: number }

const SHORTER: TimeScale[] = [
  { label: '÷ 4', by: 1 / 4 },
  { label: '÷ 2', by: 1 / 2 },
  { label: '÷ 1.5', by: 1 / 1.5 },
]

const LONGER: TimeScale[] = [
  { label: '× 1.5', by: 1.5 },
  { label: '× 2', by: 2 },
  { label: '× 4', by: 4 },
]

const TICK_SPACINGS = [0.05, 0.1, 0.25, 0.5, 1, 2, 5, 10, 30, 60, 120, 300]
const MOST_TICKS = 10

const clamp = (value: number, min: number, max: number) => Math.min(Math.max(value, min), max)

/** Where a pointer is along an element, from 0 to 1 */
function along(e: PointerEvent, element: HTMLElement) {
  const { left, width } = element.getBoundingClientRect()
  return clamp((e.clientX - left) / width, 0, 1)
}

/**
 * A scene's timing, with a track for each keyframed prop and a bar for each traversal, along the bottom of the editor.
 * `graphs` are the layers' graphs as last drawn, which measure how long a traversal with a rate takes.
 */
export default function Timeline(props: {
  playback: Playback
  rendering: boolean
  error: string | null
  graphs: (RenderedGraph | null)[]
  reveal: (track: SceneTrack) => void
  openLayer: (layer: number) => void
}) {
  const scene = useContext(SceneContext)!
  const surfaces = useContext(LayerSurfacesContext)!
  const timing = () => scene.get().timing
  const tracks = createMemo(() => sceneTracks(scene, surfaces))
  const traversals = createMemo(() => sceneTraversals(scene))

  const length = ({ traversal, layer, index }: SceneTraversal) =>
    traversalLength(traversal.get(), scene.get(), props.graphs[layer]?.lengths[index] ?? null)

  /** How much time the view shows. A key, a traversal or a sound past the end is still there to be found and moved. */
  const span = createMemo(() =>
    Math.max(
      timing()?.duration ?? 1,
      ...tracks().map(({ prop }) => trackTimes(prop.get()).at(-1) ?? 0),
      ...traversals().map((run) => run.traversal.get().start + (length(run) ?? 0)),
      ...scene.get().sounds.map((layer) => soundEnd(layer, contentSoundLength(layer.sound)) ?? layer.start),
    ),
  )
  const sounds = field(scene, 'sounds')

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
          <TimingFields lens={field(scene, 'timing') as Lens<Timing>} playback={props.playback} />
        </Show>
        <Show when={props.error}>
          <span class="scene-note scene-note-warn">{props.error}</span>
        </Show>
      </header>
      <Show
        when={timing()}
        fallback={<p class="scene-note">Set any value to Keyframes to animate this scene.</p>}
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
                    <span
                      class="scene-timeline-tick"
                      classList={{ 'scene-timeline-tick-end': tick / span() > 0.9 }}
                      style={{ left: percent(tick) }}
                    >
                      {Number(tick.toFixed(2))}
                    </span>
                  )}
                </For>
              </div>
            </div>
            <Index each={sounds.get()}>
              {(_, index) => <SoundRow lens={item(sounds, index)} index={index} end={timing()!.duration} percent={percent} span={span()} />}
            </Index>
            <Index each={traversals()}>
              {(run) => <TraversalRow run={run()} length={length(run())} end={timing()!.duration} percent={percent} span={span()} open={props.openLayer} />}
            </Index>
            {/* by position, so an edit doesn't tear down the editor it was made in */}
            <Index each={tracks()} fallback={traversals().length === 0 && sounds.get().length === 0 && <p class="scene-note">Set any value to Keyframes to give it a track here.</p>}>
              {(track) => <TrackRow track={track()} percent={percent} span={span()} reveal={props.reveal} />}
            </Index>
            <div class="scene-timeline-playhead" />
          </div>
        </div>
      </Show>
    </section>
  )
}

/** The length of a cycle, and how many times it plays */
function TimingFields(props: { lens: Lens<Timing>; playback: Playback }) {
  const scene = useContext(SceneContext)!
  const cycle = field(props.lens, 'cycle')
  const kind = () => {
    const value = cycle.get()
    return typeof value === 'string' ? value : 'Times'
  }

  const scale = (by: number) => {
    batch(() => {
      scene.set(withTimesScaled(scene.get(), by))
      props.playback.rescale(by)
    })
  }

  return (
    <>
      <label>
        Length
        <NumberBox
          min={TIME_STEP}
          step="0.1"
          // a scaled length can run on for more digits than this box has room for
          value={Number(props.lens.get().duration.toFixed(3))}
          onNumber={(duration) => {
            if (duration > 0) field(props.lens, 'duration').set(duration)
          }}
          // not on each keystroke, as typing a length passes through shorter ones on the way to it
          onChange={() => scene.set(withTimesInLength(scene.get()))}
        />
      </label>
      <TimeScaleMenu duration={props.lens.get().duration} scale={scale} />
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
        <NumberBox
          min="1"
          step="1"
          aria-label="Times to play"
          value={(cycle.get() as { Times: number }).Times}
          onNumber={(times) => {
            if (Number.isInteger(times) && times > 0) cycle.set({ Times: times })
          }}
        />
      </Show>
    </>
  )
}

/**
 * A button with a menu for scaling a scene's times, which opens by resting the pointer on it as well as by clicking it.
 * It leaves the keyboard's focus alone, as it can come and go while the length is being typed in.
 */
function TimeScaleMenu(props: { duration: number; scale: (by: number) => void }) {
  const [open, setOpen] = createSignal(false)
  let timer: ReturnType<typeof setTimeout> | undefined

  const stay = () => clearTimeout(timer)
  const after = (delay: number, next: boolean) => {
    stay()
    timer = setTimeout(() => setOpen(next), delay)
  }
  const leave = () => after(SCALE_MENU_CLOSE_DELAY, false)
  onCleanup(stay)

  const option = ({ label, by }: TimeScale) => (
    <button
      type="button"
      disabled={props.duration * by < TIME_STEP}
      onClick={() => {
        props.scale(by)
        stay()
        setOpen(false)
      }}
    >
      {label}
    </button>
  )

  return (
    <Popover.Root
      lazyMount
      unmountOnExit
      autoFocus={false}
      restoreFocus={false}
      positioning={{ placement: 'top' }}
      open={open()}
      onOpenChange={(details) => {
        stay()
        setOpen(details.open)
      }}
    >
      <Popover.Trigger
        aria-label="Scale the length and everything on the timeline"
        onPointerEnter={() => (open() ? stay() : after(SCALE_MENU_OPEN_DELAY, true))}
        onPointerLeave={() => (open() ? leave() : stay())}
      >
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path d="M2 3h1.25v10H2zM12.75 3H14v10h-1.25zM4.25 8 7 5.25v2h2v-2L11.75 8 9 10.75v-2H7v2z" />
        </svg>
      </Popover.Trigger>
      <ScenePortal>
        <Popover.Positioner>
          <Popover.Content class="scene-time-scales" onPointerEnter={stay} onPointerLeave={leave}>
            <For each={SHORTER}>{option}</For>
            <span class="scene-time-scales-divider" />
            <For each={LONGER}>{option}</For>
          </Popover.Content>
        </Popover.Positioner>
      </ScenePortal>
    </Popover.Root>
  )
}

/** A prop's name and its keys, its picks, or its wave along the timeline, with a button that follows the mouse for adding a key or a pick and one for holding it still */
function TrackRow(props: { track: SceneTrack; percent: (seconds: number) => string; span: number; reveal: (track: SceneTrack) => void }) {
  const scene = useContext(SceneContext)!
  // read through `props`, as a row goes to whichever prop comes to be at its position
  const prop: Lens<Prop<SceneValue>> = { get: () => props.track.prop.get(), set: (next) => props.track.prop.set(next) }
  const times = () => trackTimes(prop.get())
  const waved = () => 'Modulated' in prop.get()
  /** The time under the mouse, and whether that's a dot's rather than one to add a dot at */
  const [hovering, setHovering] = createSignal<{ at: number; onDot: boolean } | null>(null)
  let lane!: HTMLDivElement

  /** On the page, as the tracks scroll and cut off whatever hangs out of them, and the times would cover the first */
  const addTimePlace = () => {
    const { left, top, width } = lane.getBoundingClientRect()
    return { '--x': `${left + (hovering()!.at / props.span) * width}px`, top: `${top}px` }
  }

  const hover = (e: PointerEvent & { currentTarget: HTMLElement }) => {
    if (waved()) return
    const { width } = e.currentTarget.getBoundingClientRect()
    const at = along(e, e.currentTarget) * props.span
    const nearest = times().reduce((near, time) => (Math.abs(time - at) < Math.abs(near - at) ? time : near), Infinity)
    const onDot = (Math.abs(nearest - at) / props.span) * width < KEY_REACH
    setHovering(onDot ? { at: nearest, onDot } : { at: snap(at), onDot })
  }

  const add = (at: number) => {
    const current = prop.get()
    if ('Keyframes' in current) prop.set({ Keyframes: withKey(current.Keyframes, at) })
    if ('Random' in current) prop.set({ Random: withPick(current.Random, at) })
  }

  // a track that isn't on a drawn layer is a sound's volume
  const color = (): LayerColor => (props.track.layer === null ? 'sound' : undefined)

  return (
    <LayerColorContext.Provider value={color()}>
      <div class={`scene-timeline-row ${layerColorClass(color()) ?? ''}`} classList={{ 'scene-timeline-hidden': isHidden(prop.get()) }}>
        <div class="scene-timeline-name">
          <EyeButton label={props.track.label} hidden={isHidden(prop.get())} onChange={(hidden) => prop.set(withHidden(prop.get(), hidden))} />
          <button type="button" class="scene-timeline-label" title={`Find ${props.track.label} in Config`} onClick={() => props.reveal(props.track)}>
            {props.track.label}
          </button>
        </div>
        <div class="scene-timeline-lane" ref={lane} onPointerMove={hover} onPointerLeave={() => setHovering(null)}>
          <Show when={hovering()?.onDot === false}>
            <button
              type="button"
              class="scene-key-add"
              aria-label={`Add a key to ${props.track.label}`}
              style={{ left: props.percent(hovering()!.at) }}
              onClick={() => {
                add(hovering()!.at)
                setHovering(null)
              }}
            >
              +
            </button>
          </Show>
          <Show when={hovering() !== null}>
            <ScenePortal>
              <span class="scene-key-add-time" style={addTimePlace()}>
                {hovering()!.at.toFixed(2)} s
              </span>
            </ScenePortal>
          </Show>
          <Show when={waved()}>
            <WaveLine modulated={variant(prop, 'Modulated').get()} scene={scene.get()} duration={scene.get().timing?.duration ?? props.span} percent={props.percent} />
          </Show>
          <Index each={times()}>
            {(time, index) => (
              <Show
                when={'Keyframes' in prop.get()}
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
                  keys={field(variant(prop, 'Keyframes'), 'keys')}
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
    </LayerColorContext.Provider>
  )
}

/** Most points drawn along a wave, however many times it goes by */
const WAVE_SAMPLES = 2048

/** A modulated prop's wave over the scene's first cycle, from its low end at the bottom to its high end at the top */
function WaveLine(props: { modulated: Modulated; scene: Scene; duration: number; percent: (seconds: number) => string }) {
  const points = () => {
    const samples = Math.min(Math.ceil(96 * Math.max(props.modulated.rate, 1)), WAVE_SAMPLES)
    return Array.from({ length: samples + 1 }, (_, index) => {
      const at = index / samples
      const phase = phaseAt(props.modulated.phase, props.scene, at * props.duration)
      return `${at},${1 - waveHeight(props.modulated.wave, wavePosition(props.modulated, phase, at))}`
    }).join(' ')
  }
  return (
    <svg class="scene-wave" style={{ width: props.percent(props.duration) }} viewBox="0 0 1 1" preserveAspectRatio="none" aria-hidden="true">
      <polyline points={points()} />
    </svg>
  )
}

/**
 * A wave's phase `seconds` into the cycle, as `Keyframes::resolve` in frontbox-gfx has it for keyframes, with a curve written
 * by hand taken as linear. Only the machine knows what one from anywhere else comes to, so that stays where it starts.
 */
function phaseAt(phase: Prop<number>, scene: Scene, seconds: number): number {
  if (!('Keyframes' in phase)) return startingValue(phase, scene, 0)
  const { keys, cycle } = phase.Keyframes
  const [first, last] = [keys[0], keys[keys.length - 1]]
  if (!first || !last) return 0
  const laps = cycle === 'Once' ? 1 : cycle === 'Forever' ? Infinity : cycle.Times
  const time = last.at <= 0 ? seconds : seconds >= laps * last.at ? last.at : cycle === 'Once' ? seconds : seconds % last.at
  if (time <= first.at) return first.value
  for (let index = 1; index < keys.length; index++) {
    const [a, b] = [keys[index - 1], keys[index]]
    if (time < b.at) {
      const span = b.at - a.at
      const through = span > 0 ? (time - a.at) / span : 1
      return a.value + (b.value - a.value) * (typeof b.curve === 'string' ? sample(b.curve, through) : through)
    }
  }
  return last.value
}

/** A sound layer's name, and a bar for when it plays */
function SoundRow(props: { lens: Lens<SoundLayer>; index: number; end: number; percent: (seconds: number) => string; span: number }) {
  const label = () => soundLabel(props.lens.get(), props.index)

  return (
    <div class="scene-timeline-row scene-color-sound">
      <div class="scene-timeline-name">
        <span class="scene-timeline-mark" aria-hidden="true">
          ♪
        </span>
        <span class="scene-timeline-label" title={props.lens.get().sound}>
          {label()}
        </span>
      </div>
      <div class="scene-timeline-lane">
        <SoundBar label={label()} lens={props.lens} end={props.end} percent={props.percent} span={props.span} />
      </div>
    </div>
  )
}

/**
 * When a sound plays: a bar from its start to where it ends or is cut off, between the parts of the sound that are
 * skipped and cut off. Its left edge skips more or less of the sound's beginning and its right edge moves the end, each
 * leaving the other where it is, and the rest moves the whole of it. It can't start at `end`, as that's the start of the
 * next cycle, but it can play on past it.
 */
function SoundBar(props: { label: string; lens: Lens<SoundLayer>; end: number; percent: (seconds: number) => string; span: number }) {
  const layer = () => props.lens.get()
  const length = () => contentSoundLength(layer().sound)
  const skip = () => layer().skip ?? 0
  /** Where the sound ends on its own, once that's known */
  const natural = () => soundNaturalEnd(layer(), length())
  // until its length is known, it's given some room to be grabbed by
  const shownEnd = () => soundEnd(layer(), length()) ?? layer().start + props.span / 20
  /** Where the sound's beginning would be on the timeline if none of it were skipped, which can't be before the top */
  const head = () => Math.max(layer().start - skip(), 0)
  const latest = () => Math.max(props.end - TIME_STEP, 0)
  let bar!: HTMLDivElement

  type Times = { start: number; skip: number; end: number | undefined }

  /** `layer` with its times set, playing to the end of its sound if `end` reaches it */
  const set = ({ start, skip, end }: Times) => {
    const { end: _, skip: __, ...rest } = layer()
    const natural = soundNaturalEnd({ ...rest, start, skip }, length())
    const ends = end !== undefined && (natural === undefined || end < natural - TIME_STEP / 2)
    props.lens.set({ ...rest, start, ...(skip > 0 && { skip }), ...(ends && { end }) })
  }

  /** Dragging something that sets the times from how far it's come, and from how they were when it was pressed */
  const dragged = (times: (by: number, pressed: Times) => Times) => {
    // kept from the press, as the view's length can change with where the ends are
    let press: { x: number; span: number; times: Times } | null = null
    const lane = () => bar.parentElement!
    return {
      onPointerDown(e: PointerEvent & { currentTarget: HTMLElement }) {
        // an edge is inside the bar, which would take the drag as its own
        e.stopPropagation()
        e.currentTarget.setPointerCapture(e.pointerId)
        press = { x: along(e, lane()), span: props.span, times: { start: layer().start, skip: skip(), end: layer().end ?? natural() } }
      },
      onPointerMove(e: PointerEvent) {
        if (!press) return
        set(times((along(e, lane()) - press.x) * press.span, press.times))
      },
      onPointerUp: () => (press = null),
      onPointerCancel: () => (press = null),
    }
  }

  // made once, as each of these holds its own press
  // the sound stays put on the timeline, so its start moves with how much of its beginning is skipped
  const startEdge = dragged((by, { start, skip, end }) => {
    const next = clamp(snap(start + by), Math.max(start - skip, 0), Math.min(latest(), (end ?? shownEnd()) - TIME_STEP))
    return { start: next, skip: Math.max(snap(skip + next - start), 0), end }
  })
  const endEdge = dragged((by, { start, skip, end }) => ({ start, skip, end: clamp(snap((end ?? shownEnd()) + by), start + TIME_STEP, natural() ?? Infinity) }))
  const middle = dragged((by, { start, skip, end }) => {
    const moved = clamp(snap(by), -start, latest() - start)
    return { start: snap(start + moved), skip, end: end === undefined ? undefined : snap(end + moved) }
  })

  return (
    <>
      <Show when={skip() > 0 && head() < layer().start}>
        <span
          class="scene-sound-skipped"
          title={`The first ${skip().toFixed(2)} s of ${props.label} is skipped`}
          style={{ left: props.percent(head()), width: props.percent(layer().start - head()) }}
        />
      </Show>
      <Show when={natural() !== undefined && natural()! > shownEnd()}>
        <span
          class="scene-sound-cut"
          title={`The last ${(natural()! - shownEnd()).toFixed(2)} s of ${props.label} is cut off`}
          style={{ left: props.percent(shownEnd()), width: props.percent(natural()! - shownEnd()) }}
        />
      </Show>
      <div
        class="scene-sound-bar"
        ref={bar}
        title={`${layer().sound}${length() ? `, ${length()!.toFixed(2)} s long` : ''}`}
        style={{ left: props.percent(layer().start), width: props.percent(shownEnd() - layer().start) }}
        {...middle}
      >
        <button type="button" class="scene-sound-edge scene-sound-edge-start" aria-label={`${props.label}, ${skip().toFixed(2)} seconds skipped at its beginning`} {...startEdge} />
        <button type="button" class="scene-sound-edge scene-sound-edge-end" aria-label={`${props.label} end, ${shownEnd().toFixed(2)} seconds`} {...endEdge} />
      </div>
    </>
  )
}

/**
 * A traversal's name and when it plays out: a bar from its start for as long as it takes, marking the part past the end
 * of the scene, where it's cut off. `length` is `null` while that isn't known.
 */
function TraversalRow(props: { run: SceneTraversal; length: number | null; end: number; percent: (seconds: number) => string; span: number; open: (layer: number) => void }) {
  // read through `props`, as a row goes to whichever traversal comes to be at its position
  const traversal: Lens<Traversal> = { get: () => props.run.traversal.get(), set: (next) => props.run.traversal.set(next) }
  const over = () => (props.length === null ? 0 : traversal.get().start + props.length - props.end)
  const overText = () => (over() > TIME_STEP / 2 ? `${props.run.label} runs ${over().toFixed(2)} s past the end of the scene, so it's cut off before it finishes` : null)

  return (
    <div class="scene-timeline-row">
      <div class="scene-timeline-name">
        <span class="scene-timeline-mark" classList={{ 'scene-timeline-over': overText() !== null }} title={overText() ?? undefined}>
          <Show when={overText()}>!</Show>
        </span>
        <button type="button" class="scene-timeline-label" title={`Find ${props.run.label} in Config`} onClick={() => props.open(props.run.layer)}>
          {props.run.label}
        </button>
      </div>
      <div class="scene-timeline-lane">
        <TraversalBar label={props.run.label} lens={traversal} length={props.length} end={props.end} over={overText()} percent={props.percent} span={props.span} />
      </div>
    </div>
  )
}

/**
 * When a traversal plays out, with a grip for moving its start and, for one that finishes in a time, an arrow for
 * changing that time. The part past `end` is marked, with `over` saying what that means.
 */
function TraversalBar(props: {
  label: string
  lens: Lens<Traversal>
  length: number | null
  end: number
  over: string | null
  percent: (seconds: number) => string
  span: number
}) {
  const traversal = () => props.lens.get()
  const length = () => props.length ?? 0
  let bar!: HTMLDivElement

  /** Dragging something that changes the traversal by how far it's come, from how it was when it was pressed */
  const dragged = (moved: (by: number, pressed: { start: number; length: number; span: number }) => Partial<Traversal>) => {
    // kept from the press, as the view's length can change with where the bar is
    let press: { x: number; start: number; length: number; span: number } | null = null
    const lane = () => bar.parentElement!
    return {
      onPointerDown(e: PointerEvent & { currentTarget: HTMLElement }) {
        e.currentTarget.setPointerCapture(e.pointerId)
        press = { x: along(e, lane()), start: traversal().start, length: length(), span: props.span }
      },
      onPointerMove(e: PointerEvent) {
        if (!press) return
        props.lens.set({ ...traversal(), ...moved((along(e, lane()) - press.x) * press.span, press) })
      },
      onPointerUp: () => (press = null),
      onPointerCancel: () => (press = null),
    }
  }

  // made once, as each of these holds its own press
  const grip = dragged((by, { start }) => ({ start: clamp(snap(start + by), 0, Math.max(props.end - TIME_STEP, 0)) }))
  const endArrow = dragged((by, { start, length, span }) => ({ speed: { Duration: clamp(snap(length + by), TIME_STEP, span - start) } }))

  /** Where the end of the scene falls along the bar, as a share of it */
  const endAlong = () => (length() > 0 ? clamp((props.end - traversal().start) / length(), 0, 1) : 0)

  return (
    <div class="scene-sweep scene-traversal" ref={bar} style={{ left: props.percent(traversal().start), width: props.percent(length()) }} title={props.over ?? undefined}>
      <Show when={props.over}>
        <span class="scene-traversal-over" style={{ left: `${endAlong() * 100}%` }} />
      </Show>
      <button type="button" class="scene-sweep-grip" aria-label={`Move ${props.label} along the timeline, starting at ${traversal().start.toFixed(2)} seconds`} {...grip} />
      <Show when={'Duration' in traversal().speed}>
        <button type="button" class="scene-sweep-end scene-sweep-finish" aria-label={`${props.label} finishes in ${length().toFixed(2)} seconds`} {...endArrow}>
          ›
        </button>
      </Show>
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

/** A dot on a track. Clicking it opens what can be changed about it, and dragging it moves it along the track, past the others if it comes to them. */
function Dot<T extends { at: number }>(props: DotPlace & { label: string; moments: Lens<T[]>; pick?: boolean; background?: string; children: any }) {
  const [open, setOpen] = createSignal(false)
  // kept from the press, as the view's length can change with where the dot is. Dots are matched to moments by position,
  // so this is what follows the moment being dragged as it passes others, while the press stays with this dot.
  let press: { x: number; index: number; span: number } | null = null
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
          press = { x: e.clientX, index: props.index, span: props.span }
        }}
        onPointerMove={(e) => {
          if (!press || (!dragged && Math.abs(e.clientX - press.x) < DRAG_START)) return
          // what's open would otherwise be left showing whichever moment takes this one's place
          if (!dragged) setOpen(false)
          dragged = true
          const at = clamp(snap(along(e, e.currentTarget.parentElement!) * press.span), 0, press.span)
          press.index = moveMoment(props.moments, press.index, at)
        }}
        onPointerUp={() => (press = null)}
        onPointerCancel={() => (press = null)}
      />
      <ScenePortal>
        <Popover.Positioner>
          <Popover.Content class="scene-key-editor">{props.children}</Popover.Content>
        </Popover.Positioner>
      </ScenePortal>
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
    <Dot {...props} label={`${props.label} pick at ${pick().get().at.toFixed(2)} seconds`} moments={props.picks} pick>
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

  const background = () => {
    const value = key().get().value
    return Array.isArray(value) && value.length === 4 ? `rgb(${value.slice(0, 3).join(' ')})` : undefined
  }

  return (
    <Dot {...props} label={`${props.label} key at ${at().toFixed(2)} seconds`} moments={props.keys} background={background()}>
      <FieldRow label="Value">
        <ValueField label="Value" lens={field(key(), 'value')} ranges={props.ranges.ranges} />
      </FieldRow>
      <Show when={props.index > 0}>
        <FieldRow label="Curve into this key">
          <CurveField label="Curve into this key" lens={field(key(), 'curve')} />
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
        {/* a track with no keys reads as zero, and leaving it is a matter of switching it away from Keyframes */}
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
