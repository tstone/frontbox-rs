import { createSignal } from 'solid-js'
import { machine } from '../../state/console'

/** The latest time the console knows of: the newest record's, in milliseconds since the console started */
export const consoleNow = () => machine.log.at(-1)?.at_ms ?? 0

/**
 * When every system's timeline was paused, or `null` while they're live. Shared, so switching systems keeps the same
 * moment. History keeps being recorded meanwhile, so resuming shows everything since.
 */
const [pausedAt, setPausedAt] = createSignal<number | null>(null)

export { pausedAt }

export function togglePause() {
  setPausedAt(pausedAt() === null ? consoleNow() : null)
}
