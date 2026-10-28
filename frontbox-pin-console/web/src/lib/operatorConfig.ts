import type { ConfigDomainValue } from '../types/generated/ConfigDomainValue'
import type { ConfigView } from '../types/generated/ConfigView'
import type { DriverMode } from '../types/generated/DriverMode'
import { shortName } from './format'

/** What kind of control and units a config gets, from its Rust type. Power is a choice of eighths. */
export type ConfigKind = 'duration' | 'int' | 'bool' | 'choice'

export function configKind(config: ConfigView): ConfigKind {
  if (config.domain === 'Bool') return 'bool'
  if ('Choice' in config.domain) return 'choice'
  if (shortName(config.type_name) === 'Duration') return 'duration'
  return 'int'
}

export function sameValue(a: ConfigDomainValue, b: ConfigDomainValue): boolean {
  return JSON.stringify(a) === JSON.stringify(b)
}

export const isDefault = (config: ConfigView) => sameValue(config.value, config.default)

/** A value as the console shows it: durations in ms rather than the menu's wording */
export function formatConfigValue(config: ConfigView, value: ConfigDomainValue): string {
  if ('Bool' in value) return value.Bool ? 'On' : 'Off'
  if ('Choice' in value) {
    const names = typeof config.domain === 'object' && 'Choice' in config.domain ? config.domain.Choice : []
    return names[value.Choice] ?? `#${value.Choice}`
  }
  return configKind(config) === 'duration' ? `${value.Int} ms` : String(value.Int)
}

/** Keys of the operator configs a driver mode's values are backed by */
export function modeConfigKeys(mode: DriverMode): string[] {
  const body = Object.values(mode)[0] as Record<string, unknown>
  return Object.values(body).flatMap((value) =>
    typeof value === 'object' && value !== null && 'Config' in value ? [(value.Config as { key: string }).key] : [],
  )
}
