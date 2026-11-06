import { createEffect, createSignal, onCleanup, type Accessor } from 'solid-js'
import type { Timing } from '../../types/generated/Timing'

export type Playback = ReturnType<typeof createPlayback>

/**
 * Where the preview is in a scene, and moving that along. Time only moves once `ready`, as there's nothing to show until
 * the frames have come back from the machine.
 */
export function createPlayback(timing: Accessor<Timing | undefined>, ready: Accessor<boolean>) {
  const [position, setPosition] = createSignal(0)
  const [playing, setPlaying] = createSignal(false)
  // across the cycles, where `position` is within one
  let elapsed = 0

  const advance = (seconds: number) => {
    const { duration, cycle } = timing()!
    elapsed += seconds
    const cycles = cycle === 'Once' ? 1 : cycle === 'Forever' ? Infinity : cycle.Times
    if (elapsed >= duration * cycles) {
      setPosition(duration)
      setPlaying(false)
    } else {
      setPosition(elapsed % duration)
    }
  }

  createEffect(() => {
    if (!playing() || !ready()) return
    let last = performance.now()
    // a tick can be what ends this, by reaching the end of the scene
    let ended = false
    const tick = (now: number) => {
      advance((now - last) / 1000)
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
    playing,
    /** From where it is, or from the start if it's at the end */
    play: () => {
      if (position() >= timing()!.duration) seek(0)
      else elapsed = position()
      setPlaying(true)
    },
    pause: () => setPlaying(false),
    stop,
    /** Pauses, at a place in a cycle */
    scrub: (seconds: number) => {
      setPlaying(false)
      seek(seconds)
    },
  }
}
