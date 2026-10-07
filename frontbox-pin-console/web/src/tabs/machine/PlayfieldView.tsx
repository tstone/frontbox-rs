import { createEffect, createMemo, createRoot, createSignal, For, onCleanup, onMount, Show, untrack } from 'solid-js'
import * as THREE from 'three'
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js'
import { ledName } from '../../lib/format'
import { LED_OFF, ledColor } from '../../lib/leds'
import { type ConsoleState, driverState, machine } from '../../state/console'
import { cancelPlacing, movedPositions, place, placing, type Position } from '../../state/placement'
import { isSwitchClosed, latched, pressed, pressSwitch, release, setSwitchClosed, toggleLatch } from '../../state/switchControl'
import type { PlaneView } from '../../types/generated/PlaneView'
import './PlayfieldView.css'

type Item = { key: string; name: string; kind: 'Switch' | 'Driver' | 'LED'; location: Position | null }
type Point = Item & { location: Position; moved: boolean }

type SceneData = {
  planes: PlaneView[]
  points: Point[]
  /** Every switch, driver and LED, located or not, by key */
  items: Map<string, Item>
  unlocated: number
  /** Changes only when something drawn changes, so a fresh `Init` (e.g. on game start) doesn't rebuild the scene */
  signature: string
}

/**
 * A shape per kind of hardware, so they can be told apart from any angle (sizes in inches). The driver ring lies flat
 * and is open in the middle, so a switch under a coil stays visible and clickable.
 */
function hardwareShapes(): Record<Item['kind'] | 'Motor', THREE.BufferGeometry> {
  return {
    LED: new THREE.SphereGeometry(0.3, 16, 12),
    Switch: new THREE.BoxGeometry(0.45, 0.45, 0.45),
    Driver: new THREE.TorusGeometry(0.38, 0.09, 8, 24),
    // not in the hardware definition yet; a motor can, standing upright
    Motor: new THREE.CylinderGeometry(0.26, 0.26, 0.4, 16).rotateX(Math.PI / 2),
  }
}
const PLANE_OPACITY = 0.14
// see-through enough that hardware on the plane stays easy to spot
const IMAGE_OPACITY = 0.5
/** How much larger than its shape a switch, driver or motor's outline is */
const OUTLINE_SCALE = 1.35
/** How far the pointer can move between press and release and still count as a click, in pixels */
const CLICK_SLOP = 4

function sceneData(state: ConsoleState): SceneData {
  const hw = state.hardware
  const planes = state.planes
  const all: Item[] = hw
    ? [
        ...Object.values(hw.switches.by_id).map((s) => ({ key: `switch:${s.id}`, name: s.name, kind: 'Switch' as const, location: s.location })),
        ...Object.values(hw.drivers.by_id).map((d) => ({ key: `driver:${d.id}`, name: d.name, kind: 'Driver' as const, location: d.location })),
        ...Object.values(hw.leds.by_name).map((l) => ({ key: `led:${l.name}`, name: ledName(l.name), kind: 'LED' as const, location: l.location })),
      ]
    : []
  const points: Point[] = []
  for (const item of all) {
    const moved = movedPositions[item.key]
    const location = moved ?? item.location
    if (location) points.push({ ...item, location, moved: moved !== undefined })
  }
  return {
    planes,
    points,
    items: new Map(all.map((item) => [item.key, item])),
    unlocated: all.length - points.length,
    signature: JSON.stringify([planes, points.map((p) => [p.key, p.location])]),
  }
}

/**
 * A group for things placed in cabinet coordinates. Those are left-handed (x right, y toward the front, z up, as
 * the player sees it) while three.js is right-handed, so drawing them as-is mirrors the machine, images included.
 * Flipping x puts it back.
 */
function cabinetGroup() {
  const group = new THREE.Group()
  group.scale.x = -1
  return group
}

type Layer = 'axes' | 'planes' | 'switches' | 'leds' | 'drivers'

const LAYERS: { layer: Layer; label: string; title: string }[] = [
  { layer: 'axes', label: 'Axes', title: "The X/Y/Z axes at the cabinet's origin (0, 0, 0)" },
  { layer: 'planes', label: 'Planes', title: "The machine's surfaces and their images" },
  { layer: 'switches', label: 'Switches', title: 'Switch dots' },
  { layer: 'leds', label: 'LEDs', title: 'LED dots' },
  { layer: 'drivers', label: 'Drivers', title: 'Driver dots' },
]

