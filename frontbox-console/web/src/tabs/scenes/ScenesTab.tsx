import { SegmentGroup } from '@ark-ui/solid/segment-group'
import { Splitter } from '@ark-ui/solid/splitter'
import { Tabs } from '@ark-ui/solid/tabs'
import { createEffect, createMemo, createSignal, For, Show, untrack } from 'solid-js'
import { cssColor } from '../../lib/leds'
import { machine } from '../../state/console'
import {
  renderClip,
  renderScene,
  sceneClip,
  sceneClipError,
  sceneClipRendering,
  sceneRender,
  type SceneClip,
} from '../../state/sceneRender'
import type { RenderedScene } from '../../types/generated/RenderedScene'
import type { Scene } from '../../types/generated/Scene'
import type { SceneSource } from '../../types/generated/SceneSource'
import PlayfieldView from '../machine/PlayfieldView'
import ConfigPanel from './ConfigPanel'
import ParametersPanel from './ParametersPanel'
import { createPlayback } from './playback'
import { Dropdown, SceneContext, SurfaceContext } from './SceneFields'
import { withoutDroppedParams, withoutDroppedTiming, type Lens, type Vec2 } from './sceneModel'
import Timeline from './Timeline'
import './ScenesTab.css'

const STARTING_SCENE = `(
  version: 5,
  viewport: Sequence,
  layers: [
    (paint: (base: Gradient(stops: [
      (at: 0.0, color: Fixed(255, 0, 0, 255)),
      (at: 1.0, color: Fixed(0, 0, 255, 255)),
    ]))),
  ],
)
`

/** How a scene is previewed: played on the machine's LEDs, or drawn out point by point */
const VIEWS = ['Machine', 'Raw'] as const
type View = (typeof VIEWS)[number]

/** The fewest pixels tall a line is drawn, as one row of points would otherwise be a sliver */
const MIN_DRAWN_HEIGHT = 32

function draw(canvas: HTMLCanvasElement, width: number, height: number, colors: Uint8ClampedArray<ArrayBuffer>) {
  canvas.width = width
  canvas.height = height
  if (width === 0 || height === 0) return
  canvas.getContext('2d')!.putImageData(new ImageData(colors, width, height), 0, 0)
}

function drawClipFrame(canvas: HTMLCanvasElement, { width, height, points }: SceneClip, frame: number) {
  const length = width * height * 4
  draw(canvas, width, height, points.subarray(frame * length, (frame + 1) * length))
}

