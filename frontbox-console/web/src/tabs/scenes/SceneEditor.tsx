import { Checkbox } from '@ark-ui/solid/checkbox'
import { SegmentGroup } from '@ark-ui/solid/segment-group'
import { Splitter } from '@ark-ui/solid/splitter'
import { batch, createEffect, createMemo, createSignal, For, on, onCleanup, onMount, Show, untrack } from 'solid-js'
import * as icons from '../../components/icons'
import { cssColor } from '../../lib/leds'
import { machine } from '../../state/console'
import { addContentScene, contentScenes, deleteContentScene, saveContentScene } from '../../state/contentScenes'
import { playContentSound } from '../../state/contentSounds'
import {
  forgetSceneRender,
  renderClips,
  renderScene,
  sceneClip,
  sceneClipError,
  sceneClipRendering,
  sceneRender,
  type ClipCanvasFrames,
  type SceneClip,
} from '../../state/sceneRender'
import type { ClipSound } from '../../types/generated/ClipSound'
import type { Prop } from '../../types/generated/Prop'
import type { ReferenceScene } from '../../types/generated/ReferenceScene'
import type { RenderedScene } from '../../types/generated/RenderedScene'
import type { SceneSource } from '../../types/generated/SceneSource'
import PlayfieldView, { type PreviewSpace } from '../machine/PlayfieldView'
import ConfigPanel from './ConfigPanel'
import ConfirmDialog from './ConfirmDialog'
import { LedsNote } from './LayerPlace'
import { createPlayback } from './playback'
import { Dropdown, NumberBox, LayerSurfacesContext, NodePickingContext, RegionVisibilityContext, RevealContext, SceneContext, ShapePosesContext, type NodePicking, type RegionVisibility, type ShapePoses } from './SceneFields'
import { openTransform } from './configNodes'
import { changesEachCycle, cycleCount, editedScene, layerPlace, layerPlane, placeDisplay, sceneSource, selectedShape, withoutDroppedVariables, withoutDroppedTiming, type Lens, type Scene, type SceneLayer, type SceneTrack, type SceneValue, type Selection, type Vec2 } from './sceneModel'
import { SceneNameButton } from './SceneNameDialog'
import GraphOverlay from './GraphOverlay'
import ShapeHandles, { layerUnder } from './ShapeHandles'
import Timeline from './Timeline'

/**
 * What the preview shows: the machine, with the LEDs and the displays that have a place on it, the strip of the
 * `Sequence` layers, which is where their handles are, or one display on its own
 */
type View = 'Machine' | 'Strip' | `Display:${string}`

/** The display a view is of, which is `null` for the machine and the strip */
const viewDisplay = (view: View) => (view.startsWith('Display:') ? view.slice('Display:'.length) : null)

/** The fewest pixels tall a line is drawn, as one row of points would otherwise be a sliver */
const MIN_DRAWN_HEIGHT = 32

/** Whether points snap to a grid as they're dragged, and its size in the units of the shape being edited */
type Snap = { on: boolean; size: number }

const SNAP_KEY = 'frontbox-console.scene-snap'
const DEFAULT_SNAP: Snap = { on: false, size: 1 }
function readSnap(): Snap {
  try {
    const stored = { ...DEFAULT_SNAP, ...JSON.parse(localStorage.getItem(SNAP_KEY) ?? '{}') }
    return stored.size > 0 ? stored : DEFAULT_SNAP
  } catch {
    return DEFAULT_SNAP
  }
}
function writeSnap(snap: Snap) {
  try {
    localStorage.setItem(SNAP_KEY, JSON.stringify(snap))
  } catch {
    // storage can be unavailable (private windows); snapping still works for this session
  }
}

/** Where a reference scene is drawn, over the scene being edited or under it */
const REFERENCE_PLACES = ['Above', 'Below'] as const

/** The option for drawing the scene alone */
const NO_REFERENCE = ''

/** The scene each scene is shown with, by the name of the one being edited */
const REFERENCE_KEY = 'frontbox-console.scene-references'
function readReferences(): Record<string, ReferenceScene> {
  try {
    return JSON.parse(localStorage.getItem(REFERENCE_KEY) ?? '{}')
  } catch {
    return {}
  }
}
function writeReference(scene: string, reference: ReferenceScene | null) {
  try {
    const { [scene]: _, ...others } = readReferences()
    localStorage.setItem(REFERENCE_KEY, JSON.stringify(reference ? { ...others, [scene]: reference } : others))
  } catch {
    // storage can be unavailable (private windows); the reference is still shown for this session
  }
}

function draw(canvas: HTMLCanvasElement, width: number, height: number, colors: Uint8ClampedArray<ArrayBuffer>) {
  canvas.width = width
  canvas.height = height
  if (width === 0 || height === 0) return
  canvas.getContext('2d')!.putImageData(new ImageData(colors, width, height), 0, 0)
}

