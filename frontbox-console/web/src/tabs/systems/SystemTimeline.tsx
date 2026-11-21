import { createMemo, createSignal, For, onCleanup, onMount, Show } from 'solid-js'
import { consoleNow, pausedAt } from './timelinePause'
import { ZoomIn, ZoomOut } from '../../components/icons'
import { activityCause, despawnCause, stepEnding } from '../../lib/causes'
import { shortName } from '../../lib/format'
import { machine } from '../../state/console'
import type { HistoryEntry } from '../../types/generated/HistoryEntry'
import type { System } from '../../types/generated/System'

type Step = Extract<HistoryEntry, { kind: 'RoutineStep' }>
type Change = Extract<HistoryEntry, { kind: 'ActivityChanged' | 'Spawned' | 'Despawned' }>
type Emitted = Extract<HistoryEntry, { kind: 'Emitted' }>
type Reported = Extract<HistoryEntry, { kind: 'StatusReported' }>

/** How much time the timeline shows, in milliseconds. `null` for the system's whole life. */
const WINDOWS: { label: string; ms: number | null }[] = [
  { label: '1m', ms: 60_000 },
  { label: '5m', ms: 300_000 },
  { label: '30m', ms: 1_800_000 },
  { label: 'All', ms: null },
]

/** How much a zoom button, or a notch of the scroll wheel, zooms by */
const ZOOM_STEP = 2
const WHEEL_STEP = 1.25
/** The shortest time the timeline zooms in to, in milliseconds */
const MIN_SPAN = 200

/** Lane height and the gap between lanes, in pixels */
const LANE = 22
const GAP = 6
const LANES = ['Active', 'Routines', 'Emitted', 'Status', 'Children'] as const
const laneY = (lane: (typeof LANES)[number]) => 4 + LANES.indexOf(lane) * (LANE + GAP)
const HEIGHT = laneY('Children') + LANE + 4

/**
 * A system's history over time, in five lanes: when it was active, the routine steps it held, the events it emitted,
 * the statuses it reported, and its children spawning and despawning. Hovering anything says what it was and why.
 * Positions are percentages of the width, so the SVG stretches to the panel without squashing its text.
 *
 * It starts showing the system's whole life. The scroll wheel zooms around the pointer, and dragging moves through
 * time. Moving back from the end leaves the view where it is as new history comes in; moving to the end follows it
 * again.
 *
 * Pausing (`timelinePause`, for every system) freezes it at that moment. The history keeps being recorded meanwhile,
 * so resuming shows everything since.
 */
