import { Menu } from '@ark-ui/solid/menu'
import { batch, createMemo, createSignal, For, Show, useContext } from 'solid-js'
import { Dynamic } from 'solid-js/web'
import * as icons from '../../components/icons'
import type { Color } from '../../types/generated/Color'
import { ancestry } from './configNodes'
import { ConfigTreeContext, type ConfigTree } from './configTree'
import { ImageDialog } from './ImagePicker'
import { contentImageSize } from '../../state/contentImages'
import { shapeGenerators } from '../../state/shapeGenerators'
import { addModifier, drawnAspect, drawnPoints, drawnShape, modifierIcon, modifierOptions, newShape, newTraversal, type ModifierKind, type ModifiersNode, type ShapeKind, type TemplateKind } from './PropertiesPanel'
import ScenePortal, { LayerColorContext, layerColor, layerColorClass, type LayerColor } from './ScenePortal'
import { LayerSurfacesContext, SceneContext } from './SceneFields'
import { freshTraversalName, isOpen, newLayerPlace, newShapeSize, newStroke, traversalNames, withTiming, type LayerPlace, type LayerSurfaces, type Scene, type SceneLayer } from './sceneModel'
import type { DrawnBox, DrawnPoints } from './ShapeDrawing'
import { SoundDialog } from './SoundPicker'

/**
 * The column of tools down the side of the preview, each of which adds a layer of its kind and selects it. A layer goes
 * on `place`, the plane or display chosen for the preview, which is `null` with `reason` saying why when there's nowhere
 * to draw one. An image and a sound are chosen first, and a shape or text is drawn on the preview: picking one of those
 * hands it to `onDraw`, and `drawing` is the one being drawn. A sequence and a sound go where they always do, and a
 * sequence isn't offered on a display. A full layer has no region, so it covers all of what it's on. Each of the app's
 * shape generators is a tool too, when it has any. Each tool is in the color of the layers it adds. Below them, with
 * `behind` for a selected layer, is the tool that sends part of it behind a layer under it, which is drawn as a polygon
 * (`'Behind'` in `drawing`) and has a `reason` when it can't be. Then come the modifiers that can be added to what's selected.
 */
