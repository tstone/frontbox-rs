import { createSignal } from 'solid-js'
import type { MetricTotals } from '../types/generated/MetricTotals'
import { sendToMachine } from './console'

const [totals, setTotals] = createSignal<MetricTotals | null>(null)

/** The last totals the machine replied with; `null` until the first reply */
export const metricTotals = totals

/** Asks for every counter's total. The reply is read from the metrics database, so it trails by up to one flush. */
export function queryMetrics() {
  sendToMachine({ type: 'QueryMetrics' })
}

/** Called by the console for the machine's reply */
export function receiveMetricTotals(reply: MetricTotals) {
  setTotals(reply)
}