export default function SystemTimeline(props: {
  system: System
  hovered: Hovered | null
  onHover: (hovered: Hovered | null) => void
}) {
  // how much time it shows (`null` for its whole life), and where the view's right edge is (`null` to follow the end)
  const [windowMs, setWindowMs] = createSignal<number | null>(null)
  const [viewTo, setViewTo] = createSignal<number | null>(null)
  const [dragging, setDragging] = createSignal(false)
  const [tip, setTip] = createSignal<{ text: string; left: number; top: number } | null>(null)
  let body: HTMLDivElement | undefined
  let svg: SVGSVGElement | undefined

  // when its history starts: when it was spawned, or when the history was cleared since
  const since = () => {
    const first = props.system.history[0]
    return first?.kind === 'HistoryCleared' ? first.at_ms : props.system.spawned_ms
  }
  const cleared = () => props.system.history[0]?.kind === 'HistoryCleared'
  // now, or the moment everything was paused, but no later than its despawn
  const end = () =>
    Math.max(Math.min(pausedAt() ?? consoleNow(), props.system.despawned_ms ?? Infinity), since())
  const endsAtDespawn = () => props.system.despawned_ms !== null && end() === props.system.despawned_ms

  // the part of its life in view
  const start = () => {
    const ms = windowMs()
    if (ms === null) return since()
    return Math.max(since(), Math.min(viewTo() ?? Infinity, end()) - ms)
  }
  const to = () => {
    const ms = windowMs()
    return ms === null ? end() : Math.min(end(), start() + ms)
  }
  const span = () => Math.max(to() - start(), 1)
  /** Where a time falls across the width, in percent */
  const x = (at: number) => ((Math.min(Math.max(at, start()), to()) - start()) / span()) * 100
  const pct = (value: number) => `${value}%`
  const inWindow = (at: number) => at >= start() && at <= to()

  /**
   * Shows `ms` of time from `from`, keeping it within the system's life. Showing it all goes back to `All`, and
   * reaching the end follows it again.
   */
  const show = (from: number, ms: number) => {
    const life = end() - since()
    if (ms >= life) {
      setWindowMs(null)
      setViewTo(null)
      return
    }
    ms = Math.max(ms, MIN_SPAN)
    const rightEdge = Math.min(Math.max(from, since()) + ms, end())
    setWindowMs(ms)
    // within a hair of the end counts as at it, so zooming in near the right edge keeps following
    setViewTo(rightEdge >= end() - ms * 0.01 ? null : rightEdge)
  }

  /** Zooms by `factor` (above 1 zooms in) around the point `fraction` of the way across */
  const zoom = (factor: number, fraction: number) => {
    const ms = span() / factor
    show(start() + span() * fraction - ms * fraction, ms)
  }
  // the buttons zoom around the right edge while following the end, so it stays in view, and the middle otherwise
  const zoomFromButton = (factor: number) => zoom(factor, viewTo() === null ? 1 : 0.5)

  const fractionAcross = (clientX: number) => {
    const bounds = svg!.getBoundingClientRect()
    return Math.min(Math.max((clientX - bounds.left) / bounds.width, 0), 1)
  }

  onMount(() => {
    // added by hand, as it must be able to stop the page scrolling
    const onWheel = (event: WheelEvent) => {
      if (event.deltaY === 0) return
      event.preventDefault()
      zoom(event.deltaY < 0 ? WHEEL_STEP : 1 / WHEEL_STEP, fractionAcross(event.clientX))
    }
    svg!.addEventListener('wheel', onWheel, { passive: false })
    onCleanup(() => svg?.removeEventListener('wheel', onWheel))
  })

  // dragging moves the view through time, by the time under the distance dragged
  const startDrag = (event: MouseEvent) => {
    if (event.button !== 0 || windowMs() === null) return
    event.preventDefault()
    const fromX = event.clientX
    const fromStart = start()
    const ms = span()
    const width = svg!.getBoundingClientRect().width
    setDragging(true)
    setTip(null)
    const onMove = (move: MouseEvent) => show(fromStart - ((move.clientX - fromX) / width) * ms, ms)
    const onUp = () => {
      setDragging(false)
      window.removeEventListener('mousemove', onMove)
      window.removeEventListener('mouseup', onUp)
    }
    window.addEventListener('mousemove', onMove)
    window.addEventListener('mouseup', onUp)
  }

  // what had happened by the end: everything when live, and nothing after the moment it was paused
  const history = () => props.system.history.filter((entry) => entryTime(entry) <= end())
  // a step that ended after the pause shows as still held
  const stepEnd = (step: Step) => (step.ended_ms !== null && step.ended_ms <= end() ? step.ended_ms : null)

  // when it was active, as spans, from its spawn (or its history being cleared) and each change after
  const activeSpans = createMemo(() => {
    const spans: { from: number; to: number }[] = []
    let activeFrom: number | null = null
    for (const entry of history()) {
      if ((entry.kind === 'Spawned' || entry.kind === 'HistoryCleared') && entry.active) activeFrom = entry.at_ms
      if (entry.kind === 'ActivityChanged') {
        if (entry.active) activeFrom ??= entry.at_ms
        else if (activeFrom !== null) {
          spans.push({ from: activeFrom, to: entry.at_ms })
          activeFrom = null
        }
      }
    }
    if (activeFrom !== null) spans.push({ from: activeFrom, to: end() })
    return spans.filter((s) => s.to >= start() && s.from <= to())
  })

  const changes = () =>
    history().filter(
      (e): e is Change =>
        (e.kind === 'ActivityChanged' || e.kind === 'Spawned' || e.kind === 'Despawned') && inWindow(e.at_ms),
    )
  const steps = () =>
    history().filter(
      (e): e is Step => e.kind === 'RoutineStep' && e.started_ms <= to() && (stepEnd(e) ?? end()) >= start(),
    )
  const emitted = () => history().filter((e): e is Emitted => e.kind === 'Emitted' && inWindow(e.at_ms))
  const reported = () => history().filter((e): e is Reported => e.kind === 'StatusReported' && inWindow(e.at_ms))

  // its children spawning and despawning. Despawned ones are only kept for a while, so older ones drop off.
  const childMarks = () =>
    machine.systems
      .filter((s) => s.parent === props.system.id)
      .flatMap((child) => [
        { kind: 'spawned' as const, at_ms: child.spawned_ms, child },
        ...(child.despawned_ms === null ? [] : [{ kind: 'despawned' as const, at_ms: child.despawned_ms, child }]),
      ])
      .filter((m) => inWindow(m.at_ms))

  /** Where to point out an entry, in percent across: a step's span, or a moment. `null` outside the window. */
  const pointedOut = (entry: HistoryEntry): { from: number; to: number | null } | null => {
    if (entry.kind === 'RoutineStep') {
      const stepTo = stepEnd(entry) ?? end()
      if (entry.started_ms > to() || stepTo < start()) return null
      return { from: x(entry.started_ms), to: x(stepTo) }
    }
    return inWindow(entry.at_ms) ? { from: x(entry.at_ms), to: null } : null
  }

  /** Hover handlers that show `text` beside the pointer, and point out `entry` in the activity list */
  const hover = (text: () => string, entry?: HistoryEntry) => ({
    onMouseMove: (event: MouseEvent) => {
      if (dragging()) return
      const bounds = body?.getBoundingClientRect()
      if (bounds) setTip({ text: text(), left: event.clientX - bounds.left, top: event.clientY - bounds.top })
      if (entry && props.hovered?.key !== entryKey(entry)) props.onHover({ key: entryKey(entry), entry, from: 'timeline' })
    },
    onMouseLeave: () => {
      setTip(null)
      if (entry) props.onHover(null)
    },
  })

  return (
    <section class="system-timeline">
      <div class="timeline-header">
        <h3 class="pane-title">Timeline</h3>
        <Show when={pausedAt() !== null}>
          <span class="paused-tag">Paused</span>
        </Show>
        <div class="timeline-controls">
          <button title="Zoom in" aria-label="Zoom in" onClick={() => zoomFromButton(ZOOM_STEP)}>
            <ZoomIn size={14} />
          </button>
          <button
            title="Zoom out"
            aria-label="Zoom out"
            disabled={windowMs() === null}
            onClick={() => zoomFromButton(1 / ZOOM_STEP)}
          >
            <ZoomOut size={14} />
          </button>
          <span class="separator" />
          <For each={WINDOWS}>
            {(w) => (
              <button
                classList={{ selected: windowMs() === w.ms }}
                onClick={() => {
                  setWindowMs(w.ms)
                  setViewTo(null)
                }}
              >
                {w.label}
              </button>
            )}
          </For>
        </div>
      </div>

      <div class="timeline-body" ref={body}>
        <div class="lane-labels">
          <For each={LANES}>{(lane) => <span style={{ height: `${LANE}px`, 'margin-bottom': `${GAP}px` }}>{lane}</span>}</For>
        </div>
        <svg
          ref={svg}
          height={HEIGHT}
          role="img"
          aria-label="System timeline"
          classList={{ pannable: windowMs() !== null, dragging: dragging() }}
          onMouseDown={startDrag}
        >
          <For each={LANES}>{(lane) => <rect class="lane" x="0" y={laneY(lane)} width="100%" height={LANE} />}</For>

          <For each={activeSpans()}>
            {(s) => (
              <rect
                class="active-span"
                x={pct(x(s.from))}
                y={laneY('Active')}
                width={pct(Math.max(x(s.to) - x(s.from), 0.2))}
                height={LANE}
                {...hover(() => `Active for ${seconds(s.to - s.from)}`)}
              />
            )}
          </For>
          <For each={changes()}>
            {(c) => (
              <Mark
                class={`change ${c.kind}`}
                at={pct(x(c.at_ms))}
                y1={laneY('Active') - 3}
                y2={laneY('Active') + LANE + 3}
                {...hover(() => describe(c), c)}
              />
            )}
          </For>

          <For each={steps()}>
            {(s) => {
              const from = () => x(s.started_ms)
              const width = () => Math.max(x(stepEnd(s) ?? end()) - from(), 0.3)
              const shown = (): Step => ({ ...s, ended_ms: stepEnd(s), ending: stepEnd(s) === null ? null : s.ending })
              return (
                <g
                  classList={{ step: true, halted: shown().ending === 'Halted', open: shown().ended_ms === null }}
                  {...hover(() => describe(shown()), s)}
                >
                  <rect x={pct(from())} y={laneY('Routines') + 2} width={pct(width())} height={LANE - 4} rx={3} />
                  <Show when={width() > 8}>
                    <svg x={pct(from())} y={laneY('Routines')} width={pct(width())} height={LANE}>
                      <text x="4" y={LANE / 2 + 4}>
                        {shortName(s.routine)}
                      </text>
                    </svg>
                  </Show>
                </g>
              )
            }}
          </For>

          <For each={emitted()}>
            {(e) => (
              <Mark
                class="emitted"
                at={pct(x(e.at_ms))}
                y1={laneY('Emitted') + 3}
                y2={laneY('Emitted') + LANE - 3}
                {...hover(() => describe(e), e)}
              />
            )}
          </For>

          <For each={reported()}>
            {(r) => (
              <Mark
                class="reported"
                at={pct(x(r.at_ms))}
                y1={laneY('Status') + 3}
                y2={laneY('Status') + LANE - 3}
                {...hover(() => describe(r), r)}
              />
            )}
          </For>

          <For each={childMarks()}>
            {(m) => (
              <Mark
                class={`child ${m.kind}`}
                at={pct(x(m.at_ms))}
                y1={laneY('Children') + 3}
                y2={laneY('Children') + LANE - 3}
                {...hover(() => `${m.kind === 'spawned' ? 'Spawned' : 'Despawned'} ${childLabel(m.child)}`)}
              />
            )}
          </For>

          {/* what's hovered in the activity list (or here), across every lane: a line, or a band for a step */}
          <Show when={props.hovered && pointedOut(props.hovered.entry)}>
            {(at) => (
              <Show
                when={at().to !== null}
                fallback={<line class="pointed-out" x1={pct(at().from)} x2={pct(at().from)} y1={0} y2={HEIGHT} />}
              >
                <rect
                  class="pointed-out"
                  x={pct(at().from)}
                  y={0}
                  width={pct(Math.max(at().to! - at().from, 0.3))}
                  height={HEIGHT}
                />
              </Show>
            )}
          </Show>
        </svg>
        <span />
        <div class="axis">
          <span>{start() === since() ? (cleared() ? 'cleared' : 'spawned') : `-${seconds(end() - start())}`}</span>
          <span>
            {to() < end()
              ? `-${seconds(end() - to())}`
              : endsAtDespawn()
                ? 'despawned'
                : pausedAt() !== null
                  ? 'paused'
                  : 'now'}
          </span>
        </div>

        <Show when={tip()}>
          {(t) => (
            <div class="timeline-tip" style={{ left: `${t().left + 12}px`, top: `${t().top + 12}px` }}>
              {t().text}
            </div>
          )}
        </Show>
      </div>
    </section>
  )
}