export default function LayerTools(props: { place: LayerPlace | null; reason: string | null; drawing: DrawnTool | 'Behind' | null; onDraw: (tool: DrawnTool | 'Behind' | null) => void; behind?: { reason: string | null } }) {
  const scene = useContext(SceneContext)!
  const surfaces = useContext(LayerSurfacesContext)!
  const tree = useContext(ConfigTreeContext)!
  const [choosing, setChoosing] = createSignal<'Image' | 'Sound' | null>(null)
  const onLights = () => props.place?.startsWith('Area:') === true
  const onDisplay = () => props.place?.startsWith('Display:') === true
  const drawnColor = (): LayerColor => (onDisplay() ? 'display' : 'lights')
  /** The tool a layer is being drawn with, which isn't the one for sending part of a layer behind another */
  const drawnTool = () => (props.drawing === 'Behind' ? null : props.drawing)
  const graphReason = () => props.reason ?? (onLights() ? null : "A graph is made of the machine's lights, so it's added on a plane")

  const add = (tool: Tool) => {
    if (isDrawn(tool)) return props.onDraw(props.drawing === tool ? null : tool)
    props.onDraw(null)
    if (tool === 'Image' || tool === 'Sound') return setChoosing(tool)
    const place = tool === 'Sequence' ? 'Sequence' : props.place
    if (place === null) return
    batch(() => {
      // a graph is there to be traversed, which needs a timing to be in
      if (tool === 'Graph') scene.set(withTiming(scene.get()))
      tree.addLayer(newLayer(tool, scene.get(), place, surfaces))
    })
  }

  /**
   * An image in a rectangle of its own shape in the middle of what it's on. For one over all of it, add a full layer and
   * pick its image. On a display that's a pixel for each of the display's, and on a plane, whose units are far bigger than an image's
   * pixels, it's the size a new shape is, stretched to fit.
   */
  const addImage = async (name: string) => {
    setChoosing(null)
    const place = props.place
    if (place === null) return
    const layer = newLayer('Image', scene.get(), place, surfaces)
    const surface = surfaces(layer)
    const [width, height] = surface ?? [1, 0]
    const pixels = await contentImageSize(name)
    const onDisplay = place.startsWith('Display:')
    const across = newShapeSize(surface)
    const aspect = pixels ? pixels[0] / Math.max(pixels[1], 1) : 1
    const size: [number, number] = onDisplay && pixels ? pixels : aspect >= 1 ? [across, across / aspect] : [across * aspect, across]
    tree.addLayer({
      ...layer,
      region: drawnShape('Rect', [width / 2, height / 2], size),
      fill: { base: { Image: { name, size: onDisplay ? 'Native' : undefined } } },
    })
  }

  const addSound = (name: string) => {
    setChoosing(null)
    tree.addSound(name)
  }

  const button = (tool: Tool, color: LayerColor, reason: string | null = props.reason) => (
    <button
      type="button"
      class={`scene-tool ${layerColorClass(color)}`}
      classList={{ active: props.drawing === tool }}
      aria-label={`Add ${toolLabel(tool).toLowerCase()} layer`}
      aria-pressed={isDrawn(tool) ? props.drawing === tool : undefined}
      title={reason ?? toolLabel(tool)}
      disabled={reason !== null}
      onClick={() => add(tool)}
    >
      <Dynamic component={TOOL_ICONS[tool]} aria-hidden="true" />
    </button>
  )

  return (
    <div class="scene-tools" role="toolbar" aria-orientation="vertical" aria-label="Add a layer">
      <Show when={!onDisplay()}>{button('Sequence', 'lights', null)}</Show>
      {button('Sound', 'sound', null)}
      <span class="scene-tools-divider" />
      {button('Image', drawnColor())}
      {button('Full', drawnColor())}
      <MenuTool
        label="Shapes"
        icon={icons.CircleSquare}
        groups={SHAPES.map((group) => group.map((tool) => ({ value: tool, label: TOOL_LABELS[tool], icon: TOOL_ICONS[tool] })))}
        color={drawnColor()}
        active={SHAPES.flat().some((tool) => tool === drawnTool())}
        reason={props.reason}
        onPick={add}
      />
      <MenuTool
        label="Lines"
        icon={icons.Curve}
        groups={LINES.map((group) => group.map((tool) => ({ value: tool, label: TOOL_LABELS[tool], icon: TOOL_ICONS[tool] })))}
        color={drawnColor()}
        active={LINES.flat().some((tool) => tool === drawnTool())}
        reason={props.reason}
        onPick={add}
      />
      <Show when={shapeGenerators().length > 0}>
        <MenuTool
          label="Generators"
          icon={icons.Code}
          groups={[shapeGenerators().map(({ name }) => ({ value: generatorTool(name), label: name, icon: icons.Code }))]}
          color={drawnColor()}
          active={drawnTool() !== null && isGenerator(drawnTool()!)}
          reason={props.reason}
          onPick={add}
        />
      </Show>
      {button('Graph', drawnColor(), graphReason())}
      {button('Text', drawnColor())}
      <ImageDialog open={choosing() === 'Image'} chosen="" onChoose={addImage} onClose={() => setChoosing(null)} />
      <SoundDialog open={choosing() === 'Sound'} chosen={scene.get().sounds.at(-1)?.sound ?? ''} volume={1} onChoose={addSound} onClose={() => setChoosing(null)} />
      <Show when={props.behind}>
        {(behind) => (
          <>
            <span class="scene-tools-divider" />
            <button
              type="button"
              class={`scene-tool ${layerColorClass(drawnColor())}`}
              classList={{ active: props.drawing === 'Behind' }}
              aria-label="Send part of the layer behind another"
              aria-pressed={props.drawing === 'Behind'}
              title={behind().reason ?? 'Send part behind: draw around the part of the selected layer that goes behind the layer under it, or with none, behind a lower z-index'}
              disabled={behind().reason !== null}
              onClick={() => props.onDraw(props.drawing === 'Behind' ? null : 'Behind')}
            >
              <icons.SendToBack aria-hidden="true" />
            </button>
          </>
        )}
      </Show>
      <ModifierTools />
    </div>
  )
}

/**
 * What a tool adds: a shape, an image, a layer over all of what it's on (with no region), a graph of lights, a sequence
 * of lights, a sound, or a shape made by one of the app's shape generators
 */
type Tool = Exclude<ShapeKind, 'None' | 'External'> | TemplateKind | 'Image' | 'Full' | 'Graph' | 'Sequence' | 'Sound' | GeneratorTool

/** The tool for the shape generator named after the colon */
type GeneratorTool = `External:${string}`

const generatorTool = (name: string): GeneratorTool => `External:${name}`
const isGenerator = (tool: Tool): tool is GeneratorTool => tool.startsWith('External:')
const generatorName = (tool: GeneratorTool) => tool.slice('External:'.length)

