import type { ConfigDomainValue } from '../types/generated/ConfigDomainValue'
import type { ConfigView } from '../types/generated/ConfigView'
import { sendToMachine } from './console'

/**
 * Asks the machine to set a config, as the operator menu would. Nothing changes here until the machine reports the
 * new value back, so a rejected value never shows.
 */
export function setConfig(config: ConfigView, value: ConfigDomainValue) {
  sendToMachine({ type: 'SetConfig', key: config.key, value })
}

export function resetConfig(config: ConfigView) {
  setConfig(config, config.default)
}

/** Set an int config, clamped to its range */
export function setConfigInt(config: ConfigView, value: number) {
  if (typeof config.domain !== 'object' || !('Int' in config.domain)) return
  const { min, max } = config.domain.Int
  setConfig(config, { Int: Math.min(max, Math.max(min, Math.round(value))) })
}

/** Step an int config by its domain's step, clamping at the ends like the menu */
export function stepConfig(config: ConfigView, direction: 1 | -1) {
  if (typeof config.domain !== 'object' || !('Int' in config.domain) || !('Int' in config.value)) return
  setConfigInt(config, config.value.Int + direction * config.domain.Int.step)
}
