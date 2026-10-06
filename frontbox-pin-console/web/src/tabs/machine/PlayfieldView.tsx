import { createEffect, createMemo, createRoot, createSignal, For, onCleanup, onMount, Show, untrack } from 'solid-js'
import * as THREE from 'three'
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js'
import { ledName } from '../../lib/format'
import { ledColor } from '../../lib/leds'
import { type ConsoleState, machine } from '../../state/console'
import { cancelPlacing, movedPositions, place, placing, type Position } from '../../state/placement'
import type { PlaneView } from '../../types/generated/PlaneView'
import './PlayfieldView.css'

type Item = { key: string; name: string; kind: 'Switch' | 'Driver' | 'LED'; location: Position | null }
type Point = Item & { location: Position; moved: boolean }

type SceneData = {
  planes: PlaneView[]
  points: Point[]
  /** Every switch, driver and LED, located or not, by key */
  items: Map<string, Item>
  /** How much hardware has no location, so isn't drawn */
  unlocated: number
  /** Changes only when something drawn changes, so a fresh `Init` (e.g. on game start) doesn't rebuild the scene */
  signature: string
}

/** Diameter of a hardware dot, in inches */
const DOT_SIZE = 0.8
/** Opacity of planes without and with an image */
const PLANE_OPACITY = 0.14
const IMAGE_OPACITY = 0.92
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

/** A position on a plane as code for the machine: the `Vec3::new(..)` relative to that plane */
function relativeCode(plane: PlaneView, position: Position): string {
  return rustVec3(relativeTo(plane, position))
}

type Menu = { x: number; y: number; point: Point }