/** What a tool is called: a generator's by its name */
const toolLabel = (tool: Tool) => (isGenerator(tool) ? generatorName(tool) : TOOL_LABELS[tool])

/**
 * The width over the height of what a drawn tool draws, when it can only be one. A generator's outline is its own, so
 * it has none, and only where it's drawn counts.
 */
export const toolAspect = (tool: DrawnTool) => (isGenerator(tool) ? null : drawnAspect(tool))

/**
 * A tool whose layer is drawn on the preview: text or a shape, which a polygon or line is drawn a point at a time and
 * the rest as a box. A generator's shape is drawn as a box too, to say where it goes.
 */
export type DrawnTool = Exclude<Tool, 'Image' | 'Full' | 'Graph' | 'Sequence' | 'Sound'>

const isDrawn = (tool: Tool): tool is DrawnTool => !['Image', 'Full', 'Graph', 'Sequence', 'Sound'].includes(tool)

/**
 * A layer of a drawn tool's kind on `place`, filling the box that was drawn, or through the points of a polygon or line.
 * A click rather than a drag places one of the size a new shape has, around where it was. A generator's shape is its
 * own size either way, with its origin at the click or the middle of the box.
 */
export function drawnLayer(tool: DrawnTool, scene: Scene, place: LayerPlace, surfaces: LayerSurfaces, drawn: DrawnBox | DrawnPoints): SceneLayer {
  const layer = newLayer(tool, scene, place, surfaces)
  if (isGenerator(tool)) {
    return { ...layer, region: { base: { External: { generator: generatorName(tool) } }, transform: [{ Translate: { Fixed: 'points' in drawn ? [0, 0] : drawn.center } }] } }
  }
  const across = newShapeSize(surfaces(layer))
  const region =
    'points' in drawn
      ? drawnPoints(tool === 'Polyline' || tool === 'Bezier' ? tool : 'Polygon', drawn.points)
      : drawnShape(tool, drawn.center, drawn.size ?? [across, across])
  // with the stroke a new line comes with
  return { ...layer, region: { ...region, adjust: layer.region?.adjust } }
}

/** The shapes behind the shapes menu */
const SHAPES: Tool[][] = [
  ['Circle', 'Rect', 'Ellipse', 'Triangle'],
  ['Diamond', 'Polygon', 'Chevron', 'Star'],
  ['Pinwheel', 'Propeller'],
]

/**
 * The lines behind the lines menu: those drawn a point at a time, straight between them or curved through them, then
 * those drawn as a box: a spiral, and a wave in each of its forms
 */
const LINES: Tool[][] = [
  ['Polyline', 'Bezier'],
  ['Spiral', 'Wave', 'Zigzag', 'Serpentine'],
]

const TOOL_LABELS: Record<Tool, string> = {
  Circle: 'Circle',
  Ellipse: 'Ellipse',
  Rect: 'Rectangle',
  Triangle: 'Triangle',
  Diamond: 'Diamond',
  Polygon: 'Polygon',
  Polyline: 'Line',
  Bezier: 'Curve',
  Spiral: 'Spiral',
  Wave: 'Wave',
  Zigzag: 'Zigzag',
  Serpentine: 'Serpentine',
  Chevron: 'Chevron',
  Star: 'Star',
  Pinwheel: 'Pinwheel',
  Propeller: 'Propeller',
  Text: 'Text',
  Image: 'Image',
  Full: 'Full',
  Graph: 'Graph',
  Sequence: 'Sequence',
  Sound: 'Sound',
}

const TOOL_ICONS: Record<Tool, typeof icons.Circle> = {
  Circle: icons.Circle,
  Ellipse: icons.Egg,
  Rect: icons.Square,
  Triangle: icons.Triangle,
  Diamond: icons.Diamond,
  Polygon: icons.Pentagon,
  Polyline: icons.Activity,
  Bezier: icons.Curve,
  Spiral: icons.Spiral,
  Wave: icons.Wave,
  Zigzag: icons.Zigzag,
  Serpentine: icons.Serpentine,
  Chevron: icons.ChevronUp,
  Star: icons.Star,
  Pinwheel: icons.Fan,
  Propeller: icons.Plus,
  Text: icons.Type,
  Image: icons.ImageIcon,
  Full: icons.Scan,
  Graph: icons.Network,
  Sequence: icons.GripHorizontal,
  Sound: icons.AudioLines,
}