const layerOf = (kind: Item['kind']): Layer => (kind === 'Switch' ? 'switches' : kind === 'Driver' ? 'drivers' : 'leds')

// which layers are shown, remembered per browser
const LAYERS_KEY = 'frontbox-console.layers'
function readLayers(): Record<Layer, boolean> {
  const shown = { axes: true, planes: true, switches: true, leds: true, drivers: true }
  try {
    return { ...shown, ...JSON.parse(localStorage.getItem(LAYERS_KEY) ?? '{}') }
  } catch {
    return shown
  }
}
function writeLayers(shown: Record<Layer, boolean>) {
  try {
    localStorage.setItem(LAYERS_KEY, JSON.stringify(shown))
  } catch {
    // storage can be unavailable (private windows); the toggles still work for this session
  }
}

/** In inches */
const GIZMO_LENGTH = 8

/** Axes at the cabinet's origin: +x red, +y green, +z blue. Drawn on top so planes never hide it. */
function originGizmo() {
  const group = cabinetGroup()
  const axes = new THREE.AxesHelper(GIZMO_LENGTH)
  const axesMaterial = axes.material as THREE.LineBasicMaterial
  axesMaterial.depthTest = false
  axes.renderOrder = 3
  group.add(axes)

  const label = (text: string, color: string, position: THREE.Vector3) => {
    const canvas = document.createElement('canvas')
    canvas.width = canvas.height = 64
    const ctx = canvas.getContext('2d')!
    ctx.font = 'bold 44px system-ui, sans-serif'
    ctx.textAlign = 'center'
    ctx.textBaseline = 'middle'
    ctx.fillStyle = color
    ctx.fillText(text, 32, 34)
    const sprite = new THREE.Sprite(
      new THREE.SpriteMaterial({ map: new THREE.CanvasTexture(canvas), depthTest: false, transparent: true }),
    )
    sprite.position.copy(position)
    // the group mirrors x; mirror the label back so it reads the right way round
    sprite.scale.set(-2.4, 2.4, 1)
    sprite.renderOrder = 3
    group.add(sprite)
  }
  const tip = GIZMO_LENGTH + 1
  label('X', '#ff5a5a', new THREE.Vector3(tip, 0, 0))
  label('Y', '#5adf5a', new THREE.Vector3(0, tip, 0))
  label('Z', '#5a9cff', new THREE.Vector3(0, 0, tip))
  return group
}

/** A coordinate as a Rust f32 literal, to the thousandth of an inch: `32.1`, `0.0`, `-1.145` */
function rustFloat(n: number): string {
  const rounded = Math.round(n * 1000) / 1000 || 0 // `|| 0` folds -0 into 0
  const text = String(rounded)
  return text.includes('.') ? text : `${text}.0`
}

const rustVec3 = ([x, y, z]: Position) => `Vec3::new(${rustFloat(x)}, ${rustFloat(y)}, ${rustFloat(z)})`

/** A cabinet position in a plane's own coordinates: x/y across the plane, z its distance off it */
function relativeTo(plane: PlaneView, position: Position): Position {
  const local = new THREE.Vector3(...position)
    .sub(new THREE.Vector3(...plane.origin))
    .applyQuaternion(new THREE.Quaternion(...plane.rotation).invert())
  return [local.x, local.y, local.z]
}

/**
 * A position on a plane as code for the machine: `Vec3::new(..).relative_to(&planes::PLAYFIELD)` when the plane was
 * added with `console_plane!` (which knows its path), otherwise just the `Vec3::new(..)`.
 */
function relativeCode(plane: PlaneView, position: Position): string {
  const vec = rustVec3(relativeTo(plane, position))
  return plane.code ? `${vec}.relative_to(&${plane.code})` : vec
}

type Menu = { x: number; y: number; point: Point }
/** Overlapping switches to choose between, at the pointer */
type Chooser = { x: number; y: number; switches: Point[] }

/** How much larger a switch held from this browser is drawn */
const HELD_SCALE = 1.4

