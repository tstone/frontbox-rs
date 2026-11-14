import { createEffect } from 'solid-js'
import { displayFrame } from '../state/displays'
import type { DisplayView } from '../types/generated/DisplayView'

/** How many screen pixels wide each of a display's pixels is drawn */
const SCALE = 3

/** A display's frames, drawn flat with each pixel a crisp square */
export default function DisplayScreen(props: { display: DisplayView; index: number }) {
  let canvas!: HTMLCanvasElement

  createEffect(() => {
    const frame = displayFrame(props.index)
    const { width, height } = props.display
    const context = canvas.getContext('2d')
    if (!context) return
    if (frame && frame.length === width * height * 4) {
      context.putImageData(new ImageData(frame, width, height), 0, 0)
    } else {
      context.fillStyle = '#000'
      context.fillRect(0, 0, width, height)
    }
  })

  return (
    <figure class="display-screen">
      <canvas ref={canvas} width={props.display.width} height={props.display.height} style={{ width: `${props.display.width * SCALE}px` }} />
      <figcaption>{props.display.name}</figcaption>
    </figure>
  )
}