const WHITE: Color = [255, 255, 255, 255]
const CLEAR: Color = [0, 0, 0, 0]

type MenuItem<T> = { value: T; label: string; icon: typeof icons.Circle }

/**
 * A tool that opens out into a menu of the choices under it, beside the column, in groups set apart by a line. It's
 * marked while `active`, such as while one of its shapes is being drawn.
 */
function MenuTool<T extends string>(props: { label: string; icon: typeof icons.Circle; groups: MenuItem<T>[][]; color: LayerColor; active?: boolean; reason?: string | null; onPick: (value: T) => void }) {
  return (
    <LayerColorContext.Provider value={props.color}>
      <Menu.Root positioning={{ placement: 'right-start', gutter: 6 }} onSelect={(details) => props.onPick(details.value as T)}>
        <Menu.Trigger
          class={`scene-tool scene-tool-menu ${layerColorClass(props.color) ?? ''}`}
          classList={{ active: props.active === true }}
          aria-label={props.label}
          title={props.reason ?? props.label}
          disabled={(props.reason ?? null) !== null}
        >
          <Dynamic component={props.icon} aria-hidden="true" />
        </Menu.Trigger>
        <ScenePortal>
          <Menu.Positioner>
            <Menu.Content class="scene-dropdown-list scene-tool-list">
              <For each={props.groups}>
                {(group, index) => (
                  <>
                    <Show when={index() > 0}>
                      <Menu.Separator />
                    </Show>
                    <For each={group}>
                      {(item) => (
                        <Menu.Item value={item.value}>
                          <Dynamic component={item.icon} aria-hidden="true" />
                          <Menu.ItemText>{item.label}</Menu.ItemText>
                        </Menu.Item>
                      )}
                    </For>
                  </>
                )}
              </For>
            </Menu.Content>
          </Menu.Positioner>
        </ScenePortal>
      </Menu.Root>
    </LayerColorContext.Provider>
  )
}

/**
 * Buttons for the modifiers that can be added to what's selected: transforms and a stroke, then adjusts. The ones used
 * most have buttons of their own, and the rest of each are behind a menu. Each adds one and selects it. With nothing
 * selected that takes modifiers, there are none.
 */
function ModifierTools() {
  const tree = useContext(ConfigTreeContext)!
  const scene = useContext(SceneContext)!
  const surfaces = useContext(LayerSurfacesContext)!
  const targets = createMemo(() => modifierTargets(tree, tree.selected()))

  /** What each modifier offered for what's selected is called and adds, by its kind */
  const offered = createMemo(() => {
    const found = targets()
    const adds = new Map<ModifierKind, { label: string; add: () => void }>()
    if (!found) return adds
    const offer = (node: ModifiersNode | null, groups: ('transforms' | 'fillTransforms' | 'adjusts' | 'strokes')[]) => {
      if (!node) return
      const options = modifierOptions(node, found.traversals)
      for (const group of groups) {
        for (const { value, label } of options[group]) adds.set(value, { label, add: () => addModifier(tree, scene.get(), node, value, surfaces(found.layer), found.traversals) })
      }
    }
    offer(found.transforms, ['transforms', 'strokes'])
    offer(found.adjusts, ['fillTransforms', 'adjusts'])
    return adds
  })
  const sections = () =>
    MODIFIER_SECTIONS.map(({ label, icon, buttons, menu }) => ({
      label,
      icon,
      buttons: buttons.filter((kind) => offered().has(kind)),
      menu: menu.map((group) => group.filter((kind) => offered().has(kind))).filter((group) => group.length > 0),
    })).filter((section) => section.buttons.length > 0 || section.menu.length > 0)
  const add = (kind: ModifierKind) => offered().get(kind)?.add()
  const label = (kind: ModifierKind) => offered().get(kind)?.label ?? kind

  return (
    <Show when={targets()}>
      {(found) => (
        <div class={`scene-modifier-tools ${layerColorClass(layerColor(found().layer)) ?? ''}`} role="group" aria-label="Add a modifier">
          <For each={sections()}>
            {(section) => (
              <>
                <span class="scene-tools-divider" />
                <For each={section.buttons}>
                  {(kind) => (
                    <button type="button" class="scene-tool" aria-label={`Add ${label(kind).toLowerCase()}`} title={label(kind)} onClick={() => add(kind)}>
                      <Dynamic component={modifierIcon(kind)} aria-hidden="true" />
                    </button>
                  )}
                </For>
                <Show when={section.menu.length > 0}>
                  <MenuTool
                    label={section.label}
                    icon={section.icon}
                    groups={section.menu.map((group) => group.map((kind) => ({ value: kind, label: label(kind), icon: modifierIcon(kind) })))}
                    color={layerColor(found().layer)}
                    onPick={add}
                  />
                </Show>
              </>
            )}
          </For>
        </div>
      )}
    </Show>
  )
}