/**
 * A vertical mark at one moment, with a wider invisible line over it so it's easy to hover. Hover handlers go on the
 * group, which covers both.
 */
function Mark(props: {
  class: string
  at: string
  y1: number
  y2: number
  onMouseMove: (event: MouseEvent) => void
  onMouseLeave: () => void
}) {
  return (
    <g class={props.class} onMouseMove={props.onMouseMove} onMouseLeave={props.onMouseLeave}>
      <line class="mark" x1={props.at} x2={props.at} y1={props.y1} y2={props.y2} />
      <line class="hit" x1={props.at} x2={props.at} y1={props.y1} y2={props.y2} />
    </g>
  )
}

/** One history entry, as a sentence for a tooltip or the activity log */
export function describe(entry: HistoryEntry): string {
  switch (entry.kind) {
    case 'Spawned':
      return `Spawned${entry.active ? '' : ', inactive'}`
    case 'ActivityChanged':
      return `${entry.active ? 'Activated' : 'Deactivated'} ${activityCause(entry.cause)}`
    case 'RoutineStep': {
      const held = entry.ended_ms === null ? 'still holding it' : `for ${seconds(entry.ended_ms - entry.started_ms)}`
      const how = entry.ending === null ? '' : `, ${stepEnding(entry.ending)}`
      return `Step in ${shortName(entry.routine)}, ${held}${how}`
    }
    case 'Emitted':
      return `Emitted ${shortName(entry.event)}`
    case 'StatusReported':
      return 'Reported its status'
    case 'HistoryCleared':
      return `History cleared${entry.active ? '' : ', inactive'}`
    case 'Despawned':
      return `Despawned ${despawnCause(entry.cause)}`
  }
}