/** One frame of one of a clip's canvases */
function clipFrameColors({ width, height, points }: ClipCanvasFrames, frame: number) {
  const length = width * height * 4
  return points.subarray(frame * length, (frame + 1) * length)
}

/** An LED's color in a frame of a clip, as CSS or `null` for off */
function clipLedColor({ ledIndex, leds, frames }: SceneClip, frame: number, name: string) {
  const index = ledIndex.get(name)
  if (index === undefined) return null
  const at = (frame * (leds.length / frames) / 3 + index) * 3
  return cssColor({ r: leds[at], g: leds[at + 1], b: leds[at + 2] })
}

/**
 * Edits a scene from the content folder. A scene is opened by mounting one of these, with `source` being its file. It hands
 * `onLeaving` what says whether it can be left now, which asks about unsaved changes when it can't.
 */
export default function SceneEditor(props: {
  name: string
  source: string
  onClose: () => void
  onRename: (name: string) => void
  onLeaving: (mayLeave: () => boolean) => void
}) {
  let canvas!: HTMLCanvasElement
  let root!: HTMLDivElement
  // the scene is held in both forms, each kept in step by the machine as this side never reads or writes RON. `scene`
  // is the last that could be read, so it's `null` until the first reply and stays put while the RON doesn't parse.
  const [ron, setRon] = createSignal(props.source)
  const [scene, setScene] = createSignal<Scene | null>(null)
  // the file being edited, which a save as moves on to
  const [name, setName] = createSignal(props.name)
  // the file as it's last known to be on the machine
  const [saved, setSaved] = createSignal(props.source)
  const [fileWork, setFileWork] = createSignal<'saving' | 'deleting' | null>(null)
  const [fileError, setFileError] = createSignal<string | null>(null)
  const [confirming, setConfirming] = createSignal<'close' | 'reset' | 'delete' | null>(null)
  const [leds, setLeds] = createSignal(32)
  const [width, setWidth] = createSignal(128)
  const [height, setHeight] = createSignal(32)
  const [chosenView, setView] = createSignal<View>('Machine')
  // another scene drawn with this one, to see how the two look together. Only this one's layers can be selected.
  const [reference, setReferenceState] = createSignal<ReferenceScene | null>(readReferences()[props.name] ?? null)
  const setReference = (next: ReferenceScene | null) => {
    setReferenceState(next)
    writeReference(name(), next)
  }
  const [snap, setSnapState] = createSignal(readSnap())
  const setSnap = (change: Partial<Snap>) => {
    setSnapState({ ...snap(), ...change })
    writeSnap(snap())
  }
  const selection = createSignal<Selection | null>({ layer: 0, mask: false })
  // the node selected in the config tree, by id, which says what the handles are for
  const [treeNode, setTreeNode] = createSignal<string | null>(null)
  // the source panel shows the scene as a tree, or as RON
  const [showingCode, setShowingCode] = createSignal(false)
  // where the config tree puts its own buttons, at the end of its section's header
  const [treeTools, setTreeTools] = createSignal<HTMLElement>()
  const revealing = createSignal<Prop<SceneValue> | null>(null)
  // what's on show, which stays put while the source doesn't parse
  const [drawn, setDrawn] = createSignal<RenderedScene | null>(null)

  const size = createMemo(() => ({ leds: leds(), width: width(), height: height() }), undefined, {
    equals: (a, b) => a.leds === b.leds && a.width === b.width && a.height === b.height,
  })

  const playback = createPlayback(() => scene()?.timing, () => sceneClip(clipCycle()) !== null && !sceneClipError())

  // read without tracking while playing, as those moments come from the clip
  const moment = () => (playback.playing() ? untrack(playback.position) : playback.position())
  const momentCycle = () => (playback.playing() ? untrack(playback.cycle) : playback.cycle())

  const changesEach = createMemo(() => scene() !== null && changesEachCycle(scene()!))

  /** Which clip is playing. A scene that's the same every cycle has the one, on a loop. */
  const clipCycle = () => (changesEach() ? (playback.cycle() ?? 0) : 0)

  /** The clips to have ready: the one playing, and the one after it so that it's there when this one ends */
  const clipCycles = createMemo(() => {
    const cycle = clipCycle()
    const timing = scene()?.timing
    return changesEach() && timing && cycle + 1 < cycleCount(timing) ? [cycle, cycle + 1] : [cycle]
  })

  let latest: SceneSource = { Ron: props.source }
  const send = (source: SceneSource) => {
    latest = source
    renderScene(source, size(), moment(), momentCycle(), reference())
    if (playback.playing()) renderClips(source, size(), reference(), untrack(clipCycles))
  }

  // moving on to a cycle wants the one after it, where `send` is for changes to the scene
  createEffect(() => {
    const cycles = clipCycles()
    untrack(() => {
      if (playback.playing()) renderClips(latest, size(), reference(), cycles)
    })
  })

  createEffect(on(reference, () => send(latest), { defer: true }))

  /**
   * What's on show from the clips, which stays put if the next cycle is late back from the machine. `null` is when the
   * preview is showing the last render instead.
   */
  const clipFrame = createMemo<{ clip: SceneClip; frame: number } | null>(
    (held) => {
      const duration = scene()?.timing?.duration
      if (!playback.playing() || !duration) return null
      const clip = sceneClip(clipCycle())
      if (!clip) return held
      return { clip, frame: Math.min(Math.round((playback.position() / duration) * (clip.frames - 1)), clip.frames - 1) }
    },
    null,
    { equals: (a, b) => a?.clip === b?.clip && a?.frame === b?.frame },
  )

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
    setDrawn(reply.rendered)
    if (!reply.settled) return
    if ('Ron' in reply.source) setScene(editedScene(reply.rendered.scene))
    else setRon(reply.rendered.ron)
  })

  createEffect(() => {
    const shown = shownCanvas()
    const [width, height] = canvasSize()
    if (untrack(playback.playing)) return
    if (shown) draw(canvas, shown.width, shown.height, new Uint8ClampedArray(shown.colors))
    else draw(canvas, width, height, new Uint8ClampedArray(width * height * 4))
  })

  createEffect(() => {
    const shown = clipFrame()
    if (!shown) return
    const [width, height] = canvasSize()
    const on = shown.clip.canvases.find((clipCanvas) => clipCanvas.display === viewDisplay(view()))
    if (on) draw(canvas, on.width, on.height, clipFrameColors(on, shown.frame))
    else draw(canvas, width, height, new Uint8ClampedArray(width * height * 4))
  })

  createEffect(() => {
    if (sceneClipError()) playback.pause()
  })

  // the preview's sounds play on the machine as it passes their starts, as they would in the game, so playing from partway
  // through a cycle leaves out the ones before. `heard` is how far that's got.
  let heard: { cycle: number; position: number } | null = null
  createEffect(() => {
    if (!playback.playing()) {
      heard = null
      return
    }
    const clip = sceneClip(clipCycle())
    const [cycle, position] = [playback.cycle() ?? 0, playback.position()]
    if (!clip) return
    const from = heard
    const passed = ({ at }: ClipSound) => {
      if (from === null) return at === position
      return (from.cycle !== cycle || at > from.position) && at <= position
    }
    clip.sounds.filter(passed).forEach(({ sound, volume, skip, length }) => playContentSound(sound, volume, skip, length))
    heard = { cycle, position }
  })

  const editing: Lens<Scene> = {
    get: () => scene()!,
    set: (next) => {
      const edited = withoutDroppedTiming(scene()!, withoutDroppedVariables(scene()!, next))
      setScene(edited)
      send(sceneSource(edited))
    },
  }

  const unsaved = () => ron() !== saved()

  const workOnFile = async (work: 'saving' | 'deleting', action: () => Promise<void>) => {
    if (fileWork()) return
    setFileWork(work)
    setFileError(null)
    try {
      await action()
    } catch (err) {
      setFileError(err instanceof Error ? err.message : String(err))
    } finally {
      setFileWork(null)
    }
  }

  /** Saves `latest` rather than `ron`, which is a reply behind an edit made a moment ago */
  const save = () =>
    workOnFile('saving', async () => {
      setSaved(await saveContentScene(name(), latest))
    })

  /** Goes on editing the new file. A name that's taken is refused, so this never writes over another scene. */
  const saveAs = async (chosen: string) => {
    const written = await addContentScene(chosen, latest)
    batch(() => {
      setSaved(written)
      setName(chosen)
      writeReference(chosen, reference())
      props.onRename(chosen)
      setFileError(null)
    })
  }

  /** Goes back to the file. What's selected may not be there anymore. */
  const reset = () => {
    batch(() => {
      setConfirming(null)
      selection[1]({ layer: 0, mask: false })
      setRon(saved())
    })
    send({ Ron: saved() })
  }

  const remove = () => {
    setConfirming(null)
    workOnFile('deleting', async () => {
      await deleteContentScene(name())
      leave()
    })
  }

  /** Leaves at once, unsaved changes or not */
  const leave = () => props.onClose()

  const close = () => (unsaved() ? setConfirming('close') : leave())

  // going back to the list, by Back or by the URL, asks first when there's something to lose
  props.onLeaving(() => {
    if (!unsaved()) return true
    setConfirming('close')
    return false
  })

  onMount(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      // the tab stays mounted while another is showing
      if (!root.checkVisibility()) return
      if (e.key === 's' && (e.ctrlKey || e.metaKey)) {
        e.preventDefault()
        save()
      } else if (e.key === ' ' && !e.ctrlKey && !e.metaKey && !e.altKey && !typingIn(e.target)) {
        // taken from a focused button too, which would otherwise be pressed
        e.preventDefault()
        if (e.repeat) return
        if (playback.playing()) playback.stop()
        else if (scene()?.timing) playback.play()
      }
    }
    const onBeforeUnload = (e: BeforeUnloadEvent) => {
      if (unsaved()) e.preventDefault()
    }
    window.addEventListener('keydown', onKeyDown)
    window.addEventListener('beforeunload', onBeforeUnload)
    onCleanup(() => {
      window.removeEventListener('keydown', onKeyDown)
      window.removeEventListener('beforeunload', onBeforeUnload)
    })
  })

  // the next scene to open shouldn't start out showing this one
  onCleanup(forgetSceneRender)

  // a file that won't read has nothing to show in the config, so it opens where it can be fixed
  createEffect(() => {
    if (scene() === null && error() !== null) setShowingCode(true)
  })

  /** The layer the preview is showing the surface of: the open one, or with none open, the last that was */
  const shownIndex = createMemo<number>((last) => selection[0]()?.layer ?? last, 0)
  const shownLayer = () => scene()?.layers[shownIndex()]
  const isSequence = () => shownLayer() !== undefined && shownLayer()!.display === undefined && shownLayer()!.surface === 'Sequence'

  /** The machine's displays, then any others the scene or the scene shown with it draws on */
  const displays = () => {
    const named = scene()?.layers.flatMap((layer) => (layer.display === undefined ? [] : [layer.display])) ?? []
    const drawnOn = drawn()?.canvases.flatMap((on) => (on.display === null ? [] : [on.display])) ?? []
    return [...new Set([...machine.displays.map((display) => display.name), ...named, ...drawnOn])]
  }
  const hasStrip = () => scene()?.layers.some((layer) => layer.display === undefined && layer.surface === 'Sequence') ?? false
  const views = (): { value: View; label: string }[] => [
    { value: 'Machine', label: 'Machine' },
    ...(hasStrip() ? [{ value: 'Strip' as const, label: 'Strip' }] : []),
    ...displays().map((display) => ({ value: `Display:${display}` as const, label: display })),
  ]
  /** What's on show, which is the machine once what was chosen isn't there to show */
  const view = (): View => (views().some(({ value }) => value === chosenView()) ? chosenView() : 'Machine')
  const onMachine = () => view() === 'Machine'

  // the view goes to where the open layer draws. A sequence is on the machine too, so it stays on the strip if it's there.
  createEffect(
    on(
      () => shownLayer() && layerPlace(shownLayer()!),
      (place) => {
        if (place === undefined) return
        const display = placeDisplay(place)
        if (display !== null) setView(`Display:${display}`)
        else if (place !== 'Sequence' || untrack(view) !== 'Strip') setView('Machine')
      },
    ),
  )

  // a plane with no extent has no size to give a layer
  const planes = () => machine.planes.filter((plane) => plane.extent[0] > 0 && plane.extent[1] > 0)
  const planeOf = (layer: SceneLayer | undefined) => planes().find(({ name }) => layer !== undefined && name === layerPlane(layer)) ?? null
  const plane = () => planeOf(shownLayer())

  /** The size the editor previews a display at when the machine doesn't have it, and an area on a plane it doesn't have */
  const ownSize = (): Vec2 => [Math.max(width(), 1), Math.max(height(), 1)]
  const displaySize = (name: string): Vec2 => {
    const display = machine.displays.find((display) => display.name === name)
    return display ? [display.width, display.height] : ownSize()
  }
  const machineHasDisplay = (name: string) => machine.displays.some((display) => display.name === name)

  /** A display layer is the size of its display, and an area the size of its plane */
  const layerSurface = (layer: SceneLayer) => {
    if (layer.display !== undefined) return displaySize(layer.display)
    if (layer.surface === 'Sequence') return null
    return planeOf(layer)?.extent ?? ownSize()
  }

  /** Whether a layer is on what's on show, as those are the ones a click there can reach and have handles there */
  const onView = (layer: SceneLayer) => {
    const shown = view()
    if (shown === 'Strip') return layer.display === undefined && layer.surface === 'Sequence'
    if (shown === 'Machine') return layer.display === undefined && layer.surface !== 'Sequence' && JSON.stringify(layer.surface) === JSON.stringify(shownLayer()?.surface)
    return layer.display === viewDisplay(shown)
  }

  /** The open layer or mask, if its layer is on what's on show */
  const openOnView = () => {
    const open = selection[0]()
    const layer = open && scene()?.layers[open.layer]
    return layer && onView(layer) ? open : null
  }

  /** The render of what the canvas shows. A display nothing draws on has none. */
  const shownCanvas = () => drawn()?.canvases.find((on) => on.display === viewDisplay(view()) && (on.display !== null || view() === 'Strip')) ?? null
  /** The canvas's size in points, which for a display nothing draws on is the display's */
  const canvasSize = (): Vec2 => {
    const shown = shownCanvas()
    if (shown) return [shown.width, shown.height]
    const display = viewDisplay(view())
    return display === null ? [0, 0] : displaySize(display)
  }

  /** What a display shows, for drawing it where it is on the machine. `null` for one the scene doesn't draw on. */
  const previewDisplayFrame = (display: string) => {
    const shown = clipFrame()
    if (shown) {
      const on = shown.clip.canvases.find((clipCanvas) => clipCanvas.display === display)
      return on ? clipFrameColors(on, shown.frame) : null
    }
    const rendered = drawn()?.canvases.find((on) => on.display === display)
    return rendered ? new Uint8ClampedArray(rendered.colors) : null
  }

  const ledColors = () => drawn()?.led_colors ?? {}

  const ledColor = (name: string) => {
    const shown = clipFrame()
    if (shown) return clipLedColor(shown.clip, shown.frame, name)
    const color = ledColors()[name]
    return color ? cssColor(color) : null
  }

  /** Regions that aren't drawn over the preview, by layer and whether it's the layer's or its mask's */
  const [hiddenRegions, setHiddenRegions] = createSignal<ReadonlySet<string>>(new Set())
  const regionKey = ({ layer, mask }: Selection) => `${layer}${mask ? ' mask' : ''}`
  const regionVisibility: RegionVisibility = {
    shown: (of) => !hiddenRegions().has(regionKey(of)),
    show(of, shown) {
      const next = new Set(hiddenRegions())
      if (shown) next.delete(regionKey(of))
      else next.add(regionKey(of))
      setHiddenRegions(next)
    },
  }
  /** Whether what's open has its region drawn, which it does with nothing open */
  const openShown = () => {
    const open = selection[0]()
    return open === null || regionVisibility.shown(open)
  }

  /** Where the open layer or mask is drawn, in scene units. Left out while playing, where it's the scene that's being watched. */
  const outline = (): Vec2[][] => {
    const open = openOnView()
    const rendered = drawn()
    if (playback.playing() || open === null || !rendered || !openShown()) return []
    // one loop for each copy of its shape
    return (open.mask ? rendered.mask_outlines : rendered.outlines)[open.layer] ?? []
  }

  /**
   * What's open is what's selected in the preview, if it's on what's on show. Its handles are left out while playing,
   * as its outline is.
   */
  const selected = () => (playback.playing() || error() !== null ? null : openOnView())
  const selectedPose = () => {
    const open = selected()
    const rendered = drawn()
    if (open === null || !rendered || !openShown()) return null
    return (open.mask ? rendered.mask_shapes : rendered.shapes)[open.layer] ?? null
  }
  /** Which of the selected shape's transforms is being edited in the config tree, if any */
  const editedTransform = () => {
    const open = selected()
    return open && openTransform(treeNode(), open)
  }
  const hasShape = (open: Selection | null) => open !== null && selectedShape(scene(), open) !== undefined

  /**
   * Selects the region at a point, or leaves it with nothing selected. A layer with no region isn't selected this way.
   * While a field is choosing a node of the open layer's graph, it gets the one nearest the point instead.
   */
  const pick = (point: Vec2 | null) => {
    const [open, setOpen] = selection
    if (playback.playing() || error() !== null || !drawn()) return
    const choosing = nodePicking.choosing[0]()
    const graph = selectedGraph()
    if (choosing && graph && point) {
      const nearest = graph.nodes.reduce((best, node, index) => (distance(node, point) < distance(graph.nodes[best], point) ? index : best), 0)
      if (graph.names[nearest] !== undefined) choosing.choose(graph.names[nearest])
      nodePicking.choosing[1](null)
      return
    }
    const under = point && layerUnder(scene()!, drawn()!.outlines, point, onView)
    if (under !== null) setOpen({ layer: under, mask: false })
    else if (hasShape(open())) setOpen(null)
  }

  /** Brings a track's field up in the config, from wherever that's been left */
  const reveal = ({ prop, layer, mask }: SceneTrack) => {
    batch(() => {
      setShowingCode(false)
      // a sound layer is always open
      if (layer !== null) selection[1]({ layer, mask })
      revealing[1](prop.get())
    })
  }

  /** Brings a layer up in the config, as for a traversal, which is under its graph */
  const openLayer = (layer: number) => {
    batch(() => {
      setShowingCode(false)
      selection[1]({ layer, mask: false })
    })
  }

  /** The open layer's graph. Left out while playing, as its outline is. */
  const selectedGraph = () => {
    const open = selected()
    return open === null ? null : (drawn()?.graphs[open.layer] ?? null)
  }

  const nodePicking: NodePicking = {
    nodes: () => selectedGraph()?.names ?? [],
    choosing: createSignal<{ choose: (name: string) => void } | null>(null),
    highlighted: createSignal<string | null>(null),
  }

  // a choice is for the layer it started in
  createEffect(on(() => selection[0]()?.layer, () => nodePicking.choosing[1](null), { defer: true }))

  const handles = (space: PreviewSpace) => (
    <>
      <GraphOverlay
        space={space}
        // still there to click on while choosing one of its LEDs
        graph={openShown() || nodePicking.choosing[0]() !== null ? selectedGraph() : null}
        layer={selected()?.layer ?? null}
        highlighted={nodePicking.highlighted[0]()}
        choosing={nodePicking.choosing[0]() !== null}
      />
      <ShapeHandles space={space} selection={selected()} pose={selectedPose()} outlines={outline()} time={playback.position()} grid={snap().on ? snap().size : null} openTransform={editedTransform()} />
    </>
  )

  const [screenSize, setScreenSize] = createSignal<Vec2>([0, 0])
  onMount(() => {
    const observer = new ResizeObserver(() => setScreenSize([canvas.clientWidth, canvas.clientHeight]))
    observer.observe(canvas)
    onCleanup(() => observer.disconnect())
  })

  /** Keeps its shape however much room the preview has, with one point of a sequence being as tall as it is wide */
  const screenAspect = () => {
    const [width, height] = canvasSize()
    return Math.max(width, 1) / Math.max(height, 1)
  }

  /** The canvas is the part of the scene the machine rendered, stretched over it */
  const screenSpace: PreviewSpace = {
    project([x, y]) {
      const area = shownCanvas()?.area
      const [width, height] = screenSize()
      return area ? [((x - area.x) / area.width) * width, ((y - area.y) / area.height) * height] : null
    },
    unproject(clientX, clientY) {
      const area = shownCanvas()?.area
      const { left, top, width, height } = canvas.getBoundingClientRect()
      return area ? [area.x + ((clientX - left) / width) * area.width, area.y + ((clientY - top) / height) * area.height] : null
    },
  }

  /** Each layer's and mask's region as the machine last drew it, found by the shape itself */
  const shapePoses: ShapePoses = {
    of: (shape) => {
      const rendered = drawn()
      const layers = scene()?.layers ?? []
      if (!rendered) return null
      const index = layers.findIndex((layer) => layer.region === shape)
      if (index >= 0) return rendered.shapes[index] ?? null
      const masked = layers.findIndex((layer) => layer.mask?.region === shape)
      return masked >= 0 ? (rendered.mask_shapes[masked] ?? null) : null
    },
    time: playback.position,
  }

  return (
    <SceneContext.Provider value={editing}>
      <NodePickingContext.Provider value={nodePicking}>
      <LayerSurfacesContext.Provider value={layerSurface}>
        <RevealContext.Provider value={revealing}>
        <RegionVisibilityContext.Provider value={regionVisibility}>
        <ShapePosesContext.Provider value={shapePoses}>
        <div class="scene-editor" ref={root}>
        <header class="scene-file">
          <button type="button" class="scene-button" onClick={close}>
            ← Scenes
          </button>
          <span class="scene-file-name" title={name()}>
            {name()}
          </span>
          <div class="scene-file-actions">
            <button type="button" class="scene-button" disabled={!unsaved() || fileWork() !== null} onClick={save}>
              {fileWork() === 'saving' ? 'Saving…' : 'Save'}
            </button>
            <SceneNameButton label="Save as" title="Save scene as" action="Save" acting="Saving…" name={name()} disabled={fileWork() !== null} onName={saveAs} />
            <button type="button" class="scene-button" disabled={!unsaved() || fileWork() !== null} onClick={() => setConfirming('reset')}>
              Reset
            </button>
            <button type="button" class="scene-button" disabled={fileWork() !== null} onClick={() => setConfirming('delete')}>
              {fileWork() === 'deleting' ? 'Deleting…' : 'Delete'}
            </button>
          </div>
          <Show when={unsaved()}>
            <span class="scene-note">Unsaved changes</span>
          </Show>
          <Show when={fileError()}>{(text) => <span class="scene-note scene-note-warn">{text()}</span>}</Show>
          <Show when={drawn()?.reference_error}>{(text) => <span class="scene-note scene-note-warn">{text()}</span>}</Show>
          <div class="scene-file-view">
            <div class="scene-reference" title="Another scene drawn with this one, to see how the two look together. Only this scene's layers can be selected.">
              <span>Show with</span>
              <Dropdown
                compact
                label="Scene to show with this one"
                options={[{ value: NO_REFERENCE, label: 'Nothing' }, ...contentScenes().filter((scene) => scene !== name()).map((scene) => ({ value: scene, label: scene }))]}
                value={reference()?.name ?? NO_REFERENCE}
                onChange={(chosen) => setReference(chosen === NO_REFERENCE ? null : { name: chosen, above: reference()?.above ?? true })}
              />
              <Show when={reference()}>
                {(shown) => (
                  <SegmentGroup.Root
                    class="scene-prop-kind scene-view"
                    aria-label="Where the other scene is drawn"
                    value={shown().above ? 'Above' : 'Below'}
                    onValueChange={(details) => setReference({ ...shown(), above: details.value === 'Above' })}
                  >
                    <SegmentGroup.Indicator />
                    <For each={REFERENCE_PLACES}>
                      {(option) => (
                        <SegmentGroup.Item value={option}>
                          <SegmentGroup.ItemText>{option}</SegmentGroup.ItemText>
                          <SegmentGroup.ItemControl />
                          <SegmentGroup.ItemHiddenInput />
                        </SegmentGroup.Item>
                      )}
                    </For>
                  </SegmentGroup.Root>
                )}
              </Show>
            </div>
            <div class="scene-snap" title="Snaps the points of a polygon, route or bezier, and how far a shape is moved, to a grid in the shape's own units as they're dragged">
              <Checkbox.Root checked={snap().on} onCheckedChange={(details) => setSnap({ on: details.checked === true })}>
                <Checkbox.Control>
                  <Checkbox.Indicator>✓</Checkbox.Indicator>
                </Checkbox.Control>
                <Checkbox.Label>Snap to grid</Checkbox.Label>
                <Checkbox.HiddenInput />
              </Checkbox.Root>
              <NumberBox
                min="0.001"
                step="any"
                aria-label="Grid size"
                disabled={!snap().on}
                value={snap().size}
                onNumber={(size) => size > 0 && setSnap({ size })}
              />
            </div>
            <Show when={onMachine() && !isSequence() && shownLayer()?.display === undefined && planes().length === 0}>
              <span class="scene-note">The machine has no planes</span>
            </Show>
            <Show when={onMachine() && isSequence() && hasShape(selection[0]())}>
              <span class="scene-note">A sequence has no place on the machine, so its handles are on the strip</span>
            </Show>
            <Show when={view() === 'Strip'}>
              <NumberBox min="1" aria-label="Number of LEDs" value={leds()} onNumber={setLeds} />
            </Show>
            <Show when={viewDisplay(view()) !== null && !machineHasDisplay(viewDisplay(view())!)}>
              <span class="scene-note" title="The machine has no display by this name, so it's previewed at this size">
                Not on the machine
              </span>
              <NumberBox min="1" aria-label="Width" value={width()} onNumber={setWidth} />
              <span>×</span>
              <NumberBox min="1" aria-label="Height" value={height()} onNumber={setHeight} />
            </Show>
            <Show when={views().length > 1}>
              <SegmentGroup.Root class="scene-prop-kind scene-view" aria-label="Preview" value={view()} onValueChange={(details) => setView(details.value as View)}>
                <SegmentGroup.Indicator />
                <For each={views()}>
                  {(option) => (
                    <SegmentGroup.Item value={option.value}>
                      <SegmentGroup.ItemText>{option.label}</SegmentGroup.ItemText>
                      <SegmentGroup.ItemControl />
                      <SegmentGroup.ItemHiddenInput />
                    </SegmentGroup.Item>
                  )}
                </For>
              </SegmentGroup.Root>
            </Show>
          </div>
        </header>
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
              defaultSize={[29, 71]}
            >
              <Splitter.Panel id="source">
                <div class="scene-source">
                  <Show when={scene() !== null}>
                    <div class="scene-leds">
                      <LedsNote />
                    </div>
                  </Show>
                  <div class="scene-source-header">
                    <span class="scene-tabs-end" ref={setTreeTools} hidden={showingCode()} />
                    <button
                      type="button"
                      class="scene-icon-button"
                      classList={{ active: showingCode() }}
                      aria-label={showingCode() ? 'Show the tree' : 'Show the code'}
                      aria-pressed={showingCode()}
                      title={showingCode() ? 'Show the tree' : 'Show the scene as RON'}
                      onClick={() => setShowingCode(!showingCode())}
                    >
                      <icons.Code aria-hidden="true" />
                    </button>
                  </div>
                  <div class="scene-source-body">
                    <Show
                      when={!showingCode()}
                      fallback={
                        <textarea
                          aria-label="Scene, as RON"
                          spellcheck={false}
                          value={ron()}
                          onInput={(e) => {
                            setRon(e.currentTarget.value)
                            send({ Ron: e.currentTarget.value })
                          }}
                        />
                      }
                    >
                      <Show when={error() !== null}>
                        <p class="scene-note scene-note-warn">
                          {scene() === null ? "This scene can't be read. Fix the error to edit it." : 'This is the last scene that rendered. Fix the error to edit it.'}
                        </p>
                      </Show>
                      <Show when={scene() !== null}>
                        {/* an edit here would replace the RON with the last scene that could be read */}
                        <div class="scene-editing" inert={error() !== null}>
                          <ConfigPanel selection={selection} toolbar={treeTools()} onSelect={setTreeNode} />
                        </div>
                      </Show>
                    </Show>
                  </div>
                </div>
              </Splitter.Panel>
              <Splitter.ResizeTrigger id="source:output" aria-label="Resize scene source" />
              <Splitter.Panel id="output" class="scene-output">
                <Show when={error()}>{(text) => <pre class="scene-error">{text()}</pre>}</Show>
                {/* both stay up, so the camera and the last render are still there on the way back */}
                <div class="scene-machine" classList={{ stale: error() !== null, 'scene-choosing-node': nodePicking.choosing[0]() !== null }} hidden={!onMachine()}>
                  <PlayfieldView
                    ledColor={ledColor}
                    displayFrame={previewDisplayFrame}
                    plane={() => (isSequence() ? null : plane())}
                    outline={outline}
                    onClickPlane={(point) => !isSequence() && plane() && pick(point)}
                    overlay={(space) => <Show when={!isSequence() && plane()}>{handles(space)}</Show>}
                  />
                </div>
                <div class="scene-dmd" classList={{ stale: error() !== null, 'scene-choosing-node': nodePicking.choosing[0]() !== null }} hidden={onMachine()}>
                  <div class="scene-dmd-screen" style={{ '--aspect': screenAspect(), 'min-height': `${view() === 'Strip' ? MIN_DRAWN_HEIGHT : 0}px` }}>
                    <canvas ref={canvas} onClick={(e) => pick(screenSpace.unproject(e.clientX, e.clientY))} />
                    <Show when={shownCanvas()?.area}>
                      {(area) => (
                        <svg viewBox={`${area().x} ${area().y} ${area().width} ${area().height}`} preserveAspectRatio="none" aria-hidden="true">
                          <For each={outline()}>{(loop) => <polygon points={loop.join(' ')} />}</For>
                        </svg>
                      )}
                    </Show>
                    {handles(screenSpace)}
                  </div>
                </div>
              </Splitter.Panel>
            </Splitter.Root>
          </Splitter.Panel>
          <Splitter.ResizeTrigger id="workspace:timeline" aria-label="Resize timeline" />
          <Splitter.Panel id="timeline">
            <Show when={scene() !== null}>
              <div class="scene-editing" inert={error() !== null}>
                <Timeline playback={playback} rendering={playback.playing() && sceneClipRendering()} error={sceneClipError()} graphs={drawn()?.graphs ?? []} reveal={reveal} openLayer={openLayer} />
              </div>
            </Show>
          </Splitter.Panel>
        </Splitter.Root>
        <ConfirmDialog
          open={confirming() === 'close'}
          title="Discard unsaved changes?"
          description={`${name()} has changes that haven't been saved.`}
          cancel="Keep editing"
          action="Discard"
          onCancel={() => setConfirming(null)}
          onConfirm={leave}
        />
        <ConfirmDialog
          open={confirming() === 'reset'}
          title="Reset to the saved scene?"
          description={`The unsaved changes to ${name()} will be lost.`}
          cancel="Keep editing"
          action="Reset"
          onCancel={() => setConfirming(null)}
          onConfirm={reset}
        />
        <ConfirmDialog
          open={confirming() === 'delete'}
          title="Delete this scene?"
          description={`${name()} will be removed from the machine's content folder. This can't be undone.`}
          cancel="Keep editing"
          action="Delete"
          onCancel={() => setConfirming(null)}
          onConfirm={remove}
        />
        </div>
        </ShapePosesContext.Provider>
        </RegionVisibilityContext.Provider>
        </RevealContext.Provider>
      </LayerSurfacesContext.Provider>
      </NodePickingContext.Provider>
    </SceneContext.Provider>
  )
}

const distance = ([ax, ay]: Vec2, [bx, by]: Vec2) => Math.hypot(ax - bx, ay - by)

/** Whether a key pressed in `target` goes to text being edited there */
const typingIn = (target: EventTarget | null) =>
  target instanceof HTMLElement &&
  (target.isContentEditable ||
    target instanceof HTMLTextAreaElement ||
    target instanceof HTMLSelectElement ||
    (target instanceof HTMLInputElement && !['button', 'checkbox', 'radio', 'range', 'color', 'file', 'reset', 'submit'].includes(target.type)))