/** The machine in 3D: its planes (with any images) and every located piece of hardware as a dot */
export default function PlayfieldView() {
  let container!: HTMLDivElement
  const data = createMemo(() => sceneData(machine), undefined, { equals: (a, b) => a.signature === b.signature })
  const [hovered, setHovered] = createSignal<{ name: string; x: number; y: number } | null>(null)
  const [menu, setMenu] = createSignal<Menu | null>(null)
  const [toast, setToast] = createSignal<string | null>(null)
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

    // everything drawn for the current data, so it can be torn down on a rebuild
    let content = cabinetGroup()
    scene.add(content)
    const dots = new Map<THREE.Object3D, Point>()
    const planeMeshes: THREE.Mesh[] = []
    const textures = new Map<string, THREE.Texture>()
    const textureLoader = new THREE.TextureLoader()
    const dotGeometry = new THREE.SphereGeometry(DOT_SIZE / 2, 16, 12)
    const ghostMaterial = new THREE.MeshBasicMaterial({ transparent: true, opacity: 0.8 })
    // theme colors, read once per theme change rather than per dot
    const palette = { dot: '', moved: '', hover: '' }
    const planeMaterials: THREE.MeshBasicMaterial[] = []
    const lineMaterials: THREE.LineBasicMaterial[] = []
    let fitted = false
    let hoveredDot: THREE.Mesh | null = null
    // the per-LED color effects for the current dots, torn down on a rebuild
    let disposeLedEffects: (() => void) | undefined

    // the dot following the cursor while placing hardware
    const ghost = new THREE.Mesh(dotGeometry, ghostMaterial)
    ghost.renderOrder = 2
    ghost.visible = false
    let ghostPosition: Position | null = null

    /**
     * Each dot has its own material so LEDs can show their own colors. Hovered wins, then moved (not yet in the
     * machine's code), then an LED's live color, then the default gray.
     */
    function paint(dot: THREE.Mesh) {
      const point = dots.get(dot)
      if (!point) return
      const color =
        dot === hoveredDot
          ? palette.hover
          : point.moved
            ? palette.moved
            : ((point.kind === 'LED' && ledColor(point.key.slice('led:'.length))) || palette.dot)
      ;(dot.material as THREE.MeshBasicMaterial).color.set(color)
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
      ghostMaterial.color.set(css('--accent'))
      untrack(() => dots.forEach((_, dot) => paint(dot as THREE.Mesh)))
      render()
    }

    function build({ planes, points }: SceneData) {
      scene.remove(content)
      content.traverse((obj) => {
        if (obj instanceof THREE.Mesh || obj instanceof THREE.LineSegments) {
          if (obj.geometry !== dotGeometry) obj.geometry.dispose()
        }
      })
      planeMaterials.forEach((m) => m.dispose())
      lineMaterials.forEach((m) => m.dispose())
      dots.forEach((_, dot) => ((dot as THREE.Mesh).material as THREE.Material).dispose())
      planeMaterials.length = 0
      lineMaterials.length = 0
      planeMeshes.length = 0
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
          obj.position.set(...plane.origin)
          obj.quaternion.set(...plane.rotation)
          content.add(obj)
        }
      }

      for (const point of points) {
        const dot = new THREE.Mesh(dotGeometry, new THREE.MeshBasicMaterial())
        dot.position.set(...point.location)
        dot.renderOrder = 1
        dots.set(dot, point)
        content.add(dot)
      }

      // one small effect per LED dot, so a color change recolors only that dot
      disposeLedEffects?.()
      disposeLedEffects = createRoot((dispose) => {
        dots.forEach((point, dot) => {
          if (point.kind !== 'LED') return
          createEffect(() => {
            paint(dot as THREE.Mesh)
            render()
          })
        })
        return dispose
      })

      content.add(ghost)
      scene.add(content)
      // frame the machine the first time there's something to show, then leave the camera to the user
      if (!fitted && (planes.length > 0 || points.length > 0)) {
        fitCamera()
        fitted = true
      }
      applyTheme()
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
    const dotUnder = (e: MouseEvent) => {
      aim(e)
      return raycaster.intersectObjects([...dots.keys()])[0]?.object as THREE.Mesh | undefined
    }

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
        const hit = raycaster.intersectObjects(planeMeshes)[0]
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
      const hit = dotUnder(e)
      setHoveredDot(hit ?? null)
      const point = hit && dots.get(hit)
      setHovered(point ? { name: `${point.name} (${point.kind})`, x: e.clientX, y: e.clientY } : null)
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

    function onKeyDown(e: KeyboardEvent) {
      if (e.key !== 'Escape') return
      if (placing()) cancelPlacing()
      setMenu(null)
    }

    const canvas = renderer.domElement
    canvas.addEventListener('pointermove', onPointerMove)
    canvas.addEventListener('pointerleave', onPointerLeave)
    canvas.addEventListener('pointerdown', onPointerDown)
    canvas.addEventListener('pointerup', onPointerUp)
    canvas.addEventListener('contextmenu', onContextMenu)
    window.addEventListener('keydown', onKeyDown)

    // entering and leaving placement
    createEffect(() => {
      const key = placing()
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
    resize()

    onCleanup(() => {
      window.removeEventListener('keydown', onKeyDown)
      disposeLedEffects?.()
      resizeObserver.disconnect()
      themeObserver.disconnect()
      darkQuery.removeEventListener('change', applyTheme)
      controls.dispose()
      textures.forEach((t) => t.dispose())
      dotGeometry.dispose()
      dots.forEach((_, dot) => ((dot as THREE.Mesh).material as THREE.Material).dispose())
      ghostMaterial.dispose()
      renderer.dispose()
      clearTimeout(toastTimer)
    })
  })

  return (
    <div class="playfield-view" ref={container}>
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
            <button type="button" role="menuitem" onClick={() => copy(rustVec3(m().point.location))}>
              Copy Vec3
            </button>
            <For each={data().planes}>
              {(plane) => (
                <button type="button" role="menuitem" onClick={() => copy(relativeCode(plane, m().point.location))}>
                  Copy relative to {plane.name}
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