/** An entry hovered in the timeline or the activity list, which the other one points out */
export type Hovered = { key: string; entry: HistoryEntry; from: 'timeline' | 'list' }

/** Tells entries apart, the same in the timeline and the activity list */
export function entryKey(entry: HistoryEntry): string {
  switch (entry.kind) {
    case 'Emitted':
      return `emitted-${entry.seq}`
    case 'StatusReported':
      return `status-${entry.seq}`
    case 'RoutineStep':
      return `step-${entry.started_ms}-${entry.routine}`
    default:
      return `${entry.kind}-${entry.at_ms}`
  }
}

/** When the entry happened: its start, for a routine step */
export function entryTime(entry: HistoryEntry): number {
  return entry.kind === 'RoutineStep' ? entry.started_ms : entry.at_ms
}

function seconds(ms: number): string {
  if (ms < 1000) return `${Math.round(ms)}ms`
  if (ms < 60_000) return `${(ms / 1000).toFixed(1)}s`
  const s = Math.floor(ms / 1000)
  return s < 3600 ? `${Math.floor(s / 60)}m ${s % 60}s` : `${Math.floor(s / 3600)}h ${Math.floor(s / 60) % 60}m`
}

/** A child's name for a tooltip: a `Group`'s own name, or its type's short name */
function childLabel(child: System): string {
  return child.group ? `group ${child.name}` : shortName(child.name)
}
