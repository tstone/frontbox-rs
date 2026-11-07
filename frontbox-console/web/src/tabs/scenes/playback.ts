import { batch, createEffect, createSignal, onCleanup, type Accessor } from 'solid-js'
import type { Timing } from '../../types/generated/Timing'
import { cycleCount } from './sceneModel'

export type Playback = ReturnType<typeof createPlayback>

/**
 * Where the preview is in a scene, and moving that along. Time only moves once `ready`, as there's nothing to show until
 * the frames have come back from the machine.
 */
export function createPlayback(timing: Accessor<Timing | undefined>, ready: Accessor<boolean>) {
  const [position, setPosition] = createSignal(0)
  const [playing, setPlaying] = createSignal(false)
  const [cycle, setCycle] = createSignal<number | null>(null)
  // across the cycles, where `position` is within one
  let elapsed = 0

  const advance = (seconds: number) => {
    const { duration } = timing()!
    elapsed += seconds
    if (elapsed >= duration * cycleCount(timing()!)) {
      setPosition(duration)
      setPlaying(false)
    } else {
      batch(() => {
        setCycle(Math.floor(elapsed / duration))
        setPosition(elapsed % duration)
      })
    }
  }

  createEffect(() => {
    if (!playing() || !ready()) return
    let last = performance.now()
    // a tick can be what ends this, by reaching the end of the scene
    let ended = false
    const tick = (now: number) => {
      // the first can be stamped from before this started
      advance(Math.max(now - last, 0) / 1000)
      last = now
      if (!ended) frame = requestAnimationFrame(tick)
    }
    let frame = requestAnimationFrame(tick)
    onCleanup(() => {
      ended = true
      cancelAnimationFrame(frame)
    })
  })

  const seek = (seconds: number) => {
    elapsed = seconds
    setCycle(null)
    setPosition(seconds)
  }

  const stop = () => {
    setPlaying(false)
    seek(0)
  }

  createEffect(() => {
    if (!timing()) stop()
  })

  return {
    /** Seconds into a cycle of the scene */
    position,
    /** Which cycle a play of the scene has reached, which pausing keeps. `null` when one isn't under way. */
    cycle,
    playing,
    /** From where it is, or from the start if it's at the end */
    play: () => {
      if (position() >= timing()!.duration) seek(0)
      setCycle(Math.floor(elapsed / timing()!.duration))
      setPlaying(true)
    },
    pause: () => setPlaying(false),
    stop,
    /** Stays at the same moment of a scene whose times have all been scaled by `by` */
    rescale: (by: number) => {
      elapsed *= by
      setPosition(position() * by)
    },
    /** Pauses, at a place in a cycle */
    scrub: (seconds: number) => {
      setPlaying(false)
      seek(seconds)
    },
  }
}
