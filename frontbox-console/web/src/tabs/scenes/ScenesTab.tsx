import { Splitter } from '@ark-ui/solid/splitter'
import { Tabs } from '@ark-ui/solid/tabs'
import { createEffect, createMemo, createSignal, Show } from 'solid-js'
import { machine } from '../../state/console'
import { renderScene, sceneRender } from '../../state/sceneRender'
import type { RenderedScene } from '../../types/generated/RenderedScene'
import type { Scene } from '../../types/generated/Scene'
import type { SceneSource } from '../../types/generated/SceneSource'
import ConfigPanel from './ConfigPanel'
import ParametersPanel from './ParametersPanel'
import { SceneContext } from './SceneFields'
import { withoutDroppedParams, type Lens } from './sceneModel'
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
  // the scene is held in both forms, each kept in step by the machine as this side never reads or writes RON. `scene`
  // is the last that could be read, so it's `null` until the first reply and stays put while the RON doesn't parse.
  const [ron, setRon] = createSignal(STARTING_SCENE)
  const [scene, setScene] = createSignal<Scene | null>(null)
  const [leds, setLeds] = createSignal(32)
  const [width, setWidth] = createSignal(128)
  const [height, setHeight] = createSignal(32)

  const size = createMemo(() => ({ leds: leds(), width: width(), height: height() }), undefined, {
    equals: (a, b) => a.leds === b.leds && a.width === b.width && a.height === b.height,
  })

  let latest: SceneSource = { Ron: STARTING_SCENE }
  const send = (source: SceneSource) => {
    latest = source
    renderScene(source, size())
  }

  const error = () => {
    const reply = sceneRender()
    return reply && 'error' in reply ? reply.error : null
  }

  // also the first render, and picks back up after a reconnect
  createEffect(() => {
    if (machine.connection === 'connected') send(latest)
  })

  createEffect(() => {
    const reply = sceneRender()
    if (!reply || !('rendered' in reply)) return
    draw(canvas, reply.rendered)
    if (!reply.settled || !reply.source) return
    if ('Ron' in reply.source) setScene(reply.rendered.scene)
    else setRon(reply.rendered.ron)
  })

  const editing: Lens<Scene> = {
    get: () => scene()!,
    set: (next) => {
      const edited = withoutDroppedParams(scene()!, next)
      setScene(edited)
      send({ Definition: edited })
    },
  }

  const isSequence = () => (scene()?.viewport ?? 'Sequence') === 'Sequence'

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
        <div class="scene-source">
          <Show when={scene() !== null}>
            <SceneContext.Provider value={editing}>
              {/* an edit here would replace the RON with the last scene that could be read */}
              <div class="scene-editing" inert={error() !== null}>
                <ParametersPanel />
              </div>
              <Tabs.Root class="scene-tabs" defaultValue="config">
                <Tabs.List>
                  <Tabs.Trigger value="config">Config</Tabs.Trigger>
                  <Tabs.Trigger value="raw">Raw</Tabs.Trigger>
                  <Tabs.Indicator />
                </Tabs.List>
                <Tabs.Content value="config">
                  <Show when={error() !== null}>
                    <p class="scene-note scene-note-warn">This is the last scene that rendered. Fix the error to edit it.</p>
                  </Show>
                  <div class="scene-editing" inert={error() !== null}>
                    <ConfigPanel />
                  </div>
                </Tabs.Content>
                <Tabs.Content value="raw">
                  <textarea
                    aria-label="Scene, as RON"
                    spellcheck={false}
                    value={ron()}
                    onInput={(e) => {
                      setRon(e.currentTarget.value)
                      send({ Ron: e.currentTarget.value })
                    }}
                  />
                </Tabs.Content>
              </Tabs.Root>
            </SceneContext.Provider>
          </Show>
        </div>
      </Splitter.Panel>
      <Splitter.ResizeTrigger id="source:output" aria-label="Resize scene source" />
      <Splitter.Panel id="output" class="scene-output">
        <header>
          <Show
            when={isSequence()}
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
        </header>
        <Show when={error()}>{(text) => <pre class="scene-error">{text()}</pre>}</Show>
        <canvas
          ref={canvas}
          classList={{ stale: error() !== null }}
          style={{ 'min-height': `${isSequence() ? MIN_DRAWN_HEIGHT : 0}px` }}
        />
      </Splitter.Panel>
    </Splitter.Root>
  )
}
