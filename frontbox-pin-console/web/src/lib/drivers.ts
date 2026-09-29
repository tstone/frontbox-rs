import type { DriverMode } from '../types/generated/DriverMode'
import type { DriverTriggerDualMode } from '../types/generated/DriverTriggerDualMode'
import type { DriverTriggerMode } from '../types/generated/DriverTriggerMode'

/**
 * The physical switches that make the hardware fire this driver on its own (e.g. flippers,
 * slingshots). Empty when only software fires it. Tracers only see drivers commanded by software.
 */
export function triggerSwitches(mode: DriverMode | undefined): string[] {
  if (!mode) return []
  if ('FlipperMainDirect' in mode) return [mode.FlipperMainDirect.button_switch]
  if ('FlipperHoldDirect' in mode) return [mode.FlipperHoldDirect.button_switch]
  if ('PulseCancel' in mode) return dualTrigger(mode.PulseCancel.trigger_mode)
  if ('PulseHoldCancel' in mode) return dualTrigger(mode.PulseHoldCancel.trigger_mode)

  const { trigger_mode } = Object.values(mode)[0] as { trigger_mode: DriverTriggerMode }
  return singleTrigger(trigger_mode)
}

function singleTrigger(trigger: DriverTriggerMode): string[] {
  if (typeof trigger === 'string') return []
  if ('Switch' in trigger) return [trigger.Switch]
  return [trigger.InvertedSwitch]
}

function dualTrigger(trigger: DriverTriggerDualMode): string[] {
  if (typeof trigger === 'string') return []
  const [variant, value] = Object.entries(trigger)[0]
  // a virtual flip means software fires it; with a virtual flop, software only arms it
  if (variant.startsWith('VirtualFlip')) return []
  return [typeof value === 'string' ? value : value.flip_switch]
}
