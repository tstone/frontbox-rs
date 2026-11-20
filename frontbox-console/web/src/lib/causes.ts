import { systemName } from '../state/console'
import type { ActivityCause } from '../types/generated/ActivityCause'
import type { DespawnCause } from '../types/generated/DespawnCause'
import type { StepEnding } from '../types/generated/StepEnding'
import { shortName } from './format'

/** Why a system's activity changed, as a phrase: "by its parent Game" */
export function activityCause(cause: ActivityCause): string {
  if ('Parent' in cause) return `by its parent ${systemName(cause.Parent.id)}`
  if ('Ancestor' in cause) return `with ${systemName(cause.Ancestor.id)}`
  const during = cause.IsActive.during
  return during === null ? 'by its own is_active' : `by its own is_active, during ${shortName(during)}`
}

/** Why a system was despawned, as a phrase: "by its parent Game" */
export function despawnCause(cause: DespawnCause): string {
  if (cause === 'Itself') return 'by itself'
  if (cause === 'Panicked') return 'after a panic'
  if (cause === 'Replaced') return 'replaced by another system'
  if ('Parent' in cause) return `by its parent ${systemName(cause.Parent.id)}`
  return `with ${systemName(cause.Ancestor.id)}`
}

/** How a routine step ended, as a word: "completed" */
export function stepEnding(ending: StepEnding): string {
  return {
    Completed: 'completed',
    Halted: 'halted the routine',
    Stopped: 'stopped mid-step',
    Cancelled: 'cancelled',
    Abandoned: 'abandoned',
  }[ending]
}