/** The modifiers that have buttons, which are those used most, and the menu the rest of them are in, in groups */
const MODIFIER_SECTIONS: { label: string; icon: typeof icons.Circle; buttons: ModifierKind[]; menu: ModifierKind[][] }[] = [
  { label: 'More transforms', icon: icons.WandSparkles, buttons: ['Translate', 'Rotate', 'Scale', 'Round'], menu: [['Mirror', 'Revolve', 'Bisect', 'Repeat'], ['Shift', 'Bulge'], ['Warp', 'Polar'], ['Fit'], ['Stroke', 'Feather', 'Shadow']] },
  { label: 'More adjustments', icon: icons.SlidersHorizontal, buttons: ['Opacity', 'Brightness'], menu: [['HueShift', 'Contrast']] },
]

/** A layer, by its id */
const MODIFIED = /^layers\/\d+$/

/**
 * Where modifiers go for what's selected, in the layer it's in. Transforms go in the nearest shape or fill that has
 * them, and on a layer or an adjust, in its region, or its fill where it has no region. Adjusts go where the transforms
 * do when that's a fill, or else in the layer's fill. `traversals` are the layer's, which a traversal's own props can't
 * use.
 */
function modifierTargets(tree: ConfigTree, id: string | null) {
  if (id === null) return null
  const chain = ancestry(id).reverse()
  const owner = chain.find((up) => MODIFIED.test(up))
  if (owner === undefined) return null
  const layerNode = tree.find(owner)
  if (layerNode?.kind !== 'layer') return null
  const modifiers = (at: string) => {
    const node = tree.find(`${at}/modifiers`)
    return node?.kind === 'modifiers' ? node : null
  }
  // an adjust only goes in a fill, so one that's selected, such as one just added, doesn't take transforms there
  const inAdjust = chain.some((up) => /\/fill\/adjust\/\d+$/.test(up))
  const nearest = inAdjust ? null : chain.slice(0, chain.indexOf(owner)).reduce<ModifiersNode | null>((found, up) => found ?? modifiers(up), null)
  const transforms = nearest ?? modifiers(`${owner}/region`) ?? modifiers(`${owner}/fill`)
  const adjusts = transforms?.adjusts ? transforms : modifiers(`${owner}/fill`)
  const layer = layerNode.lens.get()
  const inTraversal = chain.some((up) => /\/traversals\/\d+$/.test(up))
  return { layer, transforms, adjusts, traversals: inTraversal ? [] : traversalNames(layer) }
}

/**
 * A layer of a tool's kind on `place`, filled white and named for what it is. A shape is placed in the middle of what
 * it's on at a size that fits, and a line is stroked white over a clear fill, an image covers all of it, and a graph starts with a traversal out from its middle.
 */
function newLayer(tool: Exclude<Tool, 'Sound'>, scene: Scene, place: LayerPlace, surfaces: LayerSurfaces): SceneLayer {
  const layer: SceneLayer = { ...newLayerPlace(scene, place), name: freshLayerName(scene, toolLabel(tool)), fill: { base: { Solid: { Fixed: WHITE } } } }
  const surface = surfaces(layer)
  if (tool === 'Image' || tool === 'Full') return layer
  if (tool === 'Graph') return { ...layer, graph: { traversals: [newTraversal(freshTraversalName(layer, 'walk'), [], surface)] } }
  if (tool === 'Sequence') return layer
  // placed by whatever draws it
  if (isGenerator(tool)) return { ...layer, region: { base: { External: { generator: generatorName(tool) } } } }
  const region = newShape(tool, surface)
  // a line has no inside to fill, so it's drawn by a stroke, over a fill left clear
  if (isOpen(region.base)) return { ...layer, fill: { base: { Solid: { Fixed: CLEAR } } }, region: { ...region, adjust: [newStroke(surface)] } }
  return { ...layer, region }
}

/** `base`, or `base 2` and so on if a layer already has that name */
function freshLayerName(scene: Scene, base: string) {
  const taken = new Set(scene.layers.map((layer) => layer.name))
  if (!taken.has(base)) return base
  let n = 2
  while (taken.has(`${base} ${n}`)) n++
  return `${base} ${n}`
}
