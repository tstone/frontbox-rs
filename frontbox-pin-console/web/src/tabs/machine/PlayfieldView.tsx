import { createEffect, createMemo, createSignal, onCleanup, onMount, Show } from 'solid-js'
import * as THREE from 'three'
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js'
import { ledName } from '../../lib/format'
import { type ConsoleState, machine } from '../../state/console'
import type { PlaneView } from '../../types/generated/PlaneView'
import './PlayfieldView.css'

type Point = { key: string; name: string; kind: 'Switch' | 'Driver' | 'LED'; location: [number, number, number] }

type SceneData = {
  planes: PlaneView[]
  points: Point[]
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

function sceneData(state: ConsoleState): SceneData {
  const hw = state.hardware
  const planes = state.planes
  const items = hw
    ? [
        ...Object.values(hw.switches.by_id).map((s) => ({ key: `switch:${s.id}`, name: s.name, kind: 'Switch' as const, location: s.location })),
        ...Object.values(hw.drivers.by_id).map((d) => ({ key: `driver:${d.id}`, name: d.name, kind: 'Driver' as const, location: d.location })),
        ...Object.values(hw.leds.by_name).map((l) => ({ key: `led:${l.name}`, name: ledName(l.name), kind: 'LED' as const, location: l.location })),
      ]
    : []
  const points = items.filter((item): item is Point => item.location !== null)
  return {
    planes,
    points,
    unlocated: items.length - points.length,
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

/** The machine in 3D: its planes (with any images) and every located piece of hardware as a dot */
export default function PlayfieldView() {
  let container!: HTMLDivElement
  const data = createMemo(() => sceneData(machine), undefined, { equals: (a, b) => a.signature === b.signature })
  const [hovered, setHovered] = createSignal<{ name: string; x: number; y: number } | null>(null)

  onMount(() => {
    const css = (name: string) => getComputedStyle(document.documentElement).getPropertyValue(name).trim()

    const renderer = new THREE.WebGLRenderer({ antialias: true })
    renderer.setPixelRatio(window.devicePixelRatio)
    container.appendChild(renderer.domElement)

    const scene = new THREE.Scene()
    const camera = new THREE.PerspectiveCamera(40, 1, 0.5, 2000)
    // the cabinet's z is up
    camera.up.set(0, 0, 1)
    const controls = new OrbitControls(camera, renderer.domElement)

    const render = () => renderer.render(scene, camera)
    controls.addEventListener('change', render)

    // everything drawn for the current data, so it can be torn down on a rebuild
    let content = cabinetGroup()
    scene.add(content)
    const dots = new Map<THREE.Object3D, Point>()
    const textures = new Map<string, THREE.Texture>()
    const textureLoader = new THREE.TextureLoader()
    const dotGeometry = new THREE.SphereGeometry(DOT_SIZE / 2, 16, 12)
    const dotMaterial = new THREE.MeshBasicMaterial()
    const hoverMaterial = new THREE.MeshBasicMaterial()
    const planeMaterials: THREE.MeshBasicMaterial[] = []
    const lineMaterials: THREE.LineBasicMaterial[] = []
    let fitted = false
    let hoveredDot: THREE.Mesh | null = null

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
      dotMaterial.color.set(css('--hw-dot'))
      hoverMaterial.color.set(css('--accent'))
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
      planeMaterials.length = 0
      lineMaterials.length = 0
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
        for (const obj of [new THREE.Mesh(geometry, material), new THREE.LineSegments(new THREE.EdgesGeometry(geometry), line)]) {
          obj.position.set(...plane.origin)
          obj.quaternion.set(...plane.rotation)
          content.add(obj)
        }
      }

      for (const point of points) {
        const dot = new THREE.Mesh(dotGeometry, dotMaterial)
        dot.position.set(...point.location)
        dot.renderOrder = 1
        dots.set(dot, point)
        content.add(dot)
      }

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
    function onPointerMove(e: PointerEvent) {
      const rect = renderer.domElement.getBoundingClientRect()
      pointer.set(((e.clientX - rect.left) / rect.width) * 2 - 1, -((e.clientY - rect.top) / rect.height) * 2 + 1)
      raycaster.setFromCamera(pointer, camera)
      const hit = raycaster.intersectObjects([...dots.keys()])[0]?.object as THREE.Mesh | undefined
      if (hit !== hoveredDot) {
        if (hoveredDot) hoveredDot.material = dotMaterial
        if (hit) hit.material = hoverMaterial
        hoveredDot = hit ?? null
        render()
      }
      const point = hit && dots.get(hit)
      setHovered(point ? { name: `${point.name} (${point.kind})`, x: e.clientX, y: e.clientY } : null)
    }
    function onPointerLeave() {
      if (hoveredDot) {
        hoveredDot.material = dotMaterial
        hoveredDot = null
        render()
      }
      setHovered(null)
    }
    renderer.domElement.addEventListener('pointermove', onPointerMove)
    renderer.domElement.addEventListener('pointerleave', onPointerLeave)

    // follow the theme, whether it comes from the OS or a data-theme attribute
    const darkQuery = matchMedia('(prefers-color-scheme: dark)')
    darkQuery.addEventListener('change', applyTheme)
    const themeObserver = new MutationObserver(applyTheme)
    themeObserver.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] })

    createEffect(() => build(data()))
    resize()

    onCleanup(() => {
      resizeObserver.disconnect()
      themeObserver.disconnect()
      darkQuery.removeEventListener('change', applyTheme)
      controls.dispose()
      textures.forEach((t) => t.dispose())
      dotGeometry.dispose()
      dotMaterial.dispose()
      hoverMaterial.dispose()
      renderer.dispose()
    })
  })

  return (
    <div class="playfield-view" ref={container}>
      <Show when={data().planes.length === 0 && data().points.length === 0}>
        <p class="notice empty">
          Nothing to draw yet. Give hardware a <code>location</code>, or add planes with <code>WebTracer::plane</code>.
        </p>
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
    </div>
  )
}
