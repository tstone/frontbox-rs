import { Checkbox } from '@ark-ui/solid/checkbox'
import { Splitter } from '@ark-ui/solid/splitter'
import { batch, createEffect, createMemo, createSignal, For, on, onCleanup, onMount, Show, untrack } from 'solid-js'
import * as icons from '../../components/icons'
import { HeaderTakeover } from '../../components/HeaderTakeover'
import ShortcutsButton, { MOD } from './ShortcutsButton'
import { createHistory } from './history'
import { LightClickingContext } from './LightPicker'
import ViewAngleMenu from './ViewAngleMenu'
import { typingIn } from '../../lib/keys'
import { cssColor } from '../../lib/leds'
import { machine } from '../../state/console'
import { addContentScene, contentScenes, saveContentScene } from '../../state/contentScenes'
import { playContentSound } from '../../state/contentSounds'
import {
  forgetSceneRender,
  renderClips,
  renderScene,
  reseedScenePreview,
  sceneClip,
  sceneClipError,
  sceneClipRendering,
  sceneRender,
  type ClipCanvasFrames,
  type SceneClip,
} from '../../state/sceneRender'
import type { ClipSound } from '../../types/generated/ClipSound'
import type { ReferenceScene } from '../../types/generated/ReferenceScene'
import type { RenderedScene } from '../../types/generated/RenderedScene'
import type { SceneSource } from '../../types/generated/SceneSource'
import PlayfieldView, { countOnPlane, type PreviewSpace, type ViewAngle } from '../machine/PlayfieldView'
import PropertiesPanel, { drawnAspect } from './PropertiesPanel'
import ConfirmDialog from './ConfirmDialog'
import { createPlayback } from './playback'
import { Dropdown, NumberBox, LayerSurfacesContext, NodePickingContext, RegionVisibilityContext, SceneContext, ShapePosesContext, type NodePicking, type RegionVisibility, type ShapePoses } from './SceneFields'
import { configNodes, is, openTransform, selectionNode } from './configNodes'
import { ConfigTreeContext, createConfigTree } from './configTree'
import { changesEachCycle, cycleCount, editedScene, isOpen, layerPlace, layerPlane, placeDisplay, sceneSource, selectedShape, snap as snapTime, withKey, withPick, withoutDroppedVariables, withoutDroppedTiming, type Lens, type LayerPlace, type Scene, type SceneLayer, type Selection, type Vec2 } from './sceneModel'
import LayerTools, { drawnLayer, type DrawnTool } from './LayerTools'
import { LayerColorContext, layerColor, layerColorClass, type LayerColor } from './ScenePortal'
import { createLayerMenu, LayerMenuContext } from './LayerMenu'
import TextEditor, { editableText, withText } from './TextEditing'
import ShapeDrawing, { PointsDrawing, type DrawnBox, type DrawnPoints } from './ShapeDrawing'
import { SceneNameButton } from './SceneNameDialog'
import GraphOverlay from './GraphOverlay'
import ShapeHandles, { fullOutline, HoverOutline, layerUnder } from './ShapeHandles'
import Timeline from './Timeline'

/**
 * What the preview shows: the machine, with the camera on one of its planes, the strip of the `Sequence` layers, which
 * is where their handles are, or one display on its own. It's also where the tools add a layer. `Machine` is the
 * machine with no plane, for one that has none.
 */
type View = 'Machine' | `Plane:${string}` | 'Strip' | `Display:${string}`

/** The display a view is of, which is `null` for the machine and the strip */
const viewDisplay = (view: View) => (view.startsWith('Display:') ? view.slice('Display:'.length) : null)
/** The plane a view is of, which is `null` for the others */
const viewPlane = (view: View) => (view.startsWith('Plane:') ? view.slice('Plane:'.length) : null)
/** The color of the layers a view shows */
const viewColor = (view: View): LayerColor => (viewDisplay(view) === null ? 'lights' : 'display')

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

