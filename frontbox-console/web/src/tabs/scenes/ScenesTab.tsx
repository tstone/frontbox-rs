import { Splitter } from '@ark-ui/solid/splitter'
import { Tabs } from '@ark-ui/solid/tabs'
import { createEffect, createSignal, Show } from 'solid-js'
import { renderScene, sceneRender } from '../../state/sceneRender'
import type { RenderedScene } from '../../types/generated/RenderedScene'
import type { SceneViewport } from '../../types/generated/SceneViewport'
import './ScenesTab.css'

const STARTING_SCENE = `(
  version: 1,
  viewport: Sequence,
  layers: [
    (paint: (base: Gradient(stops: [
      (at: 0.0, color: Fixed(255, 0, 0, 255)),
      (at: 1.0, color: Fixed(0, 0, 255, 255)),
    ]))),
  ],
)
`

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
  // the viewport of the last scene to render, as the source is opaque here
  const [viewport, setViewport] = createSignal<SceneViewport>('Sequence')
  const [leds, setLeds] = createSignal(32)
  const [width, setWidth] = createSignal(128)
  const [height, setHeight] = createSignal(32)

  const render = () => renderScene(source(), { leds: leds(), width: width(), height: height() })

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
    if (!scene) return
    setViewport(scene.viewport)
    draw(canvas, scene)
  })

  const number = (set: (value: number) => void) => (e: { currentTarget: HTMLInputElement }) => set(e.currentTarget.valueAsNumber || 0)

  return (
    <Splitter.Root
      class="scenes-tab"
      panels={[
        { id: 'source', minSize: 15 },
        { id: 'output', minSize: 15 },
      ]}
      defaultSize={[25, 75]}
    >
      <Splitter.Panel id="source">
        <Tabs.Root class="scene-source" defaultValue="raw">
          <Tabs.List>
            <Tabs.Trigger value="config">Config</Tabs.Trigger>
            <Tabs.Trigger value="raw">Raw</Tabs.Trigger>
            <Tabs.Indicator />
          </Tabs.List>
          <Tabs.Content value="config" />
          <Tabs.Content value="raw">
            <textarea
              aria-label="Scene, as RON"
              spellcheck={false}
              value={source()}
              onInput={(e) => setSource(e.currentTarget.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) render()
              }}
            />
          </Tabs.Content>
        </Tabs.Root>
      </Splitter.Panel>
      <Splitter.ResizeTrigger id="source:output" aria-label="Resize scene source" />
      <Splitter.Panel id="output" class="scene-output">
        <header>
          <Show
            when={viewport() === 'Sequence'}
            fallback={
              <>
                <input type="number" min="1" aria-label="Width" value={width()} onInput={number(setWidth)} />
                <span>×</span>
                <input type="number" min="1" aria-label="Height" value={height()} onInput={number(setHeight)} />
              </>
            }
          >
            <input type="number" min="1" aria-label="Number of LEDs" value={leds()} onInput={number(setLeds)} />
          </Show>
          <button type="button" onClick={render}>
            Render
          </button>
        </header>
        <Show when={error()}>{(text) => <pre class="scene-error">{text()}</pre>}</Show>
        <canvas
          ref={canvas}
          classList={{ stale: error() !== null }}
          style={{ 'min-height': `${viewport() === 'Sequence' ? MIN_DRAWN_HEIGHT : 0}px` }}
        />
      </Splitter.Panel>
    </Splitter.Root>
  )
}