/** An LED's color in a frame of a clip, as CSS or `null` for off */
function clipLedColor({ ledIndex, leds, frames }: SceneClip, frame: number, name: string) {
  const index = ledIndex.get(name)
  if (index === undefined) return null
  const at = (frame * (leds.length / frames) / 3 + index) * 3
  return cssColor({ r: leds[at], g: leds[at + 1], b: leds[at + 2] })
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
  const [view, setView] = createSignal<View>('Machine')
  const [chosenPlane, setChosenPlane] = createSignal<number | null>(null)
  const openLayer = createSignal<number | null>(0)
  // what's on show, which stays put while the source doesn't parse
  const [drawn, setDrawn] = createSignal<RenderedScene | null>(null)

  const size = createMemo(() => ({ leds: leds(), width: width(), height: height() }), undefined, {
    equals: (a, b) => a.leds === b.leds && a.width === b.width && a.height === b.height,
  })

  const playback = createPlayback(() => scene()?.timing, () => sceneClip() !== null && !sceneClipError())

  // read without tracking while playing, as those moments come from the clip
  const moment = () => (playback.playing() ? untrack(playback.position) : playback.position())

  let latest: SceneSource = { Ron: STARTING_SCENE }
  const send = (source: SceneSource) => {
    latest = source
    const target = { plane: planeIndex(), query: 'Every' } as const
    renderScene(source, size(), target, moment())
    if (playback.playing()) renderClip(source, size(), target)
  }

  /** Which frame of the clip is on show. `null` is when the preview is showing the last render instead. */
  const clipFrame = createMemo(() => {
    const clip = sceneClip()
    const duration = scene()?.timing?.duration
    if (!playback.playing() || !clip || !duration) return null
    return Math.min(Math.round((playback.position() / duration) * (clip.frames - 1)), clip.frames - 1)
  })

  const error = () => {
    const reply = sceneRender()
    return reply && 'error' in reply ? reply.error : null
  }

  // also the first render, and picks back up after a reconnect or once the machine has its hardware
  createEffect(() => {
    machine.hardware
    if (machine.connection === 'connected') send(latest)
  })

  createEffect(() => {
    const reply = sceneRender()
    if (!reply || !('rendered' in reply)) return
    const { width, height, colors } = reply.rendered
    if (!untrack(playback.playing)) draw(canvas, width, height, new Uint8ClampedArray(colors))
    setDrawn(reply.rendered)
    if (!reply.settled || !reply.source) return
    if ('Ron' in reply.source) setScene(reply.rendered.scene)
    else setRon(reply.rendered.ron)
  })

  createEffect(() => {
    const frame = clipFrame()
    if (frame !== null) drawClipFrame(canvas, sceneClip()!, frame)
  })

  createEffect(() => {
    if (sceneClipError()) playback.pause()
  })

  const editing: Lens<Scene> = {
    get: () => scene()!,
    set: (next) => {
      const edited = withoutDroppedTiming(scene()!, withoutDroppedParams(scene()!, next))
      setScene(edited)
      send({ Definition: edited })
    },
  }

  const isSequence = () => (scene()?.viewport ?? 'Sequence') === 'Sequence'

  // a plane with no extent has no size to give a scene
  const planes = () =>
    machine.planes.flatMap((plane, index) => (plane.extent[0] > 0 && plane.extent[1] > 0 ? [{ plane, index }] : []))
  const planeIndex = () => chosenPlane() ?? planes()[0]?.index ?? null
  const plane = () => planes().find(({ index }) => index === planeIndex())?.plane ?? null

  /** With no plane to take a size from, the render is its own device, as a display would be */
  const surface = () => (isSequence() ? null : (plane()?.extent ?? ([Math.max(width(), 1), Math.max(height(), 1)] as Vec2)))

  const number = (set: (value: number) => void) => (e: { currentTarget: HTMLInputElement }) => set(e.currentTarget.valueAsNumber || 0)

  const ledColors = () => drawn()?.led_colors ?? {}

  const ledColor = (name: string) => {
    const frame = clipFrame()
    if (frame !== null) return clipLedColor(sceneClip()!, frame, name)
    const color = ledColors()[name]
    return color ? cssColor(color) : null
  }

  const planeOptions = () => planes().map(({ plane, index }) => ({ value: String(index), label: plane.name }))

  /** Where the open layer is drawn, in scene units. Left out while playing, where it's the scene that's being watched. */
  const outline = (): Vec2[] => {
    const layer = openLayer[0]()
    return (!playback.playing() && layer !== null && drawn()?.outlines[layer]) || []
  }

  const planeOutline = () => {
    const target = plane()
    return target && outline().length > 0 ? { plane: target, points: outline() } : null
  }

  return (
    <SceneContext.Provider value={editing}>
      <SurfaceContext.Provider value={surface}>
        <Splitter.Root
          class="scenes-tab"
          orientation="vertical"
          panels={[
            { id: 'workspace', minSize: 30 },
            { id: 'timeline', minSize: 10 },
          ]}
          defaultSize={[82, 18]}
        >
          <Splitter.Panel id="workspace">
            <Splitter.Root
              panels={[
                { id: 'source', minSize: 15 },
                { id: 'output', minSize: 15 },
              ]}
              defaultSize={[25, 75]}
            >
              <Splitter.Panel id="source">
                <div class="scene-source">
                  <Show when={scene() !== null}>
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
                            <ConfigPanel openLayer={openLayer} />
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
                  </Show>
                </div>
              </Splitter.Panel>
              <Splitter.ResizeTrigger id="source:output" aria-label="Resize scene source" />
              <Splitter.Panel id="output" class="scene-output">
                <header>
                  <Show when={!isSequence() && planes().length > 0}>
                    <Dropdown
                      label="Plane"
                      options={planeOptions()}
                      value={String(planeIndex())}
                      onChange={(value) => setChosenPlane(Number(value))}
                    />
                  </Show>
                  <Show
                    when={view() === 'Raw'}
                    fallback={
                      <>
                        <Show when={!isSequence() && planes().length === 0}>
                          <span class="scene-note">The machine has no planes</span>
                        </Show>
                        <span class="scene-note">Every LED</span>
                        <Show when={!isSequence() && planes().length > 0 && Object.keys(ledColors()).length === 0}>
                          <span class="scene-note scene-note-warn">None are on this plane</span>
                        </Show>
                      </>
                    }
                  >
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
                  </Show>
                  <SegmentGroup.Root
                    class="scene-prop-kind scene-view"
                    aria-label="Preview"
                    value={view()}
                    onValueChange={(details) => setView(details.value as View)}
                  >
                    <SegmentGroup.Indicator />
                    <For each={VIEWS}>
                      {(option) => (
                        <SegmentGroup.Item value={option}>
                          <SegmentGroup.ItemText>{option}</SegmentGroup.ItemText>
                          <SegmentGroup.ItemControl />
                          <SegmentGroup.ItemHiddenInput />
                        </SegmentGroup.Item>
                      )}
                    </For>
                  </SegmentGroup.Root>
                </header>
                <Show when={error()}>{(text) => <pre class="scene-error">{text()}</pre>}</Show>
                {/* both stay up, so the camera and the last render are still there on the way back */}
                <div class="scene-machine" classList={{ stale: error() !== null }} hidden={view() !== 'Machine'}>
                  <PlayfieldView ledColor={ledColor} outline={planeOutline} />
                </div>
                <div class="scene-raw" classList={{ stale: error() !== null }} hidden={view() !== 'Raw'}>
                  <canvas ref={canvas} style={{ 'min-height': `${isSequence() ? MIN_DRAWN_HEIGHT : 0}px` }} />
                  <Show when={drawn()?.area}>
                    {(area) => (
                      <svg viewBox={`${area().x} ${area().y} ${area().width} ${area().height}`} preserveAspectRatio="none" aria-hidden="true">
                        <polygon points={outline().join(' ')} />
                      </svg>
                    )}
                  </Show>
                </div>
              </Splitter.Panel>
            </Splitter.Root>
          </Splitter.Panel>
          <Splitter.ResizeTrigger id="workspace:timeline" aria-label="Resize timeline" />
          <Splitter.Panel id="timeline">
            <Show when={scene() !== null}>
              <div class="scene-editing" inert={error() !== null}>
                <Timeline playback={playback} rendering={playback.playing() && sceneClipRendering()} error={sceneClipError()} />
              </div>
            </Show>
          </Splitter.Panel>
        </Splitter.Root>
      </SurfaceContext.Provider>
    </SceneContext.Provider>
  )
}