/** A light's color in a frame of a clip, as CSS or `null` for off */
function clipLightColor({ lightIndex, lights, frames }: SceneClip, frame: number, name: string) {
  const index = lightIndex.get(name)
  if (index === undefined) return null
  const at = (frame * (lights.length / frames) / 3 + index) * 3
  return cssColor({ r: lights[at], g: lights[at + 1], b: lights[at + 2] })
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
  const [fileWork, setFileWork] = createSignal<'saving' | null>(null)
  const [fileError, setFileError] = createSignal<string | null>(null)
  const [confirming, setConfirming] = createSignal<'close' | 'reset' | null>(null)
  const [lights, setLights] = createSignal(32)
  const [width, setWidth] = createSignal(128)
  const [height, setHeight] = createSignal(32)
  const [chosenView, setView] = createSignal<View>('Machine')
  // another scene drawn with this one, to see how the two look together. Only this one's layers can be selected. One
  // stored before z stacked them can have more than its name.
  const stored = readReferences()[props.name]
  const [reference, setReferenceState] = createSignal<ReferenceScene | null>(stored ? { name: stored.name } : null)
  const setReference = (next: ReferenceScene | null) => {
    setReferenceState(next)
    writeReference(name(), next)
  }
  const [snap, setSnapState] = createSignal(readSnap())
  const setSnap = (change: Partial<Snap>) => {
    setSnapState({ ...snap(), ...change })
    writeSnap(snap())
  }
  const selection = createSignal<Selection | null>({ layer: 0 })
  // the sidebar shows the selected node's properties, or the scene as RON
  const [showingCode, setShowingCode] = createSignal(false)
  // what's on show, which stays put while the source doesn't parse
  const [drawn, setDrawn] = createSignal<RenderedScene | null>(null)

  const size = createMemo(() => ({ lights: lights(), width: width(), height: height() }), undefined, {
    equals: (a, b) => a.lights === b.lights && a.width === b.width && a.height === b.height,
  })

  // each play of the scene makes its own random choices, as each playback in the game does
  const playback = createPlayback(() => scene()?.timing, () => sceneClip(clipCycle()) !== null && !sceneClipError(), reseedScenePreview)

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

  // what the scene was before each edit, to undo
  const history = createHistory<Scene>()

  /** Shows a scene and has the machine draw it, without it being an edit to undo */
  const apply = (next: Scene) => {
    setScene(next)
    send(sceneSource(next))
  }

  const editing: Lens<Scene> = {
    get: () => scene()!,
    set: (next) => {
      const current = scene()!
      history.record(current)
      apply(withoutDroppedTiming(current, withoutDroppedVariables(current, next)))
    },
  }

  const undo = () => {
    const current = scene()
    const before = current && history.undo(current)
    if (before) apply(before)
  }
  const redo = () => {
    const current = scene()
    const after = current && history.redo(current)
    if (after) apply(after)
  }

  // a light layer from before layers started on every light has none, which the game can't play, so it's given every
  // light. That's how the file is read rather than an edit, so it isn't one to undo.
  createEffect(() => {
    const current = scene()
    if (current === null || current.layers.every((layer) => layer.display !== undefined || layer.lights !== undefined)) return
    apply({ ...current, layers: current.layers.map((layer) => (layer.display === undefined && layer.lights === undefined ? { ...layer, lights: 'Every' } : layer)) })
  })

  const unsaved = () => ron() !== saved()

  const workOnFile = async (work: 'saving', action: () => Promise<void>) => {
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
    history.clear()
    batch(() => {
      setConfirming(null)
      selection[1]({ layer: 0 })
      setRon(saved())
    })
    send({ Ron: saved() })
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
      } else if ((e.ctrlKey || e.metaKey) && !e.altKey && (e.key.toLowerCase() === 'z' || e.key.toLowerCase() === 'y') && !typingIn(e.target)) {
        // a field being typed in undoes its own typing
        e.preventDefault()
        if (e.key.toLowerCase() === 'y' || e.shiftKey) redo()
        else undo()
      } else if (e.key === ' ' && !e.ctrlKey && !e.metaKey && !e.altKey && !typingIn(e.target)) {
        // taken from a focused button too, which would otherwise be pressed
        e.preventDefault()
        if (e.repeat) return
        if (playback.playing()) playback.stop()
        else if (scene()?.timing) playback.play()
      } else if ((e.key === 'Delete' || e.key === 'Backspace') && drawing() === null && !typingIn(e.target) && !(e.target instanceof Element && e.target.closest('[role="dialog"], [role="alertdialog"], [role="menu"]'))) {
        // a polygon being drawn takes Backspace for its last corner
        if (removeSelected()) e.preventDefault()
      } else if (e.key.toLowerCase() === 'k' && !e.ctrlKey && !e.metaKey && !e.altKey && !typingIn(e.target)) {
        if (keyAtPlayhead()) e.preventDefault()
      } else if (e.key === 'Escape' && !typingIn(e.target) && drawing() === null && !(e.target instanceof Element && e.target.closest('[role="dialog"], [role="menu"]'))) {
        // a click where there's no shape can select a layer under it that covers everything, so this is how to select nothing
        if (tree.selected() !== null) {
          e.preventDefault()
          tree.select(null)
        }
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

  // a plane with no extent has no size to give a layer
  const planes = () => machine.planes.filter((plane) => plane.extent[0] > 0 && plane.extent[1] > 0)
  const planeOf = (layer: SceneLayer | undefined) => planes().find(({ name }) => layer !== undefined && name === layerPlane(layer)) ?? null

  /** How many of the machine's lights are on each plane, by its name */
  const lightsOnPlanes = createMemo(() => {
    const positions = Object.values(machine.hardware?.lights.by_name ?? {}).flatMap(({ location }) => (location ? [location] : []))
    return new Map(planes().map((plane) => [plane.name, countOnPlane(plane, positions)]))
  })

  /**
   * The plane to show when none is chosen: that of the scene's last layer on one the machine has, or else the one with
   * the most lights, which is where a scene most likely goes. The largest wins a tie, such as a machine with no lights placed.
   */
  const defaultPlane = () => {
    const names = planes().map(({ name }) => name)
    const last = scene()?.layers.map(layerPlane).findLast((name) => name !== null && names.includes(name))
    if (last) return last
    const lights = (name: string) => lightsOnPlanes().get(name) ?? 0
    const area = ({ extent }: { extent: [number, number] }) => extent[0] * extent[1]
    const [likeliest] = [...planes()].sort((a, b) => lights(b.name) - lights(a.name) || area(b) - area(a))
    return likeliest?.name ?? null
  }
  const defaultView = (): View => (defaultPlane() === null ? 'Machine' : `Plane:${defaultPlane()}`)

  /** What the preview can show, grouped by planes, displays, and the strip */
  const views = (): { value: View; label: string; group?: string; color?: LayerColor }[] =>
    [
      ...(planes().length === 0 ? [{ value: 'Machine' as const, label: 'Machine', group: 'Planes' }] : planes().map(({ name }) => ({ value: `Plane:${name}` as const, label: name, group: 'Planes' }))),
      ...displays().map((display) => ({ value: `Display:${display}` as const, label: display, group: 'Displays' })),
      ...(hasStrip() ? [{ value: 'Strip' as const, label: 'Sequence strip', group: 'Sequences' }] : []),
    ].map((option) => ({ ...option, color: viewColor(option.value) }))
  /** What's on show, which is the default plane once what was chosen isn't there to show */
  const view = (): View => (views().some(({ value }) => value === chosenView()) ? chosenView() : defaultView())
  const onMachine = () => view() === 'Machine' || viewPlane(view()) !== null

  // the view goes to where the open layer draws. A sequence is on every plane, so it leaves the view alone unless it's
  // on a display.
  createEffect(
    on(
      () => shownLayer() && layerPlace(shownLayer()!),
      (place) => {
        if (place === undefined) return
        const display = placeDisplay(place)
        if (display !== null) setView(`Display:${display}`)
        else if (place !== 'Sequence') setView(`Plane:${place.slice('Area:'.length)}`)
        else if (viewDisplay(untrack(view)) !== null) setView(defaultView())
      },
    ),
  )

  // a press of Reset view, which puts the camera back to how it first looks at the plane on show
  const [refocus, setRefocus] = createSignal(0)
  // the angle the camera looks at a plane from, which stays for the planes after it
  const [viewAngle, setViewAngle] = createSignal<ViewAngle>({ tilt: 30, turn: 0 })

  /** The plane on show, if it's one the machine has */
  const plane = () => planes().find(({ name }) => name === viewPlane(view())) ?? null

  /**
   * Where a layer from the tools goes: on the plane or display on show. With neither, `reason` says why there's
   * nowhere.
   */
  const newLayerAt = (): { place: LayerPlace | null; reason: string | null } => {
    const shown = view()
    const display = viewDisplay(shown)
    if (display !== null) return { place: `Display:${display}`, reason: null }
    const on = viewPlane(shown)
    if (on !== null) return { place: `Area:${on}`, reason: null }
    if (shown === 'Strip') return { place: null, reason: 'Choose a plane or a display to add a layer there' }
    return { place: null, reason: 'The machine has no planes to draw on. Choose a display to add a layer there.' }
  }

  // a shape or text being drawn on the preview, which goes once it's drawn or there's nowhere to draw it
  const [drawing, setDrawing] = createSignal<DrawnTool | null>(null)
  createEffect(() => {
    if (newLayerAt().place === null || playback.playing()) setDrawing(null)
  })
  const finishDrawing = (drawn: DrawnBox | DrawnPoints) => {
    const tool = drawing()
    const place = newLayerAt().place
    setDrawing(null)
    if (!tool || !place) return
    tree.addLayer(drawnLayer(tool, editing.get(), place, layerSurface, drawn))
    // new text is typed straight away, over the box it was drawn as
    if (tool === 'Text') editText(editing.get().layers.length - 1)
  }

  // a text layer whose words are being typed over in the preview, and what they were, to put back
  const [editingText, setEditingText] = createSignal<{ index: number; was: string } | null>(null)
  const editText = (index: number) => {
    const was = editableText(editing.get(), editing.get().layers[index])
    if (was === null) return
    selection[1]({ layer: index })
    setEditingText({ index, was })
  }
  const finishText = (keep: boolean) => {
    const edit = editingText()
    setEditingText(null)
    if (edit && !keep) editing.set(withText(editing.get(), edit.index, edit.was))
  }
  createEffect(() => {
    if (playback.playing()) setEditingText(null)
  })

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

  // the tree on the timeline and the properties panel share what's selected, which also says what the handles are for
  const nodes = createMemo(() => (scene() === null ? [] : configNodes(editing, layerSurface)))
  const tree = createConfigTree(editing, nodes, selection)

  // right-clicking a layer, in the preview or on the timeline
  const layerMenu = createLayerMenu(tree, {
    scene: () => editing.get(),
    editing,
    color: (id) => {
      if (id.startsWith('sounds/')) return 'sound'
      const layer = scene()?.layers[Number(id.split('/')[1])]
      return layer && layerColor(layer)
    },
    canEditText: (index) => editableText(editing.get(), editing.get().layers[index]) !== null,
    editText: (index) => editText(index),
  })

  /**
   * Asks to take out what's selected, as its Remove button would: a layer, a sound, or anything else in a list. Returns
   * whether there was something to take out.
   */
  const removeSelected = () => {
    const id = tree.selected()
    return id !== null && layerMenu.remove(id)
  }

  /**
   * Adds a key at the playhead to the selected prop, if it's keyframed, or a pick there if it's random, as the button in
   * its lane would. Returns whether there was one to add to.
   */
  const keyAtPlayhead = () => {
    const id = tree.selected()
    const node = id === null ? undefined : tree.find(id)
    const prop = node && is(node, 'prop')
    if (!prop || !scene()?.timing) return false
    const at = snapTime(playback.position())
    const current = prop.lens.get()
    if ('Keyframes' in current) prop.lens.set({ Keyframes: withKey(current.Keyframes, at) })
    else if ('Random' in current) prop.lens.set({ Random: withPick(current.Random, at) })
    else return false
    return true
  }

  /** Whether a layer is on what's on show, as those are the ones a click there can reach and have handles there */
  const onView = (layer: SceneLayer) => {
    const shown = view()
    if (shown === 'Strip') return layer.display === undefined && layer.surface === 'Sequence'
    if (shown === 'Machine') return false
    if (viewPlane(shown) !== null) return layer.display === undefined && layerPlane(layer) === viewPlane(shown)
    return layer.display === viewDisplay(shown)
  }

  /** The open layer, if it's on what's on show */
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

  const lightColors = () => drawn()?.light_colors ?? {}

  const lightColor = (name: string) => {
    const shown = clipFrame()
    if (shown) return clipLightColor(shown.clip, shown.frame, name)
    const color = lightColors()[name]
    return color ? cssColor(color) : null
  }

  /** Layers whose regions aren't drawn over the preview */
  const [hiddenRegions, setHiddenRegions] = createSignal<ReadonlySet<number>>(new Set())
  const regionVisibility: RegionVisibility = {
    shown: (of) => !hiddenRegions().has(of.layer),
    show(of, shown) {
      const next = new Set(hiddenRegions())
      if (shown) next.delete(of.layer)
      else next.add(of.layer)
      setHiddenRegions(next)
    },
  }
  /** Whether what's open has its region drawn, which it does with nothing open */
  const openShown = () => {
    const open = selection[0]()
    return open === null || regionVisibility.shown(open)
  }

  /** Where the open layer is drawn, in scene units. Left out while playing, where it's the scene that's being watched. */
  const outline = (): Vec2[][] => {
    const open = openOnView()
    const rendered = drawn()
    if (playback.playing() || open === null || !rendered || !openShown()) return []
    // one loop for each copy of its shape
    return rendered.outlines[open.layer] ?? []
  }
  /** Whether what's open is a line, whose outline doesn't close */
  const outlineOpen = () => {
    const open = openOnView()
    const region = open && scene()?.layers[open.layer]?.region
    return !!region && isOpen(region.base)
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
    return rendered.shapes[open.layer] ?? null
  }
  /** Which of the selected shape's transforms is selected in the tree, if any */
  const editedTransform = () => {
    const open = selected()
    return open && openTransform(tree.selected(), open)
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
    const under = point && layerUnder(scene()!, drawn()!.outlines, point, onView, layerSurface)
    if (under !== null) setOpen({ layer: under })
    else if (hasShape(open())) setOpen(null)
  }

  /** Opens the menu of the layer at a point in the preview, at the pointer, if there's one there */
  const openMenuAt = (point: Vec2 | null, x: number, y: number) => {
    if (playback.playing() || error() !== null || !drawn() || !point) return
    const under = layerUnder(scene()!, drawn()!.outlines, point, onView, layerSurface)
    if (under !== null) layerMenu.open(selectionNode({ layer: under }), x, y)
  }

  /** Opens the open layer's menu for a right-click on one of its handles, which are over the preview. Returns whether it was on one. */
  const handleMenu = (e: MouseEvent) => {
    const open = selected()
    if (!(e.target instanceof Element && e.target.closest('.scene-handles')) || open === null) return false
    e.preventDefault()
    if (!playback.playing()) layerMenu.open(selectionNode({ layer: open.layer }), e.clientX, e.clientY)
    return true
  }

  // the layer a click in the preview would select, which is outlined while the pointer is over it
  const [hovered, setHovered] = createSignal<number | null>(null)
  /** Notes the layer under a point in the preview, other than the open one. Returns whether there is one, to click on. */
  const hoverAt = (point: Vec2 | null) => {
    const ready = point !== null && !playback.playing() && error() === null && drawn() && drawing() === null && nodePicking.choosing[0]() === null
    const under = ready ? layerUnder(scene()!, drawn()!.outlines, point, onView, layerSurface) : null
    const open = selected()
    setHovered(under !== null && open?.layer !== under ? under : null)
    return hovered() !== null
  }
  /** A layer's outline in the preview: its shape's, or the edge of what it's on for one that covers all of it */
  const outlineOf = (index: number) => {
    const layer = scene()?.layers[index]
    if (!layer) return null
    const full = fullOutline(layer, layerSurface(layer))
    return { outlines: full ? [full] : (drawn()?.outlines[index] ?? []), color: layerColor(layer), open: !!layer.region && isOpen(layer.region.base) }
  }
  const hoveredLayer = () => {
    const index = hovered()
    return index === null ? null : outlineOf(index)
  }
  // a layer that covers all of what it's on has no handles, so it's outlined to show it's the one selected
  const selectedFull = () => {
    const open = selected()
    const layer = open ? scene()?.layers[open.layer] : undefined
    return layer && fullOutline(layer, layerSurface(layer)) ? outlineOf(open!.layer) : null
  }

  /** The open layer's graph. Left out while playing, as its outline is. */
  const selectedGraph = () => {
    const open = selected()
    return open === null ? null : (drawn()?.graphs[open.layer] ?? null)
  }

  // what a click on a light in the preview does while lights are being chosen by clicking them
  const lightClicking = createSignal<((name: string) => void) | null>(null)

  const nodePicking: NodePicking = {
    nodes: () => selectedGraph()?.names ?? [],
    choosing: createSignal<{ choose: (name: string) => void } | null>(null),
    highlighted: createSignal<string | null>(null),
  }

  // a choice is for the layer it started in
  createEffect(on(() => selection[0]()?.layer, () => nodePicking.choosing[1](null), { defer: true }))

  /** What goes over the preview: the open layer's handles, or what draws a shape while one's being drawn there */
  const handles = (space: PreviewSpace, shown: () => boolean) => (
    <>
      <GraphOverlay
        space={space}
        // still there to click on while choosing one of its lights
        graph={openShown() || nodePicking.choosing[0]() !== null ? selectedGraph() : null}
        layer={selected()?.layer ?? null}
        highlighted={nodePicking.highlighted[0]()}
        choosing={nodePicking.choosing[0]() !== null}
      />
      <Show when={shown() && hoveredLayer()}>{(layer) => <HoverOutline space={space} outlines={layer().outlines} color={layer().color} open={layer().open} />}</Show>
      <Show when={shown() && selectedFull()}>{(layer) => <HoverOutline selected space={space} outlines={layer().outlines} color={layer().color} />}</Show>
      {/* up once the text's box has been drawn, and kept up as it's redrawn with each word typed */}
      <Show when={shown() ? editingText() : null} keyed>
        {(edit) => (
          <Show when={(drawn()?.outlines[edit.index]?.length ?? 0) > 0}>
            <TextEditor
              space={space}
              outlines={drawn()?.outlines[edit.index] ?? []}
              color={viewColor(view())}
              text={edit.was}
              onInput={(words) => editing.set(withText(editing.get(), edit.index, words))}
              onDone={finishText}
            />
          </Show>
        )}
      </Show>
      <ShapeHandles space={space} selection={selected()} pose={selectedPose()} outlines={outline()} time={playback.position()} grid={snap().on ? snap().size : null} openTransform={editedTransform()} />
      <Show when={shown() && drawing()}>
        {(tool) => (
          <Show
            when={tool() === 'Polygon' || tool() === 'Polyline' || tool() === 'Bezier'}
            fallback={<ShapeDrawing space={space} aspect={drawnAspect(tool())} color={viewColor(view())} onDrawn={finishDrawing} onCancel={() => setDrawing(null)} />}
          >
            <PointsDrawing space={space} open={tool() !== 'Polygon'} color={viewColor(view())} onDrawn={finishDrawing} onCancel={() => setDrawing(null)} />
          </Show>
        )}
      </Show>
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

  /** The part of the scene the canvas shows: what the machine rendered, or all of a display nothing draws on yet */
  const shownArea = () => {
    const area = shownCanvas()?.area
    if (area) return area
    const display = viewDisplay(view())
    if (display === null) return null
    const [width, height] = displaySize(display)
    return { x: 0, y: 0, width, height }
  }

  /** The canvas is the part of the scene it shows, stretched over it */
  const screenSpace: PreviewSpace = {
    project([x, y]) {
      const area = shownArea()
      const [width, height] = screenSize()
      return area ? [((x - area.x) / area.width) * width, ((y - area.y) / area.height) * height] : null
    },
    unproject(clientX, clientY) {
      const area = shownArea()
      const { left, top, width, height } = canvas.getBoundingClientRect()
      return area ? [area.x + ((clientX - left) / width) * area.width, area.y + ((clientY - top) / height) * area.height] : null
    },
  }

  /** Each layer's region as the machine last drew it, found by the shape itself */
  const shapePoses: ShapePoses = {
    of: (shape) => {
      const rendered = drawn()
      const layers = scene()?.layers ?? []
      if (!rendered) return null
      const index = layers.findIndex((layer) => layer.region === shape)
      return index >= 0 ? (rendered.shapes[index] ?? null) : null
    },
    time: playback.position,
  }

  return (
    <SceneContext.Provider value={editing}>
      <LightClickingContext.Provider value={lightClicking}>
      <NodePickingContext.Provider value={nodePicking}>
      <LayerSurfacesContext.Provider value={layerSurface}>
        <ConfigTreeContext.Provider value={tree}>
        <LayerMenuContext.Provider value={layerMenu}>
        <RegionVisibilityContext.Provider value={regionVisibility}>
        <ShapePosesContext.Provider value={shapePoses}>
        <div class="scene-editor" ref={root}>
        <HeaderTakeover tab="scenes">
          <div class="scene-file-header">
            <button type="button" class="scene-button" onClick={close}>
              ← Scenes
            </button>
            <span class="scene-file-name" title={name()}>
              {name()}
            </span>
            <div class="scene-file-actions">
              <button type="button" class="scene-icon-button scene-history-button" aria-label="Undo" title={`Undo (${MOD}+Z)`} disabled={!history.canUndo()} onClick={undo}>
                <icons.Undo2 aria-hidden="true" />
              </button>
              <button type="button" class="scene-icon-button scene-history-button" aria-label="Redo" title={`Redo (${MOD}+Shift+Z)`} disabled={!history.canRedo()} onClick={redo}>
                <icons.Redo2 aria-hidden="true" />
              </button>
              <button type="button" class="scene-button" disabled={!unsaved() || fileWork() !== null} onClick={save}>
                {fileWork() === 'saving' ? 'Saving…' : 'Save'}
              </button>
              <SceneNameButton label="Save as" title="Save scene as" action="Save" acting="Saving…" name={name()} disabled={fileWork() !== null} onName={saveAs} />
              <button type="button" class="scene-button" disabled={!unsaved() || fileWork() !== null} onClick={() => setConfirming('reset')}>
                Reset
              </button>
            </div>
            <Show when={unsaved()}>
              <span class="scene-note">Unsaved changes</span>
            </Show>
            <Show when={fileError()}>{(text) => <span class="scene-note scene-note-warn">{text()}</span>}</Show>
            <ShortcutsButton />
          </div>
        </HeaderTakeover>
        <Splitter.Root
          class="scenes-tab"
          orientation="vertical"
          panels={[
            { id: 'top', minSize: 15 },
            { id: 'timeline', minSize: 15 },
          ]}
          defaultSize={[74, 26]}
        >
          <Splitter.Panel id="top">
            <Splitter.Root
              panels={[
                { id: 'preview', minSize: 40 },
                { id: 'properties', minSize: 14 },
              ]}
              defaultSize={[79, 21]}
            >
              <Splitter.Panel id="preview" class="scene-output">
                <Show when={scene() !== null}>
                  <LayerTools place={newLayerAt().place} reason={newLayerAt().reason} drawing={drawing()} onDraw={setDrawing} />
                </Show>
                <div class="scene-output-main">
                  {/* how the preview is shown, and how dragging its handles works */}
                  <div class="scene-preview-bar">
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
                      <NumberBox min="1" aria-label="Number of lights" value={lights()} onNumber={setLights} />
                    </Show>
                    <Show when={viewDisplay(view()) !== null && !machineHasDisplay(viewDisplay(view())!)}>
                      <span class="scene-note" title="The machine has no display by this name, so it's previewed at this size">
                        Not on the machine
                      </span>
                      <NumberBox min="1" aria-label="Width" value={width()} onNumber={setWidth} />
                      <span>×</span>
                      <NumberBox min="1" aria-label="Height" value={height()} onNumber={setHeight} />
                    </Show>
                    {/* what's on show, which is also where the tools add a layer */}
                    <div class={`scene-view ${layerColorClass(viewColor(view()))}`}>
                      <LayerColorContext.Provider value={viewColor(view())}>
                        <Dropdown<View> label="Show and add layers to" options={views()} value={view()} onChange={setView} compact />
                      </LayerColorContext.Provider>
                    </div>
                    <Show when={plane()}>
                      <ViewAngleMenu
                        angle={viewAngle()}
                        onPick={(angle) => {
                          setViewAngle(angle)
                          setRefocus((count) => count + 1)
                        }}
                      />
                    </Show>
                  </div>
                  <Show when={error()}>{(text) => <pre class="scene-error">{text()}</pre>}</Show>
                  {/* both stay up, so the camera and the last render are still there on the way back */}
                  <div class="scene-machine" classList={{ stale: error() !== null, 'scene-choosing-node': nodePicking.choosing[0]() !== null }} hidden={!onMachine()} onContextMenu={handleMenu}>
                    <PlayfieldView
                      ledColor={lightColor}
                      displayFrame={previewDisplayFrame}
                      plane={plane}
                      focus={plane}
                      refocus={refocus}
                      focusAngle={viewAngle}
                      outline={outline}
                      onClickPlane={(point) => plane() && pick(point)}
                      onContextPlane={(point, x, y) => plane() && openMenuAt(point, x, y)}
                      onHoverPlane={(point) => plane() !== null && hoverAt(point)}
                      choosingLights={() => lightClicking[0]() !== null}
                      onClickLight={(name) => lightClicking[0]()?.(name)}
                      overlay={(space) => <Show when={plane()}>{handles(space, onMachine)}</Show>}
                    />
                  </div>
                  <div class="scene-dmd" classList={{ stale: error() !== null, 'scene-choosing-node': nodePicking.choosing[0]() !== null }} hidden={onMachine()}>
                    <div
                      class="scene-dmd-screen"
                      classList={{ 'scene-pointing': !onMachine() && hovered() !== null }}
                      onPointerMove={(e) => hoverAt(screenSpace.unproject(e.clientX, e.clientY))}
                      onPointerLeave={() => setHovered(null)}
                      onContextMenu={(e) => {
                        e.preventDefault()
                        if (!handleMenu(e)) openMenuAt(screenSpace.unproject(e.clientX, e.clientY), e.clientX, e.clientY)
                      }}
                      style={{ '--aspect': screenAspect(), 'min-height': `${view() === 'Strip' ? MIN_DRAWN_HEIGHT : 0}px` }}>
                      <canvas ref={canvas} onClick={(e) => pick(screenSpace.unproject(e.clientX, e.clientY))} />
                      <Show when={shownCanvas()?.area}>
                        {(area) => (
                          <svg viewBox={`${area().x} ${area().y} ${area().width} ${area().height}`} preserveAspectRatio="none" aria-hidden="true">
                            <For each={outline()}>{(loop) => (outlineOpen() ? <polyline class="scene-outline" points={loop.join(' ')} /> : <polygon points={loop.join(' ')} />)}</For>
                          </svg>
                        )}
                      </Show>
                      {handles(screenSpace, () => !onMachine())}
                    </div>
                  </div>
                </div>
              </Splitter.Panel>
              <Splitter.ResizeTrigger id="preview:properties" aria-label="Resize properties" />
              <Splitter.Panel id="properties">
                <aside class="scene-source" aria-label={showingCode() ? 'Scene as RON' : 'Properties'}>
                  {/* the RON is only shown for a file that can't be read, to fix it there, until it's back in the editor */}
                  <Show when={showingCode()}>
                  <div class="scene-source-header">
                    <span class="scene-source-title">RON</span>
                    <button
                      type="button"
                      class="scene-icon-button"
                      classList={{ active: showingCode() }}
                      aria-label={showingCode() ? 'Show the properties' : 'Show the code'}
                      aria-pressed={showingCode()}
                      title={showingCode() ? 'Show the properties' : 'Show the scene as RON'}
                      onClick={() => setShowingCode(!showingCode())}
                    >
                      <icons.Code aria-hidden="true" />
                    </button>
                  </div>
                  </Show>
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
                          <PropertiesPanel />
                        </div>
                      </Show>
                    </Show>
                  </div>
                </aside>
              </Splitter.Panel>
            </Splitter.Root>
          </Splitter.Panel>
          <Splitter.ResizeTrigger id="top:timeline" aria-label="Resize timeline" />
          <Splitter.Panel id="timeline">
            <Show when={scene() !== null}>
              <div class="scene-editing" inert={error() !== null}>
                <Timeline
                  playback={playback}
                  rendering={playback.playing() && sceneClipRendering()}
                  error={sceneClipError()}
                  graphs={drawn()?.graphs ?? []}
                  preview={{
                    get name() {
                      return reference()?.name ?? null
                    },
                    get error() {
                      return drawn()?.reference_error ?? null
                    },
                    get choices() {
                      return contentScenes().filter((scene) => scene !== name())
                    },
                    set: (chosen) => setReference(chosen === null ? null : { name: chosen }),
                  }}
                />
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
        </div>
        </ShapePosesContext.Provider>
        </RegionVisibilityContext.Provider>
        <layerMenu.View />
        </LayerMenuContext.Provider>
        </ConfigTreeContext.Provider>
      </LayerSurfacesContext.Provider>
      </NodePickingContext.Provider>
      </LightClickingContext.Provider>
    </SceneContext.Provider>
  )
}

const distance = ([ax, ay]: Vec2, [bx, by]: Vec2) => Math.hypot(ax - bx, ay - by)
