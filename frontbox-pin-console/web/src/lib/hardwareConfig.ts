import type { DriverMode } from '../types/generated/DriverMode'
import type { SwitchConfig } from '../types/generated/SwitchConfig'

export type ConfigField = { label: string; value: string }

export function describeDriverMode(mode: DriverMode): { mode: string; fields: ConfigField[] } {
  const [name, body] = Object.entries(mode)[0] as [string, Record<string, unknown>]
  const fields = Object.entries(body)
    .filter(([, value]) => value !== null && value !== undefined)
    .map(([key, value]) => ({
      label: humanize(key),
      value: key === 'trigger_mode' ? describeTrigger(value) : describeValue(value),
    }))
  return { mode: splitWords(name), fields }
}

export function describeSwitchConfig(config: SwitchConfig): ConfigField[] {
  const fields: ConfigField[] = [{ label: 'Inverted', value: config.inverted ? 'Yes' : 'No' }]
  if (config.debounce_close !== null) fields.push({ label: 'Debounce close', value: `${config.debounce_close} ms` })
  if (config.debounce_open !== null) fields.push({ label: 'Debounce open', value: `${config.debounce_open} ms` })
  return fields
}

/** FAST power is an 8ms window, one bit per millisecond, so duty is the share of bits set */
export function describePower(power: number): string {
  const on = power.toString(2).split('').filter((bit) => bit === '1').length
  const bits = power.toString(2).padStart(8, '0')
  return `${Math.round((on / 8) * 100)}% (${bits.slice(0, 4)} ${bits.slice(4)})`
}

export function hexAddress(address: number): string {
  return `0x${address.toString(16).toUpperCase().padStart(2, '0')}`
}

function describeValue(value: unknown): string {
  if (typeof value === 'boolean') return value ? 'Yes' : 'No'
  if (typeof value === 'string') return value
  if (typeof value === 'object' && value !== null) {
    // HardwareValue: fixed, or backed by an operator setting
    if ('Fixed' in value) return describeScalar(value.Fixed)
    if ('Config' in value) {
      const config = value.Config as { name: string; default: unknown }
      return `${describeScalar(config.default)} (operator setting "${config.name}")`
    }
  }
  return JSON.stringify(value)
}

function describeScalar(value: unknown): string {
  // durations are the only plain numbers inside hardware values
  if (typeof value === 'number') return `${value} ms`
  if (typeof value === 'object' && value !== null && 'power' in value) return describePower(value.power as number)
  return JSON.stringify(value)
}

function describeTrigger(trigger: unknown): string {
  if (typeof trigger === 'string') {
    const labels: Record<string, string> = {
      Disabled: 'Disabled',
      VirtualSwitchTrue: 'Software (virtual switch on)',
      VirtualSwitchFalse: 'Software (virtual switch off)',
    }
    return labels[trigger] ?? splitWords(trigger)
  }
  const [variant, value] = Object.entries(trigger as object)[0] as [string, unknown]
  if (variant === 'Switch') return `Switch ${value} closed`
  if (variant === 'InvertedSwitch') return `Switch ${value} open`
  // dual modes: FlipSwitchTrue_FlopSwitchFalse and friends
  const switches =
    typeof value === 'string'
      ? value
      : Object.values(value as Record<string, string>).join(', ')
  return `${variant.split('_').map(splitWords).join(', ')} (${switches})`
}

const ACRONYMS = new Set(['pwm', 'eos'])

/** `initial_pwm_length` → `Initial PWM length` */
function humanize(key: string): string {
  const words = key.split('_').map((word) => (ACRONYMS.has(word) ? word.toUpperCase() : word))
  const sentence = words.join(' ')
  return sentence.charAt(0).toUpperCase() + sentence.slice(1)
}

/** `PulseHoldCancel` → `Pulse Hold Cancel` */
function splitWords(name: string): string {
  return name.replace(/([a-z])([A-Z])/g, '$1 $2')
}
