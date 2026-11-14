import { type Accessor, createSignal, type Setter } from 'solid-js'

/** The first byte of a binary message. Mirrors `BINARY_*` in `protocol.rs`. */
export const BINARY_CLIP = 0
export const BINARY_DISPLAY_FRAME = 1

type Frame = { get: Accessor<Uint8ClampedArray<ArrayBuffer> | null>; set: Setter<Uint8ClampedArray<ArrayBuffer> | null> }

// by a display's place in `machine.displays`, made when first read or received
const frames = new Map<number, Frame>()

function frame(display: number): Frame {
  let held = frames.get(display)
  if (!held) {
    const [get, set] = createSignal<Uint8ClampedArray<ArrayBuffer> | null>(null, { equals: false })
    held = { get, set }
    frames.set(display, held)
  }
  return held
}

/** A display's newest frame as red, green, blue, and alpha for each pixel, row by row from the top left. `null` until one arrives. */
export const displayFrame = (display: number) => frame(display).get()

/** Called by the console for each display frame: a byte for the display, then red, green, and blue for each pixel */
export function receiveDisplayFrame(message: ArrayBuffer) {
  const bytes = new Uint8Array(message)
  const rgb = bytes.subarray(2)
  const pixels = rgb.length / 3
  const rgba = new Uint8ClampedArray(pixels * 4)
  for (let i = 0; i < pixels; i++) {
    rgba[i * 4] = rgb[i * 3]
    rgba[i * 4 + 1] = rgb[i * 3 + 1]
    rgba[i * 4 + 2] = rgb[i * 3 + 2]
    rgba[i * 4 + 3] = 255
  }
  frame(bytes[1]).set(rgba)
}
