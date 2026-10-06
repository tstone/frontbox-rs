import { machine } from '../state/console'
import type { Color } from '../types/generated/Color'
import type { LedChannels } from '../types/generated/LedChannels'

/**
 * Colors are traced in the order they're sent on the wire, which depends on how each LED is wired (GRB, BRG, ...).
 * This puts them back in red, green, blue order. Mirrors `remap` in frontbox/src/led/rgba_color.rs.
 */
function toRgb({ r: first, g: second, b: third }: Color, channels: LedChannels): Color {
  switch (channels) {
    case 'GRB':
    case 'GRBW':
      return { r: second, g: first, b: third }
    case 'BRG':
    case 'BRGW':
      return { r: second, g: third, b: first }
    default:
      return { r: first, g: second, b: third }
  }
}

const hex = (n: number) => n.toString(16).padStart(2, '0')

/**
 * The color an LED is showing, as CSS (`#rrggbb`), or `null` if the console hasn't seen one yet or it's off.
 */
export function ledColor(name: string): string | null {
  const wire = machine.led_colors[name]
  if (!wire) return null
  const channels = machine.hardware?.leds.configs[name]?.channels ?? 'RGB'
  const { r, g, b } = toRgb(wire, channels)
  if (r === 0 && g === 0 && b === 0) return null
  return `#${hex(r)}${hex(g)}${hex(b)}`
}

/** Whether the console has seen a color for this LED, so "off" can be told apart from "unknown" */
export const ledColorKnown = (name: string) => machine.led_colors[name] !== undefined
