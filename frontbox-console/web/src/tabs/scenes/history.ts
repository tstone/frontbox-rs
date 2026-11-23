import { createSignal } from 'solid-js'

/** How many milliseconds after one change another is taken as part of it, as a drag or typing makes many */
const MERGE_TIME = 500

/** The most changes that can be undone */
const MOST_UNDOS = 200

/**
 * What a scene was before each change, to undo back to, and what it was before each undo, to redo. Changes that come
 * quickly one after another, such as each step of a drag, are undone together.
 */
export function createHistory<T>() {
  const [undos, setUndos] = createSignal<T[]>([])
  const [redos, setRedos] = createSignal<T[]>([])
  let lastChange = 0

  return {
    canUndo: () => undos().length > 0,
    canRedo: () => redos().length > 0,

    /** Notes what it was before a change, which ends what there was to redo */
    record(before: T) {
      const now = performance.now()
      if (now - lastChange > MERGE_TIME || undos().length === 0) setUndos([...undos().slice(1 - MOST_UNDOS), before])
      setRedos([])
      lastChange = now
    },

    /** What to go back to from `current`, if there's anything to undo */
    undo(current: T): T | undefined {
      const before = undos().at(-1)
      if (before === undefined) return undefined
      setUndos(undos().slice(0, -1))
      setRedos([...redos(), current])
      lastChange = 0
      return before
    },

    /** What to go forward to from `current`, if there's anything to redo */
    redo(current: T): T | undefined {
      const after = redos().at(-1)
      if (after === undefined) return undefined
      setRedos(redos().slice(0, -1))
      setUndos([...undos(), current])
      lastChange = 0
      return after
    },

    clear() {
      setUndos([])
      setRedos([])
      lastChange = 0
    },
  }
}