const switchId = (point: Point): number | null => (point.kind === 'Switch' ? Number(point.key.slice('switch:'.length)) : null)

/** A dot's hover label, including what a left click does to it */
function describe(point: Point): string {
  const id = switchId(point)
  if (id === null) return `${point.name} (${point.kind})`
  const action = latched[id] ? 'click to open' : 'hold to close, shift-click to latch'
  return `${point.name} (Switch): ${action}`
}

export default function PlayfieldView() {
  let container!: HTMLDivElement
  const data = createMemo(() => sceneData(machine), undefined, { equals: (a, b) => a.signature === b.signature })
  const [hovered, setHovered] = createSignal<{ name: string; x: number; y: number } | null>(null)
  const [menu, setMenu] = createSignal<Menu | null>(null)
  const [chooser, setChooser] = createSignal<Chooser | null>(null)
  // set once the scene is up; used by the chooser
  let startPress: (id: number, latch: boolean) => void = () => {}
  const [toast, setToast] = createSignal<string | null>(null)
  const [layers, setLayers] = createSignal(readLayers())
  const toggleLayer = (layer: Layer) => setLayers((shown) => ({ ...shown, [layer]: !shown[layer] }))
  const placingName = () => {
    const key = placing()
    return key ? (data().items.get(key)?.name ?? key) : null
  }

  let toastTimer: ReturnType<typeof setTimeout> | undefined
  function showToast(text: string) {
    clearTimeout(toastTimer)
    setToast(text)
    toastTimer = setTimeout(() => setToast(null), 2500)
  }

  async function copy(text: string) {
    setMenu(null)
    try {
      await navigator.clipboard.writeText(text)
      showToast(`Copied ${text}`)
    } catch {
      showToast(`Couldn't reach the clipboard: ${text}`)
    }
  }

  onMount(() => {
    const css = (name: string) => getComputedStyle(document.documentElement).getPropertyValue(name).trim()

    const renderer = new THREE.WebGLRenderer({ antialias: true })
    renderer.setPixelRatio(window.devicePixelRatio)
    container.prepend(renderer.domElement)
    const canvas = renderer.domElement

    const scene = new THREE.Scene()
    const camera = new THREE.PerspectiveCamera(40, 1, 0.5, 2000)
    // the cabinet's z is up
    camera.up.set(0, 0, 1)
    const controls = new OrbitControls(camera, renderer.domElement)

    // Draw at most once per screen refresh. Changes can arrive many times a frame (LED colors come in batches of
    // 24, so one LED frame is several messages); redrawing for each would queue up GPU work and fall behind.
    let frameRequested = false
    const render = () => {
      if (frameRequested) return
      frameRequested = true
      requestAnimationFrame(() => {
        frameRequested = false
        renderer.render(scene, camera)
      })
    }
    controls.addEventListener('change', render)

    // the cabinet's origin, so positions can be checked against the code. Kept out of `content` so it isn't rebuilt
    // or counted when framing the camera.
    const origin = originGizmo()
    scene.add(origin)

    // everything drawn for the current data, so it can be torn down on a rebuild
    let content = cabinetGroup()
    scene.add(content)
    const dots = new Map<THREE.Object3D, Point>()
    const planeMeshes: THREE.Mesh[] = []
    // planes' meshes and outlines, to show or hide together
    const planeObjects: THREE.Object3D[] = []
    const textures = new Map<string, THREE.Texture>()
    const textureLoader = new THREE.TextureLoader()
    const shapes = hardwareShapes()
    const shapeGeometries = new Set<THREE.BufferGeometry>(Object.values(shapes))
    const ghostMaterial = new THREE.MeshBasicMaterial({ transparent: true, opacity: 0.8 })
    // drawn on the back faces of a slightly larger copy of a shape, so it shows as a rim around it
    const outlineMaterial = new THREE.MeshBasicMaterial({ side: THREE.BackSide })
    // theme colors, read once per theme change rather than per dot
    const palette = { dot: '', moved: '', hover: '', active: '' }
    const planeMaterials: THREE.MeshBasicMaterial[] = []
    const lineMaterials: THREE.LineBasicMaterial[] = []
    let fitted = false
    let hoveredDot: THREE.Mesh | null = null
    // the per-LED color effects for the current dots, torn down on a rebuild
    let disposeDotEffects: (() => void) | undefined

    // the dot following the cursor while placing hardware
    const ghost = new THREE.Mesh(shapes.LED, ghostMaterial)
    ghost.renderOrder = 2
    ghost.visible = false
    let ghostPosition: Position | null = null

    /**
     * Each dot has its own material so it can show its own state. Hovered wins, then moved (not yet in the machine's
     * code), then live state: green for a closed switch or an active driver (on, or just fired), an LED's color (black
     * when it has none). Otherwise the default gray. A switch held from this browser is drawn larger, so a press shows
     * before the machine confirms it.
     */
    function paint(dot: THREE.Mesh) {
      const point = dots.get(dot)
      if (!point) return
      const id = switchId(point)
      const live =
        id !== null
          ? isSwitchClosed(id) && palette.active
          : point.kind === 'Driver'
            ? driverState(Number(point.key.slice('driver:'.length))) !== 'Off' && palette.active
            : point.kind === 'LED' && (ledColor(point.key.slice('led:'.length)) ?? LED_OFF)
      const color = dot === hoveredDot ? palette.hover : point.moved ? palette.moved : live || palette.dot
      ;(dot.material as THREE.MeshBasicMaterial).color.set(color)
      dot.scale.setScalar(id !== null && (pressed() === id || latched[id]) ? HELD_SCALE : 1)
    }

    function texture(url: string) {
      let tex = textures.get(url)
      if (!tex) {
        tex = textureLoader.load(url, render)
        // images are laid out with their top-left at the plane's origin, +y running down the image
        tex.flipY = false
        tex.colorSpace = THREE.SRGBColorSpace
        textures.set(url, tex)
      }
      return tex
    }

    function applyTheme() {
      scene.background = new THREE.Color(css('--bg'))
      planeMaterials.forEach((m) => !m.map && m.color.set(css('--plane')))
      lineMaterials.forEach((m) => m.color.set(css('--plane')))
      palette.dot = css('--hw-dot')
      palette.moved = css('--warn')
      palette.hover = css('--accent')
      palette.active = css('--ok')
      outlineMaterial.color.set(css('--hw-outline'))
      ghostMaterial.color.set(css('--accent'))
      untrack(() => dots.forEach((_, dot) => paint(dot as THREE.Mesh)))
      render()
    }

    function build({ planes, points }: SceneData) {
      scene.remove(content)
      content.traverse((obj) => {
        if (obj instanceof THREE.Mesh || obj instanceof THREE.LineSegments) {
          if (!shapeGeometries.has(obj.geometry)) obj.geometry.dispose()
        }
      })
      planeMaterials.forEach((m) => m.dispose())
      lineMaterials.forEach((m) => m.dispose())
      dots.forEach((_, dot) => ((dot as THREE.Mesh).material as THREE.Material).dispose())
      planeMaterials.length = 0
      lineMaterials.length = 0
      planeMeshes.length = 0
      planeObjects.length = 0
      dots.clear()
      hoveredDot = null
      content = cabinetGroup()

      for (const plane of planes) {
        const [w, h] = plane.extent
        const geometry = new THREE.PlaneGeometry(w, h).translate(w / 2, h / 2, 0)
        const material = new THREE.MeshBasicMaterial({
          transparent: true,
          opacity: plane.image ? IMAGE_OPACITY : PLANE_OPACITY,
          side: THREE.DoubleSide,
          // planes never hide the dots
          depthWrite: false,
          map: plane.image ? texture(plane.image) : null,
        })
        const line = new THREE.LineBasicMaterial()
        planeMaterials.push(material)
        lineMaterials.push(line)
        const mesh = new THREE.Mesh(geometry, material)
        planeMeshes.push(mesh)
        for (const obj of [mesh, new THREE.LineSegments(new THREE.EdgesGeometry(geometry), line)]) {
          planeObjects.push(obj)
          obj.position.set(...plane.origin)
          obj.quaternion.set(...plane.rotation)
          content.add(obj)
        }
      }

      for (const point of points) {
        const dot = new THREE.Mesh(shapes[point.kind], new THREE.MeshBasicMaterial())
        dot.position.set(...point.location)
        dot.renderOrder = 1
        // LEDs are sphere-shaped and lit up, so they stand out without one
        if (point.kind !== 'LED') {
          const outline = new THREE.Mesh(shapes[point.kind], outlineMaterial)
          outline.scale.setScalar(OUTLINE_SCALE)
          outline.renderOrder = 1
          // part of the dot for looks only: hovering and clicking still find the dot itself
          outline.raycast = () => {}
          dot.add(outline)
        }
        dots.set(dot, point)
        content.add(dot)
      }

      // one small effect per dot, so a change repaints only that dot
      disposeDotEffects?.()
      disposeDotEffects = createRoot((dispose) => {
        dots.forEach((_, dot) => {
          createEffect(() => {
            paint(dot as THREE.Mesh)
            render()
          })
        })
        return dispose
      })

      content.add(ghost)
      applyLayers()
      scene.add(content)
      // frame the machine the first time there's something to show, then leave the camera to the user
      if (!fitted && (planes.length > 0 || points.length > 0)) {
        fitCamera()
        fitted = true
      }
      applyTheme()
    }

    function applyLayers() {
      const shown = untrack(layers)
      origin.visible = shown.axes
      planeObjects.forEach((obj) => (obj.visible = shown.planes))
      dots.forEach((point, dot) => (dot.visible = shown[layerOf(point.kind)]))
      render()
    }

    function fitCamera() {
      const box = new THREE.Box3().setFromObject(content)
      const center = box.getCenter(new THREE.Vector3())
      const size = box.getSize(new THREE.Vector3()).length()
      controls.target.copy(center)
      // from the front-left, a little above, like standing at the machine
      camera.position.copy(center).add(new THREE.Vector3(0.35, 1.1, 0.75).normalize().multiplyScalar(size * 1.3))
      controls.update()
    }

    function resize() {
      const { width, height } = container.getBoundingClientRect()
      if (width === 0 || height === 0) return
      renderer.setSize(width, height, false)
      camera.aspect = width / height
      camera.updateProjectionMatrix()
      render()
    }
    const resizeObserver = new ResizeObserver(resize)
    resizeObserver.observe(container)

    const raycaster = new THREE.Raycaster()
    const pointer = new THREE.Vector2()
    function aim(e: MouseEvent) {
      const rect = renderer.domElement.getBoundingClientRect()
      pointer.set(((e.clientX - rect.left) / rect.width) * 2 - 1, -((e.clientY - rect.top) / rect.height) * 2 + 1)
      raycaster.setFromCamera(pointer, camera)
    }
    /** Every shown dot under the pointer, nearest first. Hidden dots can't be hovered or clicked. */
    const dotsUnder = (e: MouseEvent) => {
      aim(e)
      const shown = [...dots.keys()].filter((dot) => dot.visible)
      return raycaster.intersectObjects(shown).map((hit) => hit.object as THREE.Mesh)
    }
    const dotUnder = (e: MouseEvent) => dotsUnder(e)[0]
    /** The switches under the pointer: the only dots a left click acts on */
    const switchesUnder = (e: MouseEvent) =>
      dotsUnder(e)
        .map((dot) => dots.get(dot)!)
        .filter((point) => switchId(point) !== null)

    function setHoveredDot(dot: THREE.Mesh | null) {
      if (dot === hoveredDot) return
      const previous = hoveredDot
      hoveredDot = dot
      untrack(() => {
        if (previous) paint(previous)
        if (dot) paint(dot)
      })
      render()
    }

    function onPointerMove(e: PointerEvent) {
      if (placing()) {
        // slide the ghost across whichever plane is under the cursor
        aim(e)
        const hit = raycaster.intersectObjects(planeMeshes.filter((mesh) => mesh.visible))[0]
        ghost.visible = hit !== undefined
        if (hit) {
          const local = content.worldToLocal(hit.point.clone())
          ghost.position.copy(local)
          ghostPosition = [local.x, local.y, local.z]
        } else {
          ghostPosition = null
        }
        render()
        return
      }
      // everything under the pointer is listed, so overlapping hardware is visible before clicking
      const hits = dotsUnder(e)
      setHoveredDot(hits[0] ?? null)
      const label = hits.map((dot) => describe(dots.get(dot)!)).join(' · ')
      setHovered(hits.length > 0 ? { name: label, x: e.clientX, y: e.clientY } : null)
      canvas.style.cursor = hits.some((dot) => switchId(dots.get(dot)!) !== null) ? 'pointer' : ''
    }

    /**
     * A left press on a switch closes it until the press ends; with shift it latches closed (or opens, if latched).
     * Listened for on the container before it reaches the canvas, so a press on a switch never starts an orbit.
     * When switches overlap, a chooser opens instead.
     */
    function onPressStart(e: PointerEvent) {
      if (e.target !== canvas || e.button !== 0 || placing()) return
      const switches = switchesUnder(e)
      if (switches.length === 0) return
      e.stopPropagation()
      setMenu(null)
      if (switches.length > 1) {
        const rect = container.getBoundingClientRect()
        setHovered(null)
        setChooser({ x: e.clientX - rect.left, y: e.clientY - rect.top, switches })
        return
      }
      startPress(switchId(switches[0])!, e.shiftKey)
    }

    function onPointerLeave() {
      setHoveredDot(null)
      setHovered(null)
    }

    // a press that barely moves is a click; anything more is the camera being dragged
    let pressedAt: { x: number; y: number } | null = null
    const isClick = (e: MouseEvent) => pressedAt !== null && Math.hypot(e.clientX - pressedAt.x, e.clientY - pressedAt.y) < CLICK_SLOP
    function onPointerDown(e: PointerEvent) {
      pressedAt = { x: e.clientX, y: e.clientY }
      setMenu(null)
      setChooser(null)
    }
    function onPointerUp(e: PointerEvent) {
      const key = placing()
      if (key && e.button === 0 && isClick(e) && ghostPosition) {
        place(key, ghostPosition)
      }
    }

    function onContextMenu(e: MouseEvent) {
      e.preventDefault()
      // releasing a right-drag (panning) also fires this
      if (!isClick(e) || placing()) return
      const hit = dotUnder(e)
      const point = hit && dots.get(hit)
      if (!point) return
      const rect = container.getBoundingClientRect()
      setHovered(null)
      setMenu({ x: e.clientX - rect.left, y: e.clientY - rect.top, point })
    }

    // A press is released on pointer-up anywhere, a cancelled pointer, or the window losing focus
    const endPress = () => {
      release()
      window.removeEventListener('pointerup', endPress)
      window.removeEventListener('pointercancel', endPress)
      window.removeEventListener('blur', endPress)
    }
    startPress = (id, latch) => {
      // any click on a latched switch opens it
      if (latch || latched[id]) {
        toggleLatch(id)
        return
      }
      pressSwitch(id)
      window.addEventListener('pointerup', endPress)
      window.addEventListener('pointercancel', endPress)
      window.addEventListener('blur', endPress)
    }

    function onKeyDown(e: KeyboardEvent) {
      if (e.key !== 'Escape') return
      if (placing()) cancelPlacing()
      setMenu(null)
      setChooser(null)
    }

    container.addEventListener('pointerdown', onPressStart, { capture: true })
    canvas.addEventListener('pointermove', onPointerMove)
    canvas.addEventListener('pointerleave', onPointerLeave)
    canvas.addEventListener('pointerdown', onPointerDown)
    canvas.addEventListener('pointerup', onPointerUp)
    canvas.addEventListener('contextmenu', onContextMenu)
    window.addEventListener('keydown', onKeyDown)

    // entering and leaving placement
    createEffect(() => {
      const key = placing()
      // the ghost takes the shape of what's being placed
      const kind = key ? untrack(data).items.get(key)?.kind : undefined
      ghost.geometry = shapes[kind ?? 'LED']
      setHoveredDot(null)
      setHovered(null)
      ghost.visible = false
      ghostPosition = null
      canvas.style.cursor = key ? 'crosshair' : ''
      render()
    })

    // follow the theme, whether it comes from the OS or a data-theme attribute
    const darkQuery = matchMedia('(prefers-color-scheme: dark)')
    darkQuery.addEventListener('change', applyTheme)
    const themeObserver = new MutationObserver(applyTheme)
    themeObserver.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] })

    createEffect(() => build(data()))
    createEffect(() => {
      writeLayers(layers())
      applyLayers()
    })
    resize()

    onCleanup(() => {
      origin.traverse((obj) => {
        if (obj instanceof THREE.LineSegments || obj instanceof THREE.Sprite) {
          obj.geometry.dispose()
          const material = obj.material as THREE.Material & { map?: THREE.Texture | null }
          material.map?.dispose()
          material.dispose()
        }
      })
      window.removeEventListener('keydown', onKeyDown)
      container.removeEventListener('pointerdown', onPressStart, { capture: true })
      endPress()
      disposeDotEffects?.()
      resizeObserver.disconnect()
      themeObserver.disconnect()
      darkQuery.removeEventListener('change', applyTheme)
      controls.dispose()
      textures.forEach((t) => t.dispose())
      shapeGeometries.forEach((geometry) => geometry.dispose())
      dots.forEach((_, dot) => ((dot as THREE.Mesh).material as THREE.Material).dispose())
      ghostMaterial.dispose()
      outlineMaterial.dispose()
      renderer.dispose()
      clearTimeout(toastTimer)
    })
  })

  return (
    <div class="playfield-view" ref={container}>
      <div class="view-toggles" role="group" aria-label="Show in the 3D view">
        <For each={LAYERS}>
          {({ layer, label, title }) => (
            <button type="button" class="view-toggle" aria-pressed={layers()[layer]} title={title} onClick={() => toggleLayer(layer)}>
              {label}
            </button>
          )}
        </For>
      </div>
      <Show when={data().planes.length === 0 && data().points.length === 0}>
        <p class="notice empty">
          Nothing to draw yet. Give hardware a <code>location</code>, or add planes with <code>WebTracer::plane</code>.
        </p>
      </Show>
      <Show when={placingName()}>
        {(name) => (
          <p class="placing-hint">
            Placing <strong>{name()}</strong>: click a plane to drop it, <kbd>Esc</kbd> to cancel
          </p>
        )}
      </Show>
      <Show when={data().unlocated > 0 && data().points.length > 0}>
        <p class="unlocated">{data().unlocated} items have no location</p>
      </Show>
      <Show when={hovered()}>
        {(h) => (
          <div class="info-tip-content playfield-tip" style={{ left: `${h().x + 12}px`, top: `${h().y + 12}px` }}>
            {h().name}
          </div>
        )}
      </Show>
      <Show when={menu()}>
        {(m) => (
          <div class="dot-menu" role="menu" style={{ left: `${m().x}px`, top: `${m().y}px` }}>
            <p class="dot-menu-title">
              {m().point.name} ({m().point.kind})
            </p>
            {/* switch ids start at 0, so the id can't be the `when` itself */}
            <Show when={switchId(m().point) !== null}>
              <button
                type="button"
                role="menuitem"
                onClick={() => {
                  const id = switchId(m().point)!
                  setSwitchClosed(id, !isSwitchClosed(id))
                  setMenu(null)
                }}
              >
                {isSwitchClosed(switchId(m().point)!) ? 'Open switch' : 'Close switch'}
              </button>
            </Show>
            <button type="button" role="menuitem" onClick={() => copy(rustVec3(m().point.location))}>
              Copy Vec3
            </button>
            <For each={data().planes}>
              {(plane) => (
                <button type="button" role="menuitem" onClick={() => copy(relativeCode(plane, m().point.location))}>
                  Copy Vec3 relative to {plane.name}
                </button>
              )}
            </For>
          </div>
        )}
      </Show>
      <Show when={chooser()}>
        {(c) => (
          <div class="dot-menu" role="menu" aria-label="Choose a switch" style={{ left: `${c().x}px`, top: `${c().y}px` }}>
            <p class="dot-menu-title">Switches here</p>
            <For each={c().switches}>
              {(point) => (
                <button
                  type="button"
                  role="menuitem"
                  // pressing here acts like pressing the dot: held until release, shift to latch
                  onPointerDown={(e) => {
                    startPress(switchId(point)!, e.shiftKey)
                    setChooser(null)
                  }}
                >
                  {point.name}
                </button>
              )}
            </For>
          </div>
        )}
      </Show>
      <Show when={toast()}>
        {(text) => (
          <p class="playfield-toast" role="status">
            {text()}
          </p>
        )}
      </Show>
    </div>
  )
}
