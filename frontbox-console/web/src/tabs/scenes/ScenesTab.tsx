import { createEffect, createSignal, Show } from 'solid-js'
import { renderScene, sceneRender } from '../../state/sceneRender'
import type { RenderedScene } from '../../types/generated/RenderedScene'
import './ScenesTab.css'

const STARTING_SCENE = `(
  version: 1,
  layers: [
    (paint: (base: Gradient(stops: [
      (at: 0.0, color: Fixed(255, 0, 0, 255)),
      (at: 1.0, color: Fixed(0, 0, 255, 255)),
    ]))),
  ],
)
`

type Kind = 'Line' | 'Grid'

/** The fewest pixels tall a line is drawn, as one row of points would otherwise be a sliver */
const MIN_DRAWN_HEIGHT = 32

function draw(canvas: HTMLCanvasElement, { width, height, colors }: RenderedScene) {
  canvas.width = width
  canvas.height = height
  if (width === 0 || height === 0) return
  canvas.getContext('2d')!.putImageData(new ImageData(new Uint8ClampedArray(colors), width, height), 0, 0)
}

export default function ScenesTab() {
  let canvas!: HTMLCanvasElement
  const [source, setSource] = createSignal(STARTING_SCENE)
  const [kind, setKind] = createSignal<Kind>('Line')
  const [count, setCount] = createSignal(32)
  const [width, setWidth] = createSignal(128)
  const [height, setHeight] = createSignal(32)

  const render = () => {
    const target = kind() === 'Line' ? { type: 'Line' as const, count: count() } : { type: 'Grid' as const, width: width(), height: height() }
    renderScene(source(), target)
  }

  const rendered = () => {
    const reply = sceneRender()
    return reply && 'rendered' in reply ? reply.rendered : null
  }
  const error = () => {
    const reply = sceneRender()
    return reply && 'error' in reply ? reply.error : null
  }

  createEffect(() => {
    const scene = rendered()
    if (scene) draw(canvas, scene)
  })

  const number = (set: (value: number) => void) => (e: { currentTarget: HTMLInputElement }) => set(e.currentTarget.valueAsNumber || 0)

  return (
    <div class="scenes-tab pane">
      <textarea
        aria-label="Scene, as RON"
        spellcheck={false}
        value={source()}
        onInput={(e) => setSource(e.currentTarget.value)}
        onKeyDown={(e) => {
          if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) render()
        }}
      />
      <div class="scene-output">
        <header>
          <select aria-label="Render to" value={kind()} onChange={(e) => setKind(e.currentTarget.value as Kind)}>
            <option value="Line">LEDs</option>
            <option value="Grid">Pixels</option>
          </select>
          <Show
            when={kind() === 'Line'}
            fallback={
              <>
                <input type="number" min="1" aria-label="Width" value={width()} onInput={number(setWidth)} />
                <span>×</span>
                <input type="number" min="1" aria-label="Height" value={height()} onInput={number(setHeight)} />
              </>
            }
          >
            <input type="number" min="1" aria-label="Number of LEDs" value={count()} onInput={number(setCount)} />
          </Show>
          <button type="button" onClick={render}>
            Render
          </button>
        </header>
        <Show when={error()}>{(text) => <pre class="scene-error">{text()}</pre>}</Show>
        <canvas
          ref={canvas}
          classList={{ stale: error() !== null }}
          style={{ 'min-height': `${rendered()?.height === 1 ? MIN_DRAWN_HEIGHT : 0}px` }}
        />
      </div>
    </div>
  )
}
