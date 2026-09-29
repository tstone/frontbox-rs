import type { TraceRecord } from '../types/generated/TraceRecord'

/**
 * The last path segment of a Rust type name, keeping any generics.
 * `lotko::hardware::captive_ball::CaptiveBallSystem` becomes `CaptiveBallSystem`.
 */
export function shortName(typeName: string): string {
  const generics = typeName.indexOf('<')
  const path = generics === -1 ? typeName : typeName.slice(0, generics)
  const short = path.slice(path.lastIndexOf('::') + 2)
  return generics === -1 ? short : `${short}<…>`
}

export type TraceCategory = 'event' | 'switch' | 'driver' | 'system'

/** What a log row is labelled and filtered by: the event's type, or the trace variant */
export function traceType(record: TraceRecord): string {
  const event = record.event
  return 'Event' in event ? shortName(event.Event.type_name) : Object.keys(event)[0]
}

export function traceCategory(record: TraceRecord): TraceCategory {
  const event = record.event
  if ('Event' in event) return 'event'
  if ('SwitchStateChange' in event) return 'switch'
  if ('DriverStateChange' in event) return 'driver'
  return 'system'
}

/** The part of a trace worth showing in detail. `null` for events without a body. */
export function traceBody(record: TraceRecord): unknown {
  const event = record.event
  if ('Event' in event) {
    const { event: body, interrupts } = event.Event
    return interrupts.length > 0 ? { event: body, interrupts } : body
  }
  return Object.values(event)[0]
}

/** Players are zero-based in the framework but people count from one */
export function playerLabel(player: number | null): string {
  return player === null ? 'No player' : `Player ${player + 1}`
}

/** `m:ss.mmm`, or `h:mm:ss.mmm` past the first hour */
export function formatElapsed(ms: number): string {
  const hours = Math.floor(ms / 3_600_000)
  const minutes = Math.floor(ms / 60_000) % 60
  const seconds = Math.floor(ms / 1000) % 60
  const millis = ms % 1000
  const pad = (n: number, width = 2) => String(n).padStart(width, '0')
  const tail = `${pad(seconds)}.${pad(millis, 3)}`
  return hours > 0 ? `${hours}:${pad(minutes)}:${tail}` : `${minutes}:${tail}`
}

const ROOT_GROUP = '__root'

/** System group keys, with the framework's root group given a friendlier name */
export function groupLabel(key: string): string {
  return key === ROOT_GROUP ? 'Root' : key
}
