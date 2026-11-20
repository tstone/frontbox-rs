import { createMemo, createSignal, For, Show } from 'solid-js'
import { consoleNow, pausedAt } from './timelinePause'
import { activityCause, despawnCause, stepEnding } from '../../lib/causes'
import { shortName } from '../../lib/format'
import { machine } from '../../state/console'
import type { HistoryEntry } from '../../types/generated/HistoryEntry'
import type { System } from '../../types/generated/System'

type Step = Extract<HistoryEntry, { kind: 'RoutineStep' }>
type Change = Extract<HistoryEntry, { kind: 'ActivityChanged' | 'Spawned' | 'Despawned' }>
type Emitted = Extract<HistoryEntry, { kind: 'Emitted' }>

/** How far back the timeline looks, in milliseconds. `null` for the system's whole life. */
const WINDOWS: { label: string; ms: number | null }[] = [
  { label: '15s', ms: 15_000 },
  { label: '1m', ms: 60_000 },
  { label: '5m', ms: 300_000 },
  { label: 'All', ms: null },
]

/** Lane height and the gap between lanes, in pixels */
const LANE = 22
const GAP = 6
const LANES = ['Active', 'Routines', 'Emitted', 'Children'] as const
const laneY = (lane: (typeof LANES)[number]) => 4 + LANES.indexOf(lane) * (LANE + GAP)
const HEIGHT = laneY('Children') + LANE + 4

/**
 * A system's history over time, in four lanes: when it was active, the routine steps it held, the events it emitted,
 * and its children spawning and despawning. Hovering anything says what it was and why. Positions are percentages of
 * the width, so the SVG stretches to the panel without squashing its text.
 *
 * Pausing (`timelinePause`, for every system) freezes it at that moment. The history keeps being recorded meanwhile,
 * so resuming shows everything since.
 */
export default function SystemTimeline(props: {
  system: System
  hovered: Hovered | null
  onHover: (hovered: Hovered | null) => void
}) {
  const [windowMs, setWindowMs] = createSignal<number | null>(60_000)
  const [tip, setTip] = createSignal<{ text: string; left: number; top: number } | null>(null)
  let body: HTMLDivElement | undefined

  // now, or the moment everything was paused, but no later than its despawn
  const end = () =>
    Math.max(Math.min(pausedAt() ?? consoleNow(), props.system.despawned_ms ?? Infinity), props.system.spawned_ms)
  const endsAtDespawn = () => props.system.despawned_ms !== null && end() === props.system.despawned_ms
  const start = () => {
    const ms = windowMs()
    return ms === null ? props.system.spawned_ms : Math.max(props.system.spawned_ms, end() - ms)
  }
  const span = () => Math.max(end() - start(), 1)
  /** Where a time falls across the width, in percent */
  const x = (at: number) => ((Math.min(Math.max(at, start()), end()) - start()) / span()) * 100
  const pct = (value: number) => `${value}%`
  const inWindow = (at: number) => at >= start() && at <= end()

  // what had happened by the end: everything when live, and nothing after the moment it was paused
  const history = () => props.system.history.filter((entry) => entryTime(entry) <= end())
  // a step that ended after the pause shows as still held
  const stepEnd = (step: Step) => (step.ended_ms !== null && step.ended_ms <= end() ? step.ended_ms : null)

  // when it was active, as spans, from its spawn and each change after
  const activeSpans = createMemo(() => {
    const spans: { from: number; to: number }[] = []
    let since: number | null = null
    for (const entry of history()) {
      if (entry.kind === 'Spawned' && entry.active) since = entry.at_ms
      if (entry.kind === 'ActivityChanged') {
        if (entry.active) since ??= entry.at_ms
        else if (since !== null) {
          spans.push({ from: since, to: entry.at_ms })
          since = null
        }
      }
    }
    if (since !== null) spans.push({ from: since, to: end() })
    return spans.filter((s) => s.to >= start())
  })

  const changes = () =>
    history().filter(
      (e): e is Change =>
        (e.kind === 'ActivityChanged' || e.kind === 'Spawned' || e.kind === 'Despawned') && inWindow(e.at_ms),
    )
  const steps = () => history().filter((e): e is Step => e.kind === 'RoutineStep' && (stepEnd(e) ?? end()) >= start())
  const emitted = () => history().filter((e): e is Emitted => e.kind === 'Emitted' && inWindow(e.at_ms))

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
      if (entry.started_ms > end() || stepTo < start()) return null
      return { from: x(entry.started_ms), to: x(stepTo) }
    }
    return inWindow(entry.at_ms) ? { from: x(entry.at_ms), to: null } : null
  }

  /** Hover handlers that show `text` beside the pointer, and point out `entry` in the activity list */
  const hover = (text: () => string, entry?: HistoryEntry) => ({
    onMouseMove: (event: MouseEvent) => {
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
          <For each={WINDOWS}>
            {(w) => (
              <button classList={{ selected: windowMs() === w.ms }} onClick={() => setWindowMs(w.ms)}>
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
        <svg height={HEIGHT} role="img" aria-label="System timeline">
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
          <span>{start() === props.system.spawned_ms ? 'spawned' : `-${seconds(end() - start())}`}</span>
          <span>{endsAtDespawn() ? 'despawned' : pausedAt() !== null ? 'paused' : 'now'}</span>
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
  return ms < 1000 ? `${ms}ms` : `${(ms / 1000).toFixed(1)}s`
}

/** A child's name for a tooltip: a `Group`'s own name, or its type's short name */
function childLabel(child: System): string {
  return child.group ? `group ${child.name}` : shortName(child.name)
}
