import { machine } from '../state/console'
import type { ChannelOrder } from '../types/generated/ChannelOrder'
import type { Rgb } from '../types/generated/Rgb'

/**
 * Colors are traced in the order they're sent on the wire, which depends on how each LED is wired (GRB, BRG, ...).
 * This puts them back in red, green, blue order. Mirrors `ChannelOrder::reorder` in
 * frontbox-core/src/hardware/exp/channel_order.rs.
 */
function toRgb({ r: first, g: second, b: third }: Rgb, order: ChannelOrder): Rgb {
  switch (order) {
    case 'GRB':
      return { r: second, g: first, b: third }
    case 'BRG':
      return { r: second, g: third, b: first }
    default:
      return { r: first, g: second, b: third }
  }
}

const hex = (n: number) => n.toString(16).padStart(2, '0')

/** A color in red, green, blue order as CSS (`#rrggbb`), or `null` if it's off */
export function cssColor({ r, g, b }: Rgb): string | null {
  if (r === 0 && g === 0 && b === 0) return null
  return `#${hex(r)}${hex(g)}${hex(b)}`
}

/**
 * The color an LED is showing, as CSS (`#rrggbb`), or `null` if the console hasn't seen one yet or it's off.
 */
export function ledColor(name: string): string | null {
  const wire = machine.led_colors[name]
  if (!wire) return null
  const order = machine.hardware?.leds.configs[name]?.channel_order ?? 'RGB'
  return cssColor(toRgb(wire, order))
}

/** Whether the console has seen a color for this LED, so "off" can be told apart from "unknown" */
export const ledColorKnown = (name: string) => machine.led_colors[name] !== undefined

/** How an LED with no color (off, or not seen yet) is drawn */
export const LED_OFF = '#000000'
