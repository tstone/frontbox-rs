import { Tabs } from '@ark-ui/solid/tabs'
import { Menu } from '@ark-ui/solid/menu'
import { batch, createEffect, createMemo, createSignal, For, Index, Match, on, onCleanup, onMount, Show, Switch, useContext, type JSX } from 'solid-js'
import { Dynamic } from 'solid-js/web'
import * as icons from '../../components/icons'
import ScenePortal, { LayerColorContext, layerColor, layerColorClass, type LayerColor } from './ScenePortal'
import { ConfigTreeContext, shownInTree, type ConfigTree } from './configTree'
import { LayerMenuContext } from './LayerMenu'
import { ValueField } from './SceneFields'
import { NAMED_VALUE_LABELS, namedValueKind, ValueFields } from './ValueFields'
import { VariableFields, VariableSource } from './VariableFields'
import { contentImageInfo } from '../../state/contentImages'
import { contentFonts, fontGroup, fontLabel, refreshContentFonts } from '../../state/contentFonts'
import { imageGenerator, imageGenerators, refreshImageGenerators } from '../../state/imageGenerators'
import { refreshShapeGenerators, shapeGenerator, shapeGenerators } from '../../state/shapeGenerators'
import { contentSoundLength, contentSounds, refreshContentSounds } from '../../state/contentSounds'

import type { BezierPoint } from '../../types/generated/BezierPoint'
import type { Color } from '../../types/generated/Color'
import type { GradientStyle } from '../../types/generated/GradientStyle'
import type { FillNode } from '../../types/generated/FillNode'
import type { ImageInfo } from '../../types/generated/ImageInfo'
import type { ImageSize } from '../../types/generated/ImageSize'
import type { NodeRef } from '../../types/generated/NodeRef'
import type { CellStyle } from '../../types/generated/CellStyle'
import type { NoiseStyle } from '../../types/generated/NoiseStyle'
import type { NumberFormat } from '../../types/generated/NumberFormat'
import type { Prop } from '../../types/generated/Prop'
import type { ShapeNode } from '../../types/generated/ShapeNode'
import type { SoundLayer } from '../../types/generated/SoundLayer'
import type { Speed } from '../../types/generated/Speed'
import type { StrokeAlign } from '../../types/generated/StrokeAlign'
import type { TextAlign } from '../../types/generated/TextAlign'
import type { TextSource } from '../../types/generated/TextSource'
import type { Transform } from '../../types/generated/Transform'
import type { Traversal } from '../../types/generated/Traversal'
import type { TraversalAlgorithm } from '../../types/generated/TraversalAlgorithm'
import type { TextOverflow } from '../../types/generated/TextOverflow'
import type { VerticalAlign } from '../../types/generated/VerticalAlign'
import type { WaveStyle } from '../../types/generated/WaveStyle'
import {
  algorithmKind,
  ancestry,
  is,
  selectionOf,
  TRANSFORM_LABELS,
  WANDERING,
  type AlgorithmKind,
  type ConfigNode,
  type Fill,
  type ListPlace,
  type NodeKind,
  type PointsShape,
  type PropSpec,
  type Shape,
  type TransformKind,
  type Wandering,
} from './configNodes'
import { ImagePicker } from './ImagePicker'
import { SoundPicker } from './SoundPicker'
import { ANCHOR_SPOTS, anchorPresets, writer } from './ShapeHandles'
import { displayNames, LayerLights, LightsNote } from './LayerPlace'
import { evenlySpread, spreadStops } from './configTree'
import { randomSeed } from './curves'
import {
  Dropdown,
  EyeButton,
  FieldRow,
  LayerSurfacesContext,
  NodePickingContext,
  NumberBox,
  NumberField,
  PropField,
  propKind,
  PropSource,
  SOURCE_ICONS,
  RegionVisibilityContext,
  SceneContext,
  ShapePosesContext,
  ShowModulatorContext,
  SurfaceContext,
  TraversalsContext,
  LayerIndexContext,
} from './SceneFields'
import ModulatorList, { modulatorElementId } from './ModulatorList'
import {
  ADJUSTS,
  BULGE,
  BRANCHING,
  COPIES,
  COPY_KINDS,
  crispTextSize,
  FAN_ANGLE,
  FAN_COUNT,
  field,
  framesTrack,
  freshVariableName,
  freshTraversalName,
  GROWTH_DENSITY,
  imageLength,
  layerPlane,
  LEG_WIDTH,
  LEGS,
  newAdjust,
  newShapeSize,
  newStroke,
  isOpen,
  isNoFill,
  newFeather,
  newShadow,
  NOTCH,
  RUNS,
  TURNS,
  FLARE,
  orEmpty,
  RATE,
  reachRange,
  REVOLVE_ANGLE,
  shapePoints,
  snap,
  soundEnd,
  soundVolume,
  startingValue,
  VOLUME,
  TIME_STEP,
  traversalNames,
  traversalTarget,
  usedTraversals,
  variant,
  WANDER,
  RESHAPE_KINDS,
  SHIFT,
  WARP_AMOUNT,
  withDuration,
  withVariableValue,
  withTiming,
  withTraversalRenamed,
  type AdjustKind,
  type CopyKind,
  type ReshapeKind,
  type Lens,
  type Scene,
  type SceneLayer,
  type Surface,
  type NamedValueKind,
  NAMED_VALUE_KINDS,
  withNewValue,
  type SceneValue,
  type Vec2,
  layersUnder,
} from './sceneModel'

type FillKind = 'None' | 'Solid' | GradientStyle | 'Noise' | 'Cells' | 'Image' | 'Spritesheet' | 'External'
type NoiseFill = Extract<FillNode, { Noise: unknown }>['Noise']
type CellsFill = Extract<FillNode, { Cells: unknown }>['Cells']
type WarpTransform = Extract<Transform, { Warp: unknown }>
type ImageFill = Extract<FillNode, { Image: unknown }>['Image']
type SpritesheetFill = Extract<FillNode, { Spritesheet: unknown }>['Spritesheet']
type ExternalFill = Extract<FillNode, { External: unknown }>['External']
export type ShapeKind = 'None' | 'Circle' | 'Ellipse' | 'Rect' | 'Diamond' | 'Star' | 'Chevron' | 'Text' | 'Polygon' | 'Polyline' | 'Bezier' | 'Spiral' | 'Wave' | 'External'
/** A shape set up from the others, which is a shape of their kind once it's picked */
export type TemplateKind = 'Pinwheel' | 'Propeller' | 'Triangle' | 'Zigzag' | 'Serpentine'
/** What a layer's region can be: a shape, or on a light scene its graph */
type RegionKind = ShapeKind | 'Graph'
type Icon = typeof icons.Circle

/** How wide a triangle is, as a share of how tall it is, so that its sides are all the same length */
const TRIANGLE_WIDTH = 2 / Math.sqrt(3)
/** How wide a pinwheel's blades are where they're widest, as a share of how long they are */
const PINWHEEL_BLADE_WIDTH = 0.8
/** How wide a propeller's blades are, as a share of how long they are */
const PROPELLER_BLADE_WIDTH = 0.125
/** How many blades a template has */
const TEMPLATE_BLADES = 4

const WHITE: Color = [255, 255, 255, 255]
const BLACK: Color = [0, 0, 0, 255]

const FILL_OPTIONS: { value: FillKind; label: string }[] = [
  { value: 'None', label: 'None' },
  { value: 'Solid', label: 'Solid' },
  { value: 'Linear', label: 'Linear gradient' },
  { value: 'Radial', label: 'Radial gradient' },
  { value: 'Conical', label: 'Conical gradient' },
  { value: 'Noise', label: 'Noise' },
  { value: 'Cells', label: 'Cells' },
  { value: 'Image', label: 'Image' },
  { value: 'Spritesheet', label: 'Sprite sheet' },
]
/** An image made by one of the app's image generators, which is only offered when the app has any */
const GENERATED_FILL = { value: 'External', label: 'Generator' } as const
type ShapeOption = { value: RegionKind | TemplateKind; label: string; group?: string }
const SHAPES: ShapeOption[] = ([
  { value: 'Chevron', label: 'Chevron' },
  { value: 'Circle', label: 'Circle' },
  { value: 'Diamond', label: 'Diamond' },
  { value: 'Ellipse', label: 'Ellipse' },
  { value: 'Polygon', label: 'Polygon' },
  { value: 'Rect', label: 'Rectangle' },
  { value: 'Star', label: 'Star' },
  { value: 'Text', label: 'Text' },
] as ShapeOption[]).map((option) => ({ ...option, group: 'Shapes' }))
const TEMPLATES: ShapeOption[] = [
  { value: 'Pinwheel', label: 'Pinwheel', group: 'Templates' },
  { value: 'Propeller', label: 'Propeller', group: 'Templates' },
  { value: 'Triangle', label: 'Triangle', group: 'Templates' },
]
/** The waves set up in a style other than the smooth one they start in */
const WAVE_TEMPLATES: Partial<Record<TemplateKind, WaveStyle>> = { Zigzag: 'Zigzag', Serpentine: 'Square' }
/** What has no inside and is drawn by a stroke, including the waves set up in each style */
const LINES: ShapeOption[] = ([
  { value: 'Polyline', label: 'Line' },
  { value: 'Bezier', label: 'Curve' },
  { value: 'Spiral', label: 'Spiral' },
  { value: 'Wave', label: 'Wave' },
  { value: 'Zigzag', label: 'Zigzag' },
  { value: 'Serpentine', label: 'Serpentine' },
] as ShapeOption[]).map((option) => ({ ...option, group: 'Lines' }))
const isTemplate = (kind: RegionKind | TemplateKind): kind is TemplateKind =>
  TEMPLATES.some((option) => option.value === kind) || WAVE_TEMPLATES[kind as TemplateKind] !== undefined
const ALL: ShapeOption = { value: 'None', label: 'All' }
/** A shape made by one of the app's shape generators, which is only offered when the app has any */
const GENERATED: ShapeOption = { value: 'External', label: 'Generator', group: 'Code' }
/** "All", then the shapes by label, then the templates made of them, then lines, then the app's generators */
const SHAPE_OPTIONS: ShapeOption[] = [ALL, ...SHAPES, ...TEMPLATES, ...LINES, GENERATED]
const TRANSFORM_OPTIONS: { value: TransformKind; label: string }[] = (['Translate', 'Rotate', 'Scale', 'Repeat'] as const).map((value) => ({ value, label: TRANSFORM_LABELS[value] }))
/** Only for a shape, rather than a fill */
const SHAPE_TRANSFORM_OPTIONS: { value: TransformKind; label: string }[] = [...COPY_KINDS, ...RESHAPE_KINDS].map((value) => ({ value, label: TRANSFORM_LABELS[value] }))
/** Only for a layer on a light scene */
const GRAPH_REGION_OPTIONS: ShapeOption[] = [ALL, { value: 'Graph', label: 'Graph' }, ...SHAPES, ...TEMPLATES, ...LINES, GENERATED]
/** Only for a layer with traversals to fit to */
const FIT_OPTION: { value: TransformKind; label: string } = { value: 'Fit', label: TRANSFORM_LABELS.Fit }
/** Only for a fill */
const FILL_TRANSFORM_OPTIONS: { value: TransformKind; label: string }[] = (['Warp', 'Polar'] as const).map((value) => ({ value, label: TRANSFORM_LABELS[value] }))
const ALGORITHM_OPTIONS: { value: AlgorithmKind; label: string }[] = [
  { value: 'DepthFirst', label: 'Depth first' },
  { value: 'ShortestPath', label: 'Direct path (targeted)' },
  { value: 'AStar', label: 'Exploratory path (targeted)' },
  { value: 'FanOut', label: 'Fan out (targeted)' },
  { value: 'Growth', label: 'Growth (targeted)' },
  { value: 'Dijkstra', label: 'Fill by distance' },
  { value: 'BreadthFirst', label: 'Fill by steps' },
  { value: 'RandomWalk', label: 'Random walk' },
]
const NODE_OPTIONS: { value: 'Name' | 'Nearest'; label: string }[] = [
  { value: 'Name', label: 'Light' },
  { value: 'Nearest', label: 'Nearest to a position' },
]
/** Whether a layer is a mask, and which side of it the layers below are hidden on */
const MASK_OPTIONS: { value: 'Off' | 'Outside' | 'Inside'; label: string }[] = [
  { value: 'Off', label: 'Off' },
  { value: 'Outside', label: 'Hides outside' },
  { value: 'Inside', label: 'Hides inside' },
]
const ADJUST_OPTIONS = (Object.keys(ADJUSTS) as AdjustKind[]).map((value) => ({ value, label: ADJUSTS[value].label }))
/** Only for a shape, rather than a fill */
const SHAPE_ADJUST_OPTIONS: { value: 'Stroke' | 'Feather' | 'Shadow'; label: string }[] = [
  { value: 'Stroke', label: 'Stroke' },
  { value: 'Feather', label: 'Feather' },
  { value: 'Shadow', label: 'Shadow' },
]
type TextSourceKind = 'Fixed' | 'Variable' | 'Number'
const TEXT_SOURCE_OPTIONS: { value: TextSourceKind; label: string }[] = [
  { value: 'Fixed', label: 'Fixed' },
  { value: 'Variable', label: 'Variable' },
  { value: 'Number', label: 'Number' },
]
type NumberFormatKind = 'Grouped' | 'Plain' | 'Abbreviated'
const NUMBER_FORMAT_OPTIONS: { value: NumberFormatKind; label: string }[] = [
  { value: 'Grouped', label: 'Grouped (12,942)' },
  { value: 'Plain', label: 'Plain (12942)' },
  { value: 'Abbreviated', label: 'Abbreviated (12.9k)' },
]
/** A new number variable's default, long enough to show grouping and abbreviating */
const NUMBER_PREVIEW = 1_234_567
/** Characters an abbreviated number starts out fitting in */
const ABBREVIATED_LENGTH = 5
const TEXT_ALIGN_OPTIONS: { value: TextAlign; label: string }[] = [
  { value: 'Left', label: 'Left' },
  { value: 'Center', label: 'Center' },
  { value: 'Right', label: 'Right' },
]
const WAVE_STYLE_OPTIONS: { value: WaveStyle; label: string }[] = [
  { value: 'Smooth', label: 'Smooth' },
  { value: 'Zigzag', label: 'Zigzag' },
  { value: 'Square', label: 'Square (serpentine)' },
]
const VERTICAL_ALIGN_OPTIONS: { value: VerticalAlign; label: string }[] = [
  { value: 'Top', label: 'Top' },
  { value: 'Middle', label: 'Middle' },
  { value: 'Bottom', label: 'Bottom' },
]
const TEXT_OVERFLOW_OPTIONS: { value: TextOverflow; label: string }[] = [
  { value: 'Wrap', label: 'Wraps onto more lines' },
  { value: 'Ellipsis', label: 'Cut short with …' },
]
const NOISE_STYLE_OPTIONS: { value: NoiseStyle; label: string }[] = [
  { value: 'Smooth', label: 'Smooth' },
  { value: 'Turbulent', label: 'Turbulent' },
  { value: 'Ridged', label: 'Ridged' },
]
/** What each noise style looks like, under its picker */
const NOISE_STYLE_NOTES: Record<NoiseStyle, string> = {
  Smooth: 'Soft rolling blobs, for clouds, smoke, and fog.',
  Turbulent: 'Mostly low with soft bright billows, for fire and churning smoke.',
  Ridged: 'Thin bright veins, for lightning, arcs, and marble.',
}
const CELL_STYLE_OPTIONS: { value: CellStyle; label: string }[] = [
  { value: 'Edges', label: 'Edges' },
  { value: 'Value', label: 'Value' },
]
/** What each cell style looks like, under its picker */
const CELL_STYLE_NOTES: Record<CellStyle, string> = {
  Edges: 'Bright where cells meet and dark in their middles. Two stops close together near the end make thin lines, for cracks and scales.',
  Value: 'A flat random shade in each cell, for mosaic and stained glass.',
}
/** Most octaves a noise fill sums, as in the engine */
const MAX_OCTAVES = 8
const IMAGE_SIZE_OPTIONS: { value: ImageSize; label: string }[] = [
  { value: 'Stretch', label: 'Stretch to fit' },
  { value: 'Native', label: 'Native (a pixel per unit)' },
]
const STROKE_ALIGN_OPTIONS: { value: StrokeAlign; label: string }[] = [
  { value: 'Inside', label: 'Inside the edge' },
  { value: 'Centered', label: 'Centered on the edge' },
  { value: 'Outside', label: 'Outside the edge' },
]
const SPEED_OPTIONS: { value: SpeedKind; label: string }[] = [
  { value: 'Duration', label: 'Finish in a time' },
  { value: 'Rate', label: 'Rate' },
]

const SHAPE_ICONS: Record<RegionKind, Icon> = {
  None: icons.Scan,
  Circle: icons.Circle,
  Ellipse: icons.Egg,
  Rect: icons.Square,
  Polygon: icons.Pentagon,
  Diamond: icons.Diamond,
  Star: icons.Star,
  Chevron: icons.ChevronUp,
  Text: icons.Type,
  Polyline: icons.Activity,
  Bezier: icons.Curve,
  Spiral: icons.Spiral,
  Wave: icons.Wave,
  External: icons.Code,
  Graph: icons.Network,
}
const FILL_ICONS: Record<FillKind, Icon> = { None: icons.Ban, Solid: icons.PaintBucket, Linear: icons.Blend, Radial: icons.Blend, Conical: icons.Blend, Noise: icons.Cloud, Cells: icons.Hexagon, Image: icons.ImageIcon, Spritesheet: icons.ImageIcon, External: icons.Code }
/** A transform's or an adjust's icon says which kind it is, in place of where its value comes from */
const MODIFIER_ICONS: Record<TransformKind | AdjustKind | 'Feather' | 'Shadow', Icon> = {
  Translate: icons.Move,
  Rotate: icons.RotateCw,
  Scale: icons.Scaling,
  Fit: icons.Maximize2,
  Mirror: icons.SquareSplitHorizontal,
  Revolve: icons.RefreshCw,
  Bisect: icons.Scissors,
  Shift: icons.MoveVertical,
  Bulge: icons.Squircle,
  Round: icons.SquareRoundCorner,
  Warp: icons.Waves,
  Repeat: icons.Grid3x3,
  Polar: icons.Radar,
  HueShift: icons.Palette,
  Opacity: icons.Droplet,
  Brightness: icons.Sun,
  Contrast: icons.Contrast,
  Feather: icons.Feather,
  Shadow: icons.Sparkle,
}
const KIND_ICONS: Partial<Record<NodeKind, Icon>> = {
  place: icons.MapIcon,
  lights: icons.Lightbulb,
  z: icons.Layers2,
  mask: icons.SquareDashed,
  behind: icons.SendToBack,
  stroke: icons.PenLine,
  fit: icons.Maximize2,
  points: icons.Waypoints,
  'bezier-point': icons.Dot,
  traversals: icons.Route,
  traversal: icons.Footprints,
  'node-ref': icons.MapPin,
  variable: icons.Variable,
  value: icons.Palette,
}

type SpeedKind = 'Rate' | 'Duration'

/**
 * The label of a node's control in its properties, for each kind that has one. `null` is for a button, which says what
 * it does itself. A light by name or position is labeled with which end of the traversal it is.
 */
const CONTROL_LABELS: Partial<Record<NodeKind, string | null>> = {
  place: 'Draws on',
  lights: 'Lights',
  z: 'Z-index',
  mask: 'Mask',
  behind: 'Goes behind',
  region: 'Region',
  fill: 'Fill',
  fit: 'Fit to',
  variable: 'Source',
  stops: null,
  points: null,
  traversals: null,
}

/** The kinds of node whose control is small enough to go beside its label */
const INLINE_KINDS: NodeKind[] = ['lights', 'z', 'mask', 'behind', 'fit', 'fill', 'region']

/** The kinds of node under a layer that are its settings, shown in its properties */
const SETTING_KINDS: NodeKind[] = ['lights', 'z', 'mask', 'behind']

/** The sidebar's sections, which open and close on their own */
type SidebarSection = 'properties' | 'variables' | 'modulators'

/**
 * The sidebar, in tabs: the properties of what's selected, in the tree on the timeline or by a click in the preview, the
 * scene's variables and its named values, and its modulators. Selecting something goes to its properties.
 */
export default function PropertiesPanel() {
  const tree = useContext(ConfigTreeContext)!
  const graphNodes = useGraphNodes()
  onMount(refreshContentSounds)
  onMount(refreshContentFonts)
  onMount(refreshShapeGenerators)
  onMount(refreshImageGenerators)
  const [tab, setTab] = createSignal<SidebarSection>('properties')
  createEffect(
    on(
      tree.selected,
      (id) => {
        if (id !== null) setTab('properties')
      },
      { defer: true },
    ),
  )
  const section = (id: 'variables' | 'values') => tree.nodes().find((node) => node.id === id)?.children ?? []
  const scene = useContext(SceneContext)!
  const modulatorCount = () => Object.keys(scene.get().modulators ?? {}).length
  const showModulator = (name: string) => {
    setTab('modulators')
    // once the tab's content is showing
    requestAnimationFrame(() => document.getElementById(modulatorElementId(name))?.scrollIntoView({ block: 'nearest' }))
  }

  return (
    <div class="scene-config scene-properties">
      <ShowModulatorContext.Provider value={showModulator}>
      <Tabs.Root class="scene-sidebar-tabs" value={tab()} onValueChange={(details) => setTab(details.value as SidebarSection)}>
        <Tabs.List class="scene-sidebar-tab-list">
          <SidebarTab value="properties" label="Properties" />
          <SidebarTab value="variables" label="Variables" count={section('variables').length + section('values').length} />
          <SidebarTab value="modulators" label="Modulators" count={modulatorCount()} />
        </Tabs.List>
        <SidebarItem value="properties">
          <Show when={tree.selected()} keyed fallback={<NothingSelected />}>
            {(id) => <SelectedProperties id={id} />}
          </Show>
        </SidebarItem>
        {/* the variables, which the game sets, then the named values they can be pointed at */}
        <SidebarItem value="variables">
          <section class="scene-sidebar-group" aria-label="Variables">
            <h4>Variables</h4>
            <Show when={section('variables').length > 0} fallback={<p class="scene-note">Set a prop to Variable to add one.</p>}>
              <NamedEntries nodes={section('variables')} />
            </Show>
          </section>
          <section class="scene-sidebar-group" aria-label="Values">
            <h4>Values</h4>
            <Show when={section('values').length === 0}>
              <p class="scene-note">Add a value, then point a variable at it from its default or from the game.</p>
            </Show>
            <NamedEntries nodes={section('values')} />
            <AddValue />
          </section>
        </SidebarItem>
        <SidebarItem value="modulators">
          <ModulatorList />
        </SidebarItem>
      </Tabs.Root>
      </ShowModulatorContext.Provider>
      <datalist id={LIGHT_NAMES}>
        <For each={graphNodes()}>{(name) => <option value={name} />}</For>
      </datalist>
    </div>
  )
}

/** The tab of one of the sidebar's sections, with how many are in it */
function SidebarTab(props: { value: SidebarSection; label: string; count?: number }) {
  return (
    <Tabs.Trigger value={props.value} class="scene-sidebar-tab">
      {props.label}
      <Show when={props.count}>{(count) => <span class="scene-sidebar-count">{count()}</span>}</Show>
    </Tabs.Trigger>
  )
}

/** What's in one of the sidebar's sections */
function SidebarItem(props: { value: SidebarSection; children: JSX.Element }) {
  return (
    <Tabs.Content value={props.value} class="scene-sidebar-content">
      {props.children}
    </Tabs.Content>
  )
}

/** Each of the scene's variables or named values, all of them editable at once */
function NamedEntries(props: { nodes: ConfigNode[] }) {
  return (
    // by position, with each remade once a different one is there, so an edit doesn't tear down the field it was made in
    <Index each={props.nodes}>
      {(node) => (
        <Show when={node().id} keyed>
          {(id) => (
            <NodeScope id={id}>
              <Switch>
                <Match when={is(node(), 'variable')}>
                  {(variable) => (
                    <>
                      <div class="scene-properties-title">
                        <NodeIcon node={variable()} />
                        <h3>{variable().name}</h3>
                      </div>
                      <NodeSetting node={variable()} />
                      <VariableFields name={variable().name} />
                    </>
                  )}
                </Match>
                <Match when={is(node(), 'value')}>{(value) => <ValueFields name={value().name} />}</Match>
              </Switch>
            </NodeScope>
          )}
        </Show>
      )}
    </Index>
  )
}

function NothingSelected() {
  return (
    <>
      <p class="scene-note">Select a layer in the timeline, or click one in the preview, to see its properties here.</p>
      <LightsNote />
    </>
  )
}

/** The selected node, looked up again as the scene changes so that its fields stay up while they're edited */
function SelectedProperties(props: { id: string }) {
  const tree = useContext(ConfigTreeContext)!
  return (
    <Show when={tree.find(props.id)}>
      {(node) => (
        <NodeScope id={props.id}>
          <NodeProperties node={node()} />
        </NodeScope>
      )}
    </Show>
  )
}

/**
 * What a node's fields are measured against and can respond to, which comes of the layer it's in, and the accent color
 * of a sound's. A traversal's own fields don't respond to visits, as visits are what's being set up there.
 */
function NodeScope(props: { id: string; children: JSX.Element }) {
  const tree = useContext(ConfigTreeContext)!
  const surfaces = useContext(LayerSurfacesContext)!
  // the same for as long as the node is shown, as it's shown by its id
  const chain = ancestry(props.id)
  const layerId = chain.find((id) => /^layers\/\d+$/.test(id))
  const inTraversal = chain.some((id) => /\/traversals\/\d+$/.test(id))
  const inSound = chain.some((id) => /^sounds\/\d+$/.test(id))
  const layer = () => {
    const node = layerId === undefined ? undefined : tree.find(layerId)
    return node?.kind === 'layer' ? node.lens.get() : undefined
  }
  const color = (): LayerColor => (inSound ? 'sound' : layer() ? layerColor(layer()!) : undefined)
  const index = layerId === undefined ? null : Number(layerId.split('/')[1])
  return (
    <SurfaceContext.Provider value={() => (layer() ? surfaces(layer()!) : null)}>
      <TraversalsContext.Provider value={() => (inTraversal || !layer() ? [] : traversalNames(layer()!))}>
        <LayerIndexContext.Provider value={() => index}>
          <LayerColorContext.Provider value={color()}>
            <div class={`scene-properties-node ${layerColorClass(color()) ?? ''}`}>{props.children}</div>
          </LayerColorContext.Provider>
        </LayerIndexContext.Provider>
      </TraversalsContext.Provider>
    </SurfaceContext.Provider>
  )
}

/**
 * A node's heading, its control and fields, a layer's settings, then the values under it and a button for adding a
 * modifier. A layer is one thing with its shape and its fill, which are parts of its properties rather than nodes of
 * their own.
 */
function NodeProperties(props: { node: ConfigNode }) {
  return (
    <Show when={props.node.kind === 'layer'} fallback={<OwnProperties node={props.node} />}>
      <PartsProperties node={props.node} />
    </Show>
  )
}

/** A layer's settings and values, then its shape and its fill, or for a line, each of its strokes with what it's drawn in */
function PartsProperties(props: { node: ConfigNode }) {
  const children = () => props.node.children ?? []
  const settings = () => children().filter((child) => SETTING_KINDS.includes(child.kind))
  const values = () => children().filter((child) => child.kind === 'prop' || child.kind === 'node-ref')
  const fill = () => children().find((child) => child.kind === 'fill')
  // a region that's everything, such as an image's, has nothing to set
  const shape = () => {
    const found = children().find((child) => child.kind === 'region')
    const region = found && is(found, 'region')
    return region && (region.value !== undefined || region.layer?.get().graph) ? region : undefined
  }
  const lineStrokes = () => (shape()?.children ?? []).filter((child) => child.kind === 'stroke' && child.ofLine)
  return (
    <>
      <NodeHeading node={props.node} />
      <NodeSetting node={props.node} />
      <Index each={settings()}>{(child) => <NodeSetting node={child()} />}</Index>
      <Show when={props.node.fields === true}>
        <NodeFields node={props.node} />
      </Show>
      <Index each={values()}>
        {(child) => (
          <Show when={child().id} keyed>
            {(_id) => <ValueNode node={child()} />}
          </Show>
        )}
      </Index>
      <Show when={shape()}>
        {(region) => <PartSection node={region()} title={region().layer?.get().graph ? 'Graph' : (REGION_LABELS.get(regionKind(region().lens, region().layer)) ?? 'Shape')} />}
      </Show>
      <Show when={fill()}>{(node) => <PartSection node={node()} title="Fill" />}</Show>
      {/* with what it's drawn in, which can be none for a line that isn't shown */}
      <Index each={lineStrokes()}>
        {(stroke, index) => (
          <PartSection
            node={stroke()}
            then={(stroke().children ?? []).find((child) => child.kind === 'fill')}
            title={lineStrokes().length > 1 ? `Stroke ${index + 1}` : 'Stroke'}
          />
        )}
      </Index>
    </>
  )
}

/**
 * A layer's shape or fill, as a part of its properties: a button in its heading for adding a modifier to it, then its
 * fields and values, and a gradient's colors and a polygon's points in place. Its modifiers aren't listed, as they have
 * rows and properties of their own. A part can be followed by another `then` under the same heading, such as a line's
 * stroke by the stroke's fill, which is then what the button adds a modifier to.
 */
function PartSection(props: { node: ConfigNode; title: string; then?: ConfigNode }) {
  const modifiers = () => [props.then, props.node].flatMap((node) => node?.children ?? []).find((child) => child.kind === 'modifiers') as ModifiersNode | undefined
  return (
    <section class="scene-part" aria-label={props.title}>
      <header class="scene-part-heading">
        <h4>{props.title}</h4>
        <Show when={modifiers()}>
          {(node) => (
            <AddModifier node={node()} class="scene-part-add" label={`Add a modifier to the ${props.title.toLowerCase()}`}>
              <icons.Plus aria-hidden="true" />
            </AddModifier>
          )}
        </Show>
      </header>
      <PartBody node={props.node} />
      <Show when={props.then}>{(then) => <PartBody node={then()} />}</Show>
    </section>
  )
}

/** What's under a part's heading */
function PartBody(props: { node: ConfigNode }) {
  const children = () => props.node.children ?? []
  const values = () => children().filter((child) => (child.kind === 'prop' && (child.modifier === undefined || child.placement)) || child.kind === 'node-ref')
  const stops = () => children().find((child) => child.kind === 'stops')
  const points = () => children().find((child) => child.kind === 'points')
  const traversals = () => children().find((child) => child.kind === 'traversals')
  const hasFields = () => props.node.fields === true
  return (
    <>
      <Show when={is(props.node, 'fill')}>
        <FieldRow inline label="Type">
          <NodeControl node={props.node} />
        </FieldRow>
      </Show>
      <Show when={hasFields()}>
        <NodeFields node={props.node} />
      </Show>
      <Index each={values()}>
        {(child) => (
          <Show when={child().id} keyed>
            {(_id) => <ValueNode node={child()} />}
          </Show>
        )}
      </Index>
      <Show when={stops()}>
        {(list) => (
          <PartList node={list()} label="Colors" />
        )}
      </Show>
      <Show when={points()}>{(list) => <PartList node={list()} label="Points" />}</Show>
      <Show when={traversals()}>{(list) => <NodeSetting node={list()} />}</Show>
    </>
  )
}

/** A gradient's stops or a shape's points, each edited in place, with the button for adding another */
function PartList(props: { node: ConfigNode; label: string }) {
  const items = () => (props.node.children ?? []).filter((child) => child.kind === 'prop')
  return (
    <div class="scene-part-list">
      <div class="scene-part-list-heading">
        <span>{props.label}</span>
        <NodeControl node={props.node} />
      </div>
      <Index each={items()}>
        {(child) => (
          <Show when={child().id} keyed>
            {(_id) => <ValueNode node={child()} />}
          </Show>
        )}
      </Index>
    </div>
  )
}

/** A node's heading, its control and fields, then the values under it and a button for adding a modifier */
function OwnProperties(props: { node: ConfigNode }) {
  const children = () => props.node.children ?? []
  const settings = () => children().filter((child) => SETTING_KINDS.includes(child.kind))
  const values = () => children().filter((child) => child.kind === 'prop' || child.kind === 'node-ref')
  const modifiers = () => children().find((child) => child.kind === 'modifiers') as Extract<ConfigNode, { kind: 'modifiers' }> | undefined
  // a stroke's fill, set as a layer's is
  const fill = () => children().find((child) => child.kind === 'fill')
  const hasFields = () => props.node.kind === 'prop' || props.node.fields === true

  return (
    <>
      <NodeHeading node={props.node} />
      {/* a region's own heading already says what shape it is */}
      <Show when={props.node.kind !== 'region'}>
        <NodeSetting node={props.node} />
      </Show>
      <Index each={settings()}>{(child) => <NodeSetting node={child()} />}</Index>
      <Show when={hasFields()}>
        <NodeFields node={props.node} />
      </Show>
      {/* by position, with each remade once a different node is there, so an edit doesn't tear down the field it was made in */}
      <Index each={values()}>
        {(child) => (
          <Show when={child().id} keyed>
            {(_id) => <ValueNode node={child()} />}
          </Show>
        )}
      </Index>
      <Show when={fill()}>{(node) => <PartSection node={node()} title="Fill" />}</Show>
      <Show when={modifiers()}>{(node) => <AddModifier node={node()} />}</Show>
    </>
  )
}

/** What a node is, the nodes it's under, which can be selected, and its buttons */
function NodeHeading(props: { node: ConfigNode }) {
  const tree = useContext(ConfigTreeContext)!
  // only what has a row, as a layer's shape and fill are parts of it
  const trail = () =>
    ancestry(props.node.id)
      .slice(0, -1)
      .flatMap((id) => (id === 'scene' ? [] : (tree.find(id) ?? [])))
      .filter(shownInTree)
  return (
    <header class="scene-properties-heading">
      <Show when={trail().length > 0}>
        <nav class="scene-properties-trail" aria-label="What this is in">
          <For each={trail()}>
            {(node) => (
              <>
                <button type="button" title={`Select ${node.label}`} onClick={() => tree.select(node.id)}>
                  {node.label}
                </button>
                <icons.ChevronRight aria-hidden="true" />
              </>
            )}
          </For>
        </nav>
      </Show>
      <div class="scene-properties-title">
        {/* a prop's icon picks where it takes its value from */}
        <Show when={is(props.node, 'prop')} fallback={<NodeIcon node={props.node} />}>
          {(prop) => <PropSource label={prop().label} {...propFieldSpec(prop())} />}
        </Show>
        <h3>{props.node.label}</h3>
        <NodeSummary node={props.node} />
        <span class="scene-properties-tools">
          <Show when={is(props.node, 'layer')}>
            {(layer) => <EyeButton label={layer().label} hidden={layer().lens.get().hidden === true} onChange={(hidden) => field(layer().lens, 'hidden').set(hidden || undefined)} />}
          </Show>
          <Show when={!props.node.placement && props.node.place}>{(place) => <RemoveButton node={props.node} place={place()} />}</Show>
        </span>
      </div>
    </header>
  )
}

/** The label a node's control has in its properties: `null` for one that needs none, and `undefined` for a node with no control */
function controlLabel(node: ConfigNode): string | null | undefined {
  if (node.kind === 'node-ref') return node.label
  return CONTROL_LABELS[node.kind]
}

/** A node's control, beside its label when it's small enough to fit there and under it otherwise */
function NodeSetting(props: { node: ConfigNode }) {
  const label = () => controlLabel(props.node)
  return (
    <Show when={label() !== undefined}>
      <Show when={label()} fallback={<NodeControl node={props.node} />}>
        {(text) => (
          <FieldRow inline={INLINE_KINDS.includes(props.node.kind)} label={text()}>
            <NodeControl node={props.node} />
          </FieldRow>
        )}
      </Show>
    </Show>
  )
}

/** What a node is, or where it takes its value from, and the buttons for adding to a list */
function NodeControl(props: { node: ConfigNode }) {
  return (
    <Switch>
      <Match when={is(props.node, 'variable')}>{(variable) => <VariableSource name={variable().name} />}</Match>
      <Match when={is(props.node, 'lights')}>{(lights) => <LayerLights lens={lights().lens} />}</Match>
      <Match when={is(props.node, 'z')}>
        {(z) => <NumberBox class="scene-tree-number" aria-label="Z-index" step="1" value={z().lens.get().z ?? 0} onNumber={(value) => field(z().lens, 'z').set(Math.round(value) || undefined)} />}
      </Match>
      <Match when={is(props.node, 'region')}>{(region) => <ShapeKindPicker node={region()} />}</Match>
      <Match when={is(props.node, 'fill')}>{(fill) => <FillKindPicker node={fill()} />}</Match>
      <Match when={is(props.node, 'stops')}>{(stops) => <AddStop node={stops()} />}</Match>
      <Match when={is(props.node, 'mask')}>{(mask) => <MaskPicker lens={mask().lens} />}</Match>
      <Match when={is(props.node, 'behind')}>{(behind) => <BehindPicker node={behind()} />}</Match>
      <Match when={is(props.node, 'fit')}>{(fit) => <FitPicker node={fit()} />}</Match>
      <Match when={is(props.node, 'points')}>{(points) => <AddPoint node={points()} />}</Match>
      <Match when={is(props.node, 'traversals')}>{(traversals) => <AddTraversal node={traversals()} />}</Match>
      <Match when={is(props.node, 'node-ref')}>{(ref) => <NodeRefKindPicker node={ref()} />}</Match>
      <Match when={is(props.node, 'prop')}>{(prop) => <PropSource label={prop().label} {...propFieldSpec(prop())} />}</Match>
    </Switch>
  )
}

/** A value under the selected node, edited in place. Its label selects it, to edit what's under it in turn. */
function ValueNode(props: { node: ConfigNode }) {
  return (
    <Switch>
      <Match when={is(props.node, 'prop')}>{(prop) => <InlineProp node={prop()} />}</Match>
      <Match when={is(props.node, 'node-ref')}>{(ref) => <InlineNodeRef node={ref()} />}</Match>
    </Switch>
  )
}

/**
 * Where a prop takes its value from, its label, and its field, with the props under it, such as a wave's phase. The label
 * of a prop with its own row in the timeline, such as a keyframed one, goes to that row.
 */
function InlineProp(props: { node: Extract<ConfigNode, { kind: 'prop' }> }) {
  const tree = useContext(ConfigTreeContext)!
  const under = () => (props.node.children ?? []).filter((child) => child.kind === 'prop')
  // a switch or a color's swatch fits beside its label
  const inline = () => {
    const fallback = propFieldSpec(props.node).fallback
    return (typeof fallback === 'boolean' || (Array.isArray(fallback) && fallback.length === 4)) && 'Fixed' in props.node.lens.get()
  }
  return (
    <div class="scene-field scene-prop" classList={{ 'scene-field-inline': inline() }}>
      <div class="scene-prop-header">
        <span class="scene-prop-name">
          <PropSource label={props.node.label} {...propFieldSpec(props.node)} />
          <Show when={shownInTree(props.node)} fallback={<span class="scene-field-label">{props.node.label}</span>}>
            <button type="button" class="scene-property-link" title="Go to its row in the timeline" onClick={() => tree.select(props.node.id)}>
              {props.node.label}
            </button>
          </Show>
        </span>
        <Show when={!props.node.placement && props.node.place}>{(place) => <RemoveButton node={props.node} place={place()} />}</Show>
      </div>
      <PropField bare label={props.node.label} {...propFieldSpec(props.node)} />
      <Show when={props.node.anchorOf}>{(of) => <AnchorPresets lens={props.node.lens as Lens<Prop<Vec2>>} shape={of().shape} index={of().index} />}</Show>
      <Show when={under().length > 0}>
        <div class="scene-prop-under">
          <Index each={under()}>
            {(child) => (
              <Show when={child().id} keyed>
                {(_id) => <ValueNode node={child()} />}
              </Show>
            )}
          </Index>
        </div>
      </Show>
    </div>
  )
}

/** One end of a traversal: a light by name, or whichever is nearest a position */
function InlineNodeRef(props: { node: Extract<ConfigNode, { kind: 'node-ref' }> }) {
  const under = () => (props.node.children ?? []).filter((child) => child.kind === 'prop')
  return (
    <>
      <FieldRow label={props.node.label}>
        <NodeRefKindPicker node={props.node} />
      </FieldRow>
      <Show
        when={'Name' in props.node.lens.get()}
        fallback={
          <Index each={under()}>
            {(child) => (
              <Show when={child().id} keyed>
                {(_id) => <ValueNode node={child()} />}
              </Show>
            )}
          </Index>
        }
      >
        <NodeRefFields label={props.node.label} lens={props.node.lens} />
      </Show>
    </>
  )
}

/** A node's icon, which says what it is, or for a prop where its value comes from */
export function NodeIcon(props: { node: ConfigNode }) {
  const icon = (): Icon => {
    const node = props.node
    if (node.kind === 'layer') return node.lens.get().display !== undefined ? icons.Monitor : icons.Lightbulb
    if (node.kind === 'sound') return icons.AudioLines
    if (node.kind === 'section') return node.section === 'values' ? icons.Braces : icons.Variable
    if (node.kind === 'region') return SHAPE_ICONS[regionKind(node.lens, node.layer)]
    if (node.kind === 'fill') return FILL_ICONS[fillKind(node.lens)]
    if (node.modifier) return MODIFIER_ICONS[node.modifier]
    if (node.kind === 'prop') return SOURCE_ICONS[propKind(node.lens.get())]
    if (node.kind === 'place' && node.lens.get().display !== undefined) return icons.Monitor
    return KIND_ICONS[node.kind] ?? icons.Dot
  }
  return <Dynamic component={icon()} class="scene-tree-icon" aria-hidden="true" />
}

/** All the shapes a region or path can be, for their labels */
const REGION_LABELS = new Map(GRAPH_REGION_OPTIONS.map(({ value, label }) => [value, label]))

/**
 * A glance at a node: what type of layer it is, what shape or fill, how many are in a list, or a fixed prop's value or
 * the variable or traversal it reads from. A fixed color is a swatch.
 */
export function NodeSummary(props: { node: ConfigNode }) {
  const scene = useContext(SceneContext)!
  const text = (): string | null => {
    const node = props.node
    if (node.kind === 'layer' || node.kind === 'sound') return layerType(node, scene.get())
    if (node.kind === 'region') return REGION_LABELS.get(regionKind(node.lens, node.layer)) ?? null
    if (node.kind === 'fill') return FILL_OPTIONS.find(({ value }) => value === fillKind(node.lens))?.label ?? null
    if (node.kind === 'value') return NAMED_VALUE_LABELS[namedValueKind(scene.get().values![node.name]!)]
    if (node.kind === 'fit') return (node.lens.get() as { Fit: string }).Fit || null
    if (node.kind === 'node-ref') {
      const ref = node.lens.get()
      return 'Name' in ref ? ref.Name : 'Nearest'
    }
    if (node.kind === 'stops' || node.kind === 'points' || node.kind === 'traversals' || node.kind === 'section') {
      return node.children?.length ? String(node.children.length) : null
    }
    if (node.kind === 'prop') return propSummary(node.lens.get())
    return null
  }
  const color = () => {
    const node = props.node
    if (node.kind !== 'prop') return null
    const prop = node.lens.get()
    if (!('Fixed' in prop) || !Array.isArray(prop.Fixed) || prop.Fixed.length !== 4) return null
    const [r, g, b, a] = prop.Fixed
    return `rgb(${r} ${g} ${b} / ${a / 255})`
  }
  return (
    <>
      <Show when={color()}>{(background) => <span class="scene-tree-swatch" style={{ background: background() }} />}</Show>
      <Show when={text()}>{(summary) => <span class="scene-tree-summary">{summary()}</span>}</Show>
    </>
  )
}

/** What type of layer a layer is, naming the display where the scene draws on more than one */
function layerType(node: Extract<ConfigNode, { kind: 'layer' | 'sound' }>, scene: Scene) {
  if (node.kind === 'sound') return 'Sound'
  const layer = node.lens.get()
  if (layer.display !== undefined) return displayNames(scene).length > 1 ? `Display · ${layer.display}` : 'Display'
  return layerPlane(layer) === null ? 'Light sequence' : 'Light area'
}

/** A fixed prop's value, or the variable or traversal it reads from. One that changes over time has its track instead. */
function propSummary(prop: Prop<SceneValue>): string | null {
  if ('Variable' in prop) return prop.Variable
  if ('Visit' in prop) return prop.Visit.traversal
  if (!('Fixed' in prop)) return null
  const value = prop.Fixed
  if (typeof value === 'boolean') return value ? 'Yes' : 'No'
  if (typeof value === 'number') return formatNumber(value)
  return value.length === 2 ? value.map(formatNumber).join(', ') : null
}

const formatNumber = (value: number) => String(Math.round(value * 100) / 100)

/** Controls beside one another, at the size of a row */
function Controls(props: { class?: string; children: JSX.Element }) {
  return <span class={props.class ? `scene-tree-controls ${props.class}` : 'scene-tree-controls'}>{props.children}</span>
}

/** Takes a node out of its list, after asking. Moving it is by dragging its row in the tree. */
function RemoveButton(props: { node: ConfigNode; place: ListPlace }) {
  const menu = useContext(LayerMenuContext)!
  return (
    <button type="button" class="scene-icon-button" aria-label={`Remove ${props.node.label}`} title="Remove" disabled={props.place.canRemove === false} onClick={() => menu.remove(props.node.id)}>
      <icons.X aria-hidden="true" />
    </button>
  )
}

/** The fields a node has of its own, above the values under it */
function NodeFields(props: { node: ConfigNode }) {
  return (
    <Switch>
      <Match when={is(props.node, 'sound')}>{(sound) => <SoundFields lens={sound().lens} />}</Match>
      <Match when={is(props.node, 'region')}>
        {(region) => (
          <Switch fallback={<TextFields lens={region().lens as Lens<Shape>} layer={region().layer} />}>
            <Match when={region().layer?.get().graph}>
              <GraphFields layer={region().layer!} />
            </Match>
            <Match when={region().value !== undefined && 'Wave' in (region().value as Shape).base}>
              <WaveFields lens={region().lens as Lens<Shape>} />
            </Match>
            <Match when={region().value !== undefined && 'External' in (region().value as Shape).base}>
              <GeneratorFields lens={region().lens as Lens<Shape>} />
            </Match>
          </Switch>
        )}
      </Match>
      <Match when={is(props.node, 'fill')}>{(fill) => <FillFields lens={fill().lens} />}</Match>
      <Match when={is(props.node, 'stroke')}>{(stroke) => <StrokeFields node={stroke()} />}</Match>
      <Match when={props.node.modifier === 'Warp' && is(props.node, 'copy')}>{(warp) => <WarpFields lens={warp().lens as Lens<WarpTransform>} />}</Match>
      <Match when={is(props.node, 'traversal')}>{(traversal) => <TraversalFields lens={traversal().lens} layer={traversal().layer} />}</Match>
      <Match when={is(props.node, 'node-ref')}>{(ref) => <NodeRefFields label={ref().label} lens={ref().lens} />}</Match>
      <Match when={is(props.node, 'prop')}>
        {(prop) => (
          <>
            <PropField bare label={prop().label} {...propFieldSpec(prop())} />
            <Show when={prop().anchorOf}>{(of) => <AnchorPresets lens={prop().lens as Lens<Prop<Vec2>>} shape={of().shape} index={of().index} />}</Show>
          </>
        )}
      </Match>
    </Switch>
  )
}

/**
 * A three by three grid of buttons that put a rotate's or scale's anchor on a corner, the middle of a side, or the center
 * of the shape's box, as the machine last drew it. They're off while there's no drawing of the shape to go by, or the
 * anchor can't take a value, such as while it's random.
 */
function AnchorPresets(props: { lens: Lens<Prop<Vec2>>; shape: Lens<Shape>; index: number }) {
  const scene = useContext(SceneContext)!
  const poses = useContext(ShapePosesContext)
  const presets = () => {
    const pose = poses.of(props.shape.get())
    return pose && anchorPresets(pose, props.index)
  }
  const write = () => writer(scene, poses.time())
  const usable = () => presets() !== null && write().can(props.lens.get() as Prop<SceneValue>, [0, 0])
  return (
    <div class="scene-anchor-presets" role="group" aria-label="Put the anchor on">
      <For each={presets() ?? ANCHOR_SPOTS.map((label) => ({ label, anchor: [0, 0] as Vec2 }))}>
        {({ label, anchor }) => (
          <button
            type="button"
            class="scene-button scene-anchor-preset"
            title={`Anchor on the ${label.toLowerCase()} of the shape's box`}
            aria-label={label}
            disabled={!usable()}
            onClick={() => write().to(props.lens, anchor)}
          >
            <span class="scene-anchor-glyph" data-spot={label.toLowerCase().replace(' ', '-')} />
          </button>
        )}
      </For>
    </div>
  )
}

const propFieldSpec = (spec: PropSpec) => ({
  variableName: spec.variableName,
  lens: spec.lens,
  fallback: spec.fallback,
  ranges: spec.ranges,
})

/** A sound layer's sound and how loud it is. When it starts and ends is set on the timeline. */
function SoundFields(props: { lens: Lens<SoundLayer> }) {
  const volume = () => soundVolume(props.lens)
  const length = () => contentSoundLength(props.lens.get().sound)
  const end = () => soundEnd(props.lens.get(), length())

  return (
    <>
      <FieldRow label="Sound">
        <Show when={contentSounds().length > 0 || props.lens.get().sound !== ''} fallback={<span class="scene-note">No sounds in the machine's content folder</span>}>
          <SoundPicker lens={field(props.lens, 'sound')} volume={volume().get()} />
        </Show>
      </FieldRow>
      {/* a number rather than a value that changes, as a sound takes its volume when it starts */}
      <FieldRow label="Volume">
        <NumberField label="Volume" range={VOLUME} value={volume().get()} onChange={(next) => volume().set(Math.max(0, next))} />
      </FieldRow>
      <Show when={length()}>
        {(known) => (
          <div class="scene-fit">
            <span class="scene-note">{describeSound(props.lens.get(), known())}</span>
            <FitTimelineButton length={end()!} />
          </div>
        )}
      </Show>
    </>
  )
}

function describeSound({ start, skip = 0, end }: SoundLayer, length: number) {
  const skipped = skip > 0 ? `, the first ${skip.toFixed(2)} s skipped` : ''
  const cut = end === undefined || end - start >= length - skip ? '' : `, cut off after ${(end - start).toFixed(2)} s`
  return `${length.toFixed(2)} s long${skipped}${cut}`
}

/** A button for making the scene's timeline `length` long, to fit a sound or an image to it, then doing `onFit` */
function FitTimelineButton(props: { length: number; onFit?: () => void }) {
  const scene = useContext(SceneContext)!
  const length = () => snap(props.length)
  const fits = () => scene.get().timing?.duration === length()
  return (
    <button
      type="button"
      class="scene-button"
      disabled={fits() || length() < TIME_STEP}
      title={fits() ? 'The timeline is already this long' : `Make the timeline ${length().toFixed(2)} s long`}
      onClick={() =>
        batch(() => {
          scene.set(withDuration(scene.get(), length()))
          props.onFit?.()
        })
      }
    >
      Fit timeline
    </button>
  )
}

/**
 * Makes a layer a mask over the layers below it, or a layer again. A mask hides them where its fill is clear or its
 * region doesn't reach, or, inverted, where it's opaque.
 */
function MaskPicker(props: { lens: Lens<SceneLayer> }) {
  const value = () => {
    const layer = props.lens.get()
    if (!layer.mask) return 'Off'
    return layer.invert ? 'Inside' : 'Outside'
  }
  return (
    <Dropdown
      label="Mask"
      options={MASK_OPTIONS}
      value={value()}
      onChange={(value) => {
        const { mask: _mask, invert: _invert, ...layer } = props.lens.get()
        props.lens.set(value === 'Off' ? layer : { ...layer, mask: true, ...(value === 'Inside' ? { invert: true } : {}) })
      }}
    />
  )
}

/**
 * What a part of a layer goes behind: one of the named layers under it on the same thing at the same z, or everything
 * above a z-index lower than its own, which is how it goes behind another scene's layers. With it is a button that takes
 * the part out, so the layer is drawn whole there again. Where the part is, is drawn and dragged on the preview. A layer
 * it names that's no longer under it is still listed, to be seen and changed.
 */
function BehindPicker(props: { node: Extract<ConfigNode, { kind: 'behind' }> }) {
  const scene = useContext(SceneContext)!
  const under = () => field(props.node.lens, 'under')
  const named = () => {
    const current = under().get()
    return 'Layer' in current ? current.Layer : null
  }
  const options = () => {
    const names = layersUnder(scene.get(), props.node.layer)
    const current = named()
    const missing = current === null || names.includes(current) ? [] : [{ value: `layer:${current}`, label: `${current} (not under it)` }]
    return [...missing, ...names.map((name) => ({ value: `layer:${name}`, label: name })), { value: 'z', label: 'A z-index' }]
  }
  const ownZ = () => scene.get().layers[props.node.layer]?.z ?? 0
  const z = () => {
    const current = under().get()
    return 'Z' in current ? current.Z : null
  }
  return (
    <>
      <Dropdown
        label="Goes behind"
        options={options()}
        value={named() === null ? 'z' : `layer:${named()}`}
        onChange={(value) => under().set(value === 'z' ? { Z: z() ?? ownZ() - 1 } : { Layer: value.slice('layer:'.length) })}
      />
      <Show when={z() !== null}>
        {/* lower than the layer's own, as a part can only go down */}
        <NumberBox class="scene-tree-number" aria-label="Z-index it goes behind at" step="1" value={z()!} onNumber={(value) => under().set({ Z: Math.min(Math.round(value) || 0, ownZ() - 1) })} />
      </Show>
      <Show when={props.node.place}>{(place) => <RemoveButton node={props.node} place={place()} />}</Show>
    </>
  )
}

/** Which side of a shape's edge a stroke is on */
function StrokeFields(props: { node: Extract<ConfigNode, { kind: 'stroke' }> }) {
  const stroke = () => variant(props.node.lens, 'Stroke')
  return (
    <>
      <FieldRow label="Align">
        <Dropdown
          label="Align"
          options={STROKE_ALIGN_OPTIONS}
          value={stroke().get().align ?? 'Centered'}
          onChange={(align) => field(stroke(), 'align').set(align === 'Centered' ? undefined : align)}
        />
      </FieldRow>
    </>
  )
}

/** Text's font, its words, and where they sit in its box, which can be made to cover what the layer is on */
/** How a wave crosses its box, which a zigzag and a serpentine are set up with */
function WaveFields(props: { lens: Lens<Shape> }) {
  const wave = () => variant(field(props.lens, 'base') as Lens<ShapeNode>, 'Wave')
  return (
    <FieldRow label="Style">
      <Dropdown label="Style" options={WAVE_STYLE_OPTIONS} value={wave().get().style} onChange={(style) => field(wave(), 'style').set(style)} />
    </FieldRow>
  )
}

function TextFields(props: { lens: Lens<Shape>; layer?: Lens<SceneLayer> }) {
  const scene = useContext(SceneContext)!
  const surface = useContext(SurfaceContext)!
  const text = () => variant(field(props.lens, 'base') as Lens<ShapeNode>, 'Text')
  const source = () => field(text(), 'text')
  const sourceKind = () => Object.keys(source().get())[0] as TextSourceKind
  // the words it shows now, carried over to a fixed text or a variable's default
  const shown = () => {
    const current = source().get()
    if ('Fixed' in current) return current.Fixed
    const variable = 'Variable' in current ? scene.get().variables?.[current.Variable] : undefined
    return variable && 'Text' in variable ? variable.Text : 'TEXT'
  }
  const changeSource = (kind: TextSourceKind) => {
    if (kind === sourceKind()) return
    if (kind === 'Fixed') return source().set({ Fixed: shown() })
    if (kind === 'Number') {
      // a variable, as a number shown as text is nearly always the game's, with a default that previews the format
      const name = freshVariableName(scene.get(), 'number')
      scene.set(withVariableValue(scene.get(), name, { Int: NUMBER_PREVIEW }))
      return source().set({ Number: { value: { Variable: name }, format: 'Grouped' } })
    }
    const name = freshVariableName(scene.get(), 'text')
    scene.set(withVariableValue(scene.get(), name, { Text: shown() }))
    source().set({ Variable: name })
  }
  // one that isn't in the folder stays chosen, so it can be seen and changed
  // the game's own first, then the ones frontbox comes with, each from the smallest capitals to the largest
  const fonts = () => {
    const options = [...new Set([...contentFonts(), text().get().font])].filter((name) => name !== '').map((name) => ({ value: name, label: fontLabel(name), group: fontGroup(name) }))
    return [...options.filter((option) => option.group === 'Game'), ...options.filter((option) => option.group !== 'Game')]
  }
  // a fixed size moves to the nearest the new font is crisp at
  const changeFont = (font: string) => {
    const { size } = text().get()
    text().set({ ...text().get(), font, size: 'Fixed' in size ? { Fixed: crispTextSize(font, size.Fixed) } : size })
  }
  // the display or plane the layer is on
  const place = () => {
    const layer = props.layer?.get()
    return layer?.display ?? (layer && layerPlane(layer)) ?? 'display'
  }
  const fill = ([width, height]: Vec2) => {
    const box = { ...text().get(), width: { Fixed: width }, height: { Fixed: height } }
    props.lens.set({ ...props.lens.get(), base: { Text: box }, transform: [{ Translate: { Fixed: [width / 2, height / 2] } }] })
  }
  return (
    <>
      <FieldRow label="Font">
        <Show when={fonts().length > 0} fallback={<span class="scene-note">No fonts in the machine's content folder</span>}>
          <Dropdown label="Font" options={fonts()} value={text().get().font} onChange={changeFont} />
        </Show>
      </FieldRow>
      <Show when={text().get().font !== '' && !contentFonts().includes(text().get().font)}>
        <p class="scene-note scene-note-warn">There's no font by this name in the content folder, so this draws nothing.</p>
      </Show>
      <FieldRow label="Text from">
        <Dropdown label="Text from" options={TEXT_SOURCE_OPTIONS} value={sourceKind()} onChange={changeSource} />
      </FieldRow>
      <Switch>
        <Match when={sourceKind() === 'Fixed'}>
          <FieldRow label="Text">
            <textarea
              class="scene-variable-name scene-text-words"
              aria-label="Text"
              rows={2}
              spellcheck={false}
              value={shown()}
              onInput={(e) => source().set({ Fixed: e.currentTarget.value })}
            />
          </FieldRow>
        </Match>
        <Match when={sourceKind() === 'Variable'}>
          <TextVariableField lens={source() as Lens<Extract<TextSource, { Variable: string }>>} />
        </Match>
        <Match when={sourceKind() === 'Number'}>
          <NumberFormatField lens={field(variant(source(), 'Number'), 'format')} />
        </Match>
      </Switch>
      <FieldRow label="Align">
        <Dropdown label="Align" options={TEXT_ALIGN_OPTIONS} value={text().get().align} onChange={(align) => field(text(), 'align').set(align)} />
      </FieldRow>
      <FieldRow label="Vertical align">
        <Dropdown
          label="Vertical align"
          options={VERTICAL_ALIGN_OPTIONS}
          value={text().get().vertical_align}
          onChange={(align) => field(text(), 'vertical_align').set(align)}
        />
      </FieldRow>
      <FieldRow label="Too wide">
        <Dropdown
          label="Too wide"
          options={TEXT_OVERFLOW_OPTIONS}
          value={text().get().overflow ?? 'Wrap'}
          // left out of the file while it wraps, as it does by default
          onChange={(overflow) => field(text(), 'overflow').set(overflow === 'Wrap' ? undefined : overflow)}
        />
      </FieldRow>
      <Show when={surface()}>
        {(size) => (
          <button
            type="button"
            class="scene-button scene-fields-button"
            title={`Sizes the text's box to the ${place()} and moves it over it, in place of its transforms`}
            onClick={() => fill(size())}
          >
            Fill {place()}
          </button>
        )}
      </Show>
    </>
  )
}

/** The variable a text reads its words from. A name nothing has yet becomes a variable, with the words it showed. */
function TextVariableField(props: { lens: Lens<Extract<TextSource, { Variable: string }>> }) {
  const scene = useContext(SceneContext)!
  const name = () => props.lens.get().Variable
  const variable = () => scene.get().variables?.[name()]
  const rename = (next: string) => {
    if (next === '' || next === name()) return
    const current = variable()
    if (scene.get().variables?.[next] === undefined) scene.set(withVariableValue(scene.get(), next, current && 'Text' in current ? current : { Text: '' }))
    props.lens.set({ Variable: next })
  }
  const readable = () => {
    const current = variable()
    return current !== undefined && ('Text' in current || 'Int' in current || 'Float' in current)
  }
  return (
    <>
      <FieldRow label="Variable">
        <input
          class="scene-variable-name"
          aria-label="Text variable name"
          spellcheck={false}
          value={name()}
          onChange={(e) => {
            rename(e.currentTarget.value.trim())
            e.currentTarget.value = name()
          }}
        />
      </FieldRow>
      <Show when={!readable()}>
        <p class="scene-note scene-note-warn">No text or number variable has that name, so this draws nothing.</p>
      </Show>
    </>
  )
}

/** How a number is written out, and for an abbreviated one, how many characters it fits in */
function NumberFormatField(props: { lens: Lens<NumberFormat> }) {
  const kind = (): NumberFormatKind => {
    const format = props.lens.get()
    return typeof format === 'string' ? format : 'Abbreviated'
  }
  const length = () => {
    const format = props.lens.get()
    return typeof format === 'string' ? ABBREVIATED_LENGTH : format.Abbreviated.length
  }
  return (
    <>
      <FieldRow label="Format">
        <Dropdown
          label="Format"
          options={NUMBER_FORMAT_OPTIONS}
          value={kind()}
          onChange={(next) => props.lens.set(next === 'Abbreviated' ? { Abbreviated: { length: length() } } : next)}
        />
      </FieldRow>
      <Show when={kind() === 'Abbreviated'}>
        <FieldRow label="Characters">
          <NumberBox
            class="scene-tree-number"
            aria-label="Characters"
            min="1"
            step="1"
            value={length()}
            onNumber={(value) => props.lens.set({ Abbreviated: { length: Math.max(1, Math.round(value)) } })}
          />
        </FieldRow>
      </Show>
    </>
  )
}

/** A graph built from a layer's lights, as its region. Its traversals, which props can respond to, are under it. */
function GraphFields(props: { layer: Lens<SceneLayer> }) {
  return (
    <>
      <p class="scene-note">
        Only lights in the graph light up on this layer. To show them as a traversal reaches them, set the layer's opacity to Visit. On a plane, lights
        more than half a unit above or below it, such as on ramps, aren't in the graph.
      </p>
      <Show when={props.layer.get().region}>
        <p class="scene-note scene-note-warn">This layer also has a region, set in its file, which still limits where it shows.</p>
        <button type="button" class="scene-button scene-fields-button" onClick={() => field(props.layer, 'region').set(undefined)}>
          Remove region
        </button>
      </Show>
    </>
  )
}

const LIGHT_NAMES = 'scene-light-names'

/**
 * A traversal from the first light in the graph out to all the others, over the rest of the cycle.
 * Before the graph has been drawn there are no lights to name, so it starts from whichever is nearest the middle.
 */
export function newTraversal(name: string, nodes: string[], surface: Surface): Traversal {
  const [width, height] = surface ?? [1, 0]
  const from: NodeRef = nodes.length > 0 ? { Name: nodes[0] } : { Nearest: { Fixed: [width / 2, height / 2] } }
  return { name, from, algorithm: { BreadthFirst: { wander: { Fixed: 0 } } }, speed: { Duration: null }, start: 0 }
}

function AddTraversal(props: { node: Extract<ConfigNode, { kind: 'traversals' }> }) {
  const tree = useContext(ConfigTreeContext)!
  const scene = useContext(SceneContext)!
  const surface = useContext(SurfaceContext)!
  const graphNodes = useGraphNodes()
  const add = () =>
    batch(() => {
      scene.set(withTiming(scene.get()))
      const list = props.node.lens.get()
      props.node.lens.set([...list, newTraversal(freshTraversalName(props.node.layer.get(), 'walk'), graphNodes(), surface())])
      tree.expand(props.node.id)
      tree.select(`${props.node.id}/${list.length}`)
    })
  return <AddButton label="Add traversal" onClick={add} />
}

/** A traversal's name and how it goes. Where it goes from and to, and the props of how it goes, are under it. */
function TraversalFields(props: { lens: Lens<Traversal>; layer: Lens<SceneLayer> }) {
  const graphNodes = useGraphNodes()
  const surface = useContext(SurfaceContext)!
  const traversal = () => props.lens.get()
  const algorithm = () => field(props.lens, 'algorithm')
  const kind = () => algorithmKind(traversal().algorithm)
  const target = () => traversalTarget(algorithm())
  const wander = () =>
    field(variant(algorithm() as unknown as Lens<Record<Wandering, { wander: Prop<number> }>>, kind() as Wandering), 'wander')
  const speed = () => field(props.lens, 'speed')
  const speedKind = (): SpeedKind => ('Rate' in traversal().speed ? 'Rate' : 'Duration')
  const setSpeedKind = (next: SpeedKind) => {
    if (next === speedKind()) return
    speed().set(next === 'Rate' ? { Rate: { Fixed: RATE.rest! } } : ({ Duration: null } as Speed))
  }
  const used = createMemo(() => usedTraversals(props.layer.get()).has(traversal().name))

  // its responses and fits follow it to its new name
  const rename = (to: string) => {
    const from = traversal().name
    if (to === '' || to === from || traversalNames(props.layer.get()).includes(to)) return
    props.layer.set(withTraversalRenamed(props.layer.get(), from, to))
  }

  // a traversal going somewhere keeps going there as its algorithm changes, and keeps its wander
  const setKind = (next: AlgorithmKind) => {
    if (next === kind()) return
    // or to the far end of the graph, so a new one has somewhere to go
    const to = target()?.get() ?? { Name: graphNodes().at(-1) ?? '' }
    const keptWander: Prop<number> = WANDERING.includes(kind()) ? wander().get() : { Fixed: WANDER.rest! }
    if (next === 'ShortestPath') algorithm().set({ ShortestPath: { to, wander: keptWander } })
    else if (next === 'BreadthFirst') algorithm().set({ BreadthFirst: { wander: keptWander } })
    else if (next === 'Dijkstra') algorithm().set({ Dijkstra: { wander: keptWander } })
    else if (next === 'FanOut') algorithm().set({ FanOut: { to, angle: { Fixed: FAN_ANGLE.rest! }, count: { Fixed: FAN_COUNT.rest! }, wander: keptWander } })
    else if (next === 'Growth') {
      const growing = { reach: { Fixed: reachRange(surface()).rest! }, density: { Fixed: GROWTH_DENSITY.rest! }, branching: { Fixed: BRANCHING.rest! } }
      algorithm().set({ Growth: { to, ...growing, wander: keptWander } })
    } else if (next === 'AStar') algorithm().set({ AStar: { to } })
    else algorithm().set(next as TraversalAlgorithm)
  }

  return (
    <>
      <FieldRow label="Name">
        <input
          class="scene-variable-name"
          aria-label="Traversal name"
          spellcheck={false}
          value={traversal().name}
          onChange={(e) => {
            rename(e.currentTarget.value.trim())
            e.currentTarget.value = traversal().name
          }}
        />
      </FieldRow>
      <FieldRow label="Algorithm">
        <Dropdown label="Algorithm" options={ALGORITHM_OPTIONS} value={kind()} onChange={setKind} />
      </FieldRow>
      <Show when={!target()}>
        <p class="scene-note">Goes on to every light in the graph. The targeted algorithms go to one light.</p>
      </Show>
      <Show when={kind() === 'FanOut'}>
        <p class="scene-note">Also goes to random lights along an arc around From, through To, spread across it.</p>
      </Show>
      <Show when={kind() === 'Growth'}>
        <p class="scene-note">
          Sends offshoots from the path to as many random lights within Reach of it as Density. Branching is the chance an offshoot sends out one of its
          own.
        </p>
      </Show>
      <FieldRow label="Speed">
        <Dropdown label="Speed" options={SPEED_OPTIONS} value={speedKind()} onChange={setSpeedKind} />
      </FieldRow>
      <FieldRow label="Reverse">
        <ValueField
          label="Reverse"
          lens={{ get: () => traversal().reverse ?? false, set: (reverse) => props.lens.set({ ...traversal(), reverse: reverse === true ? true : undefined }) } as Lens<SceneValue>}
          ranges={[RATE, RATE]}
        />
      </FieldRow>
      <Show when={traversal().reverse}>
        <p class="scene-note">Plays backwards: it starts where it would have ended, and ends on From.</p>
      </Show>
      <p class="scene-note">
        {speedKind() === 'Duration'
          ? 'Its start and how long it takes are set by its bar on the timeline.'
          : 'Its start is set by its bar on the timeline.'}
      </p>
      <Show when={!used()}>
        <p class="scene-note scene-note-warn">
          Nothing responds to {traversal().name}, so it doesn't change how the layer looks. Set an opacity, color or other value to Visit.
        </p>
      </Show>
    </>
  )
}

/** The lights in the selected layer's graph, by name, which are the only ones a traversal can start from or go to */
function useGraphNodes() {
  const picking = useContext(NodePickingContext)
  return () => picking?.nodes() ?? []
}

/** Whether a node of a graph is a light by name, or whichever is nearest a position, which is a prop under it */
function NodeRefKindPicker(props: { node: Extract<ConfigNode, { kind: 'node-ref' }> }) {
  const tree = useContext(ConfigTreeContext)!
  const surface = useContext(SurfaceContext)!
  const graphNodes = useGraphNodes()
  const kind = () => ('Name' in props.node.lens.get() ? 'Name' : 'Nearest')
  const setKind = (next: 'Name' | 'Nearest') => {
    if (next === kind()) return
    const [width, height] = surface() ?? [1, 0]
    batch(() => {
      props.node.lens.set(next === 'Name' ? { Name: graphNodes()[0] ?? '' } : { Nearest: { Fixed: [width / 2, height / 2] } })
      tree.expand(props.node.id)
    })
  }
  return (
    <Controls>
      <Dropdown label={`${props.node.label} node`} options={NODE_OPTIONS} value={kind()} onChange={setKind} />
    </Controls>
  )
}

/** A light of the graph by name. It can be chosen by clicking it in the preview, and is pointed out there while this is hovered. */
function NodeRefFields(props: { label: string; lens: Lens<NodeRef> }) {
  const picking = useContext(NodePickingContext)
  const graphNodes = useGraphNodes()
  const name = () => (props.lens.get() as { Name: string }).Name

  // this field's own choosing, as another field's may be under way
  let mine: { choose: (name: string) => void } | null = null
  const choosing = () => picking !== undefined && picking.choosing[0]() !== null && picking.choosing[0]() === mine
  const toggleChoosing = () => {
    if (!picking) return
    const [, setChoosing] = picking.choosing
    if (choosing()) return setChoosing(null)
    mine = { choose: (chosen) => props.lens.set({ Name: chosen }) }
    setChoosing(mine)
  }
  onCleanup(() => {
    if (choosing()) picking!.choosing[1](null)
  })

  const point = (shown: boolean) => picking?.highlighted[1](shown ? name() : null)

  return (
    <div class="scene-field" onMouseEnter={() => point(true)} onMouseLeave={() => point(false)} onFocusIn={() => point(true)} onFocusOut={() => point(false)}>
      <div class="scene-node-pick">
        <input
          class="scene-variable-name"
          aria-label={`${props.label} light`}
          list={LIGHT_NAMES}
          spellcheck={false}
          value={name()}
          onChange={(e) => props.lens.set({ Name: e.currentTarget.value.trim() })}
        />
        <button
          type="button"
          class="scene-button"
          aria-pressed={choosing()}
          title={choosing() ? 'Stop choosing' : `Choose the ${props.label.toLowerCase()} light by clicking it in the preview`}
          onClick={toggleChoosing}
        >
          {choosing() ? 'Selecting…' : 'Select'}
        </button>
      </div>
      <Show when={graphNodes().length > 0 && !graphNodes().includes(name())}>
        <p class="scene-note scene-note-warn">This light isn't in the layer's graph, so the traversal reaches nothing from it.</p>
      </Show>
    </div>
  )
}

const fillKind = (lens: Lens<Fill>): FillKind => {
  const current = lens.get().base
  if (isNoFill(lens.get())) return 'None'
  if ('Solid' in current) return 'Solid'
  if ('Gradient' in current) return current.Gradient.style
  if ('Noise' in current) return 'Noise'
  if ('Cells' in current) return 'Cells'
  if ('External' in current) return 'External'
  return 'Image' in current ? 'Image' : 'Spritesheet'
}

/** How big an image, a sprite, or a generator's image is drawn, which carries over when switching between them */
const imageSizeOf = (node: FillNode) => ('Image' in node ? node.Image.size : 'Spritesheet' in node ? node.Spritesheet.size : 'External' in node ? node.External.size : undefined)

/** A gradient's, a noise's, or cells' stops, which carry over when switching between them */
const stopsOf = (node: FillNode) => ('Gradient' in node ? node.Gradient.stops : 'Noise' in node ? node.Noise.stops : 'Cells' in node ? node.Cells.stops : undefined)

/** What a fill is. A solid's color is under it, as is an animated image's frame. */
function FillKindPicker(props: { node: Extract<ConfigNode, { kind: 'fill' }> }) {
  const tree = useContext(ConfigTreeContext)!
  const base = () => field(props.node.lens, 'base')
  const kind = () => fillKind(props.node.lens)

  const setKind = (next: FillKind) => {
    const current = base().get()
    // no fill is a clear solid, whose color isn't worth carrying over
    const color = kind() === 'None' ? undefined : 'Solid' in current ? current.Solid : stopsOf(current)?.[0]?.color
    if (next === kind()) return
    batch(() => {
      if (next === 'None') {
        base().set({ Solid: { Fixed: [0, 0, 0, 0] } })
      } else if (next === 'Solid') {
        base().set({ Solid: color ?? { Fixed: WHITE } })
      } else if (next === 'Image') {
        const sheet = 'Spritesheet' in current ? current.Spritesheet : undefined
        base().set({ Image: { name: sheet?.name ?? '', size: imageSizeOf(current) } })
      } else if (next === 'Spritesheet') {
        const image = 'Image' in current ? current.Image : undefined
        base().set({ Spritesheet: { name: image?.name ?? '', rows: 1, columns: 1, size: imageSizeOf(current) } })
      } else if (next === 'External') {
        base().set({ External: { generator: imageGenerators()[0]?.name ?? '', size: imageSizeOf(current) } })
      } else if (next === 'Noise') {
        // dark to the color, so a solid turned into noise keeps its color on the bright parts
        const stops = stopsOf(current) ?? [
          { at: { Fixed: 0 }, color: { Fixed: BLACK } },
          { at: { Fixed: 1 }, color: color ?? { Fixed: WHITE } },
        ]
        base().set({ Noise: { scale: { Fixed: 0.25 }, octaves: 3, evolution: { Fixed: 0 }, seed: randomSeed(), stops } })
      } else if (next === 'Cells') {
        // dark until close to an edge, then the color, so a solid turned into cells is a web of its color
        const stops = stopsOf(current) ?? [
          { at: { Fixed: 0.7 }, color: { Fixed: BLACK } },
          { at: { Fixed: 1 }, color: color ?? { Fixed: WHITE } },
        ]
        base().set({ Cells: { scale: { Fixed: 0.25 }, jitter: { Fixed: 1 }, evolution: { Fixed: 0 }, seed: randomSeed(), stops } })
      } else if ('Gradient' in current) {
        base().set({ Gradient: { ...current.Gradient, style: next } })
      } else {
        const stops = stopsOf(current) ?? [
          { at: { Fixed: 0 }, color: color ?? { Fixed: WHITE } },
          { at: { Fixed: 1 }, color: { Fixed: BLACK } },
        ]
        base().set({ Gradient: { stops, style: next } })
      }
      tree.expand(props.node.id)
    })
  }

  // one that's already a generator's stays so, even if the app has none
  const options = () => (imageGenerators().length > 0 || kind() === 'External' ? [...FILL_OPTIONS, GENERATED_FILL] : FILL_OPTIONS)

  return (
    <Controls>
      <Dropdown label="Fill" options={options()} value={kind()} onChange={setKind} />
    </Controls>
  )
}

/** The image a fill shows, or how a noise looks. A gradient's stops are under it, as are a noise's. */
function FillFields(props: { lens: Lens<Fill> }) {
  const base = field(props.lens, 'base')
  return (
    <Switch>
      <Match when={'Noise' in base.get()}>
        <NoiseEditor lens={variant(base, 'Noise')} />
      </Match>
      <Match when={'Cells' in base.get()}>
        <CellsEditor lens={variant(base, 'Cells')} />
      </Match>
      <Match when={'Image' in base.get()}>
        <ImageEditor lens={variant(base, 'Image')} />
      </Match>
      <Match when={'Spritesheet' in base.get()}>
        <SpritesheetEditor lens={variant(base, 'Spritesheet')} />
      </Match>
      <Match when={'External' in base.get()}>
        <ImageGeneratorEditor lens={variant(base, 'External')} />
      </Match>
    </Switch>
  )
}

/** Which of the app's image generators makes a fill's image, and how big it's drawn. Its settings are the values under it. */
function ImageGeneratorEditor(props: { lens: Lens<ExternalFill> }) {
  const name = () => props.lens.get()?.generator ?? ''
  // one the app doesn't have stays chosen, so it can be seen and changed
  const options = () => [...new Set([...imageGenerators().map((generator) => generator.name), name()])].filter((each) => each !== '').map((each) => ({ value: each, label: each }))
  // another generator has other settings
  const change = (generator: string) => props.lens.set({ generator, size: props.lens.get().size })
  return (
    <>
      <FieldRow label="Generator">
        <Show when={options().length > 0} fallback={<span class="scene-note">The app has no image generators</span>}>
          <Dropdown label="Generator" options={options()} value={name()} onChange={change} />
        </Show>
      </FieldRow>
      <ImageSizeField lens={field(props.lens, 'size')} />
      <Show when={name() !== '' && imageGenerators().length > 0 && !imageGenerator(name())}>
        <p class="scene-note scene-note-warn">The app has no image generator by this name, so this scene can't be drawn.</p>
      </Show>
    </>
  )
}

/** A noise's style and detail. Its seed is kept out of the way: a new random one is all that's usually wanted. */
function NoiseEditor(props: { lens: Lens<NoiseFill> }) {
  const style = () => props.lens.get().style ?? 'Smooth'
  const octaves = (value: number) => field(props.lens, 'octaves').set(Math.min(MAX_OCTAVES, Math.max(1, Math.round(value) || 1)))
  return (
    <>
      <FieldRow label="Style">
        <Dropdown label="Style" options={NOISE_STYLE_OPTIONS} value={style()} onChange={(next) => field(props.lens, 'style').set(next === 'Smooth' ? undefined : next)} />
      </FieldRow>
      <span class="scene-note">{NOISE_STYLE_NOTES[style()]}</span>
      <FieldRow label="Detail">
        <NumberBox class="scene-tree-number" aria-label="Detail" title="Octaves: layers of finer detail on top of the largest blobs" min="1" max={MAX_OCTAVES} step="1" value={props.lens.get().octaves} onNumber={octaves} />
      </FieldRow>
      <button type="button" class="scene-button" title="Pick a different noise at random, keeping these settings" onClick={() => field(props.lens, 'seed').set(randomSeed())}>
        <icons.Dices aria-hidden="true" /> New pattern
      </button>
      <p class="scene-note">Evolution changes the noise in place and loops at every whole number, so taking it from 0 to 1 over the timeline repeats seamlessly. A Translate scrolls it instead.</p>
    </>
  )
}

/** Cells' style, and a new pattern, as for noise */
function CellsEditor(props: { lens: Lens<CellsFill> }) {
  const style = () => props.lens.get().style ?? 'Edges'
  return (
    <>
      <FieldRow label="Style">
        <Dropdown label="Style" options={CELL_STYLE_OPTIONS} value={style()} onChange={(next) => field(props.lens, 'style').set(next === 'Edges' ? undefined : next)} />
      </FieldRow>
      <span class="scene-note">{CELL_STYLE_NOTES[style()]}</span>
      <button type="button" class="scene-button" title="Pick different cells at random, keeping these settings" onClick={() => field(props.lens, 'seed').set(randomSeed())}>
        <icons.Dices aria-hidden="true" /> New pattern
      </button>
      <p class="scene-note">Jitter takes the cells from a grid of squares at 0 to fully random at 1. Evolution moves each cell around a small circle and loops at every whole number, so taking it from 0 to 1 over the timeline repeats seamlessly.</p>
    </>
  )
}

/** A warp's detail and pattern, as for noise */
function WarpFields(props: { lens: Lens<WarpTransform> }) {
  const warp = () => variant(props.lens, 'Warp')
  const octaves = (value: number) => field(warp(), 'octaves').set(Math.min(MAX_OCTAVES, Math.max(1, Math.round(value) || 1)))
  return (
    <>
      <FieldRow label="Detail">
        <NumberBox class="scene-tree-number" aria-label="Detail" title="Octaves: layers of finer ripples on top of the largest" min="1" max={MAX_OCTAVES} step="1" value={warp().get().octaves} onNumber={octaves} />
      </FieldRow>
      <button type="button" class="scene-button" title="Pick different ripples at random, keeping these settings" onClick={() => field(warp(), 'seed').set(randomSeed())}>
        <icons.Dices aria-hidden="true" /> New pattern
      </button>
      <p class="scene-note">Bends the fill, not the region. Transforms above it are bent with the fill; those below move the ripples too.</p>
    </>
  )
}

function ImageEditor(props: { lens: Lens<ImageFill> }) {
  const scene = useContext(SceneContext)!
  const name = () => props.lens.get().name
  const info = () => contentImageInfo(name())
  const frame = () => props.lens.get().frame

  // whether a chosen image is animated is only known once the machine has decoded it
  let choosing: string | null = null

  // a frame's number means nothing in another image, but where a track or a variable leads is still wanted
  const choose = (next: string) => {
    if (next === name()) return
    const current = frame()
    choosing = next
    const size = props.lens.get().size
    props.lens.set(current && !('Fixed' in current) ? { name: next, frame: current, size } : { name: next, size })
  }

  // an animated image plays from the start, unless it's been given something else to do
  createEffect(
    on(info, (known) => {
      if (!known || name() !== choosing) return
      choosing = null
      if (known.frames <= 1 || frame() !== undefined) return
      const timed = withTiming(scene.get(), imageLength(known))
      scene.set(timed)
      field(props.lens, 'frame').set(framesTrack(known, timed.timing!))
    }),
  )

  const changes = () => (info()?.frames ?? 1) > 1 && frame() !== undefined && !('Fixed' in frame()!) && !('Variable' in frame()!)

  return (
    <>
      <ImagePicker lens={{ get: name, set: choose }} />
      <ImageSizeField lens={field(props.lens, 'size')} />
      <Show when={info()}>
        {(known) => (
          <div class="scene-fit">
            <span class="scene-note">{describeImage(known())}</span>
            <Show when={known().frames > 1 && imageLength(known())}>
              {/* a track of its frames goes back to playing it once at its own speed, which is what fitting is for */}
              <FitTimelineButton length={imageLength(known())!} onFit={() => frame() && 'Keyframes' in frame()! && field(props.lens, 'frame').set(framesTrack(known(), scene.get().timing!))} />
            </Show>
          </div>
        )}
      </Show>
      <Show when={changes() && info()}>
        {(known) => <p class="scene-note">A fraction of a frame is dropped, so going up to {known().frames} gives every frame the same time.</p>}
      </Show>
    </>
  )
}

/** The sheet's image, and how many rows and columns of sprites it's laid out in */
function SpritesheetEditor(props: { lens: Lens<SpritesheetFill> }) {
  const info = () => contentImageInfo(props.lens.get().name)
  const count = (key: 'rows' | 'columns') => (value: number) => field(props.lens, key).set(Math.max(1, Math.round(value) || 1))
  const sprite = () => {
    const known = info()
    const { rows, columns } = props.lens.get()
    return known && `Each sprite is ${+(known.width / columns).toFixed(2)} × ${+(known.height / rows).toFixed(2)}`
  }
  return (
    <>
      <ImagePicker lens={field(props.lens, 'name')} />
      <FieldRow label="Rows">
        <NumberBox class="scene-tree-number" aria-label="Rows" min="1" step="1" value={props.lens.get().rows} onNumber={count('rows')} />
      </FieldRow>
      <FieldRow label="Columns">
        <NumberBox class="scene-tree-number" aria-label="Columns" min="1" step="1" value={props.lens.get().columns} onNumber={count('columns')} />
      </FieldRow>
      <ImageSizeField lens={field(props.lens, 'size')} />
      <Show when={sprite()}>{(text) => <span class="scene-note">{text()}</span>}</Show>
    </>
  )
}

/** Whether an image is stretched over its region or drawn at its own size, which a `Translate` then pans across */
function ImageSizeField(props: { lens: Lens<ImageSize | undefined> }) {
  return (
    <FieldRow label="Size">
      <Dropdown
        label="Size"
        options={IMAGE_SIZE_OPTIONS}
        value={props.lens.get() ?? 'Stretch'}
        onChange={(size) => props.lens.set(size === 'Stretch' ? undefined : size)}
      />
    </FieldRow>
  )
}

function describeImage({ width, height, frames, duration }: ImageInfo) {
  const size = `${width} × ${height}`
  if (frames <= 1) return size
  const length = duration > 0 ? `, ${duration.toFixed(2)} s long` : ''
  return `${size}, ${frames} frames (0 to ${frames - 1})${length}`
}

/**
 * Adds a stop at the end, in the color of the last one. Stops still spaced evenly are spaced again to fit it, and
 * otherwise it goes at the end of the gradient, leaving where the others have been put.
 */
function AddStop(props: { node: Extract<ConfigNode, { kind: 'stops' }> }) {
  const tree = useContext(ConfigTreeContext)!
  const add = () => {
    const list = props.node.lens.get()
    // its own copy, as a prop is told from another by which one it is
    const color = list.length > 0 ? structuredClone(list[list.length - 1].color) : { Fixed: WHITE }
    batch(() => {
      const added = [...list, { at: { Fixed: 1 }, color }]
      props.node.lens.set(evenlySpread(list) ? spreadStops(added) : added)
      tree.expand(props.node.id)
    })
  }
  return <AddButton label="Add stop" onClick={add} />
}

/** What a region or a path is. A graph is only for a layer's region, and only on a plane. */
function regionKind(lens: Lens<Shape | undefined>, layer?: Lens<SceneLayer>): RegionKind {
  if (layer?.get().graph) return 'Graph'
  const current = lens.get()
  return current ? (Object.keys(current.base)[0] as ShapeKind) : 'None'
}

/** Which of the app's shape generators makes a shape. Its settings are the values under it. */
function GeneratorFields(props: { lens: Lens<Shape> }) {
  const external = () => variant(field(props.lens, 'base') as Lens<ShapeNode>, 'External')
  const name = () => external().get()?.generator ?? ''
  // one the app doesn't have stays chosen, so it can be seen and changed
  const options = () => [...new Set([...shapeGenerators().map((generator) => generator.name), name()])].filter((each) => each !== '').map((each) => ({ value: each, label: each }))
  // another generator has other settings
  const change = (generator: string) => external().set({ generator })
  return (
    <>
      <FieldRow label="Generator">
        <Show when={options().length > 0} fallback={<span class="scene-note">The app has no shape generators</span>}>
          <Dropdown label="Generator" options={options()} value={name()} onChange={change} />
        </Show>
      </FieldRow>
      <Show when={name() !== '' && shapeGenerators().length > 0 && !shapeGenerator(name())}>
        <p class="scene-note scene-note-warn">The app has no shape generator by this name, so this scene can't be drawn.</p>
      </Show>
    </>
  )
}

/**
 * Picks what a layer's region is. On a light scene, it can also be its graph. In the file the two are separate, and a layer could have both, but the graph already says which
 * lights it shows on. A region has its outline in the preview, which can be hidden.
 */
function ShapeKindPicker(props: { node: Extract<ConfigNode, { kind: 'region' }> }) {
  const tree = useContext(ConfigTreeContext)!
  const surface = useContext(SurfaceContext)!
  const scene = useContext(SceneContext)!
  const graphNodes = useGraphNodes()
  const visibility = useContext(RegionVisibilityContext)
  const layer = () => props.node.layer
  const lens = () => props.node.lens as Lens<Shape | undefined>
  const base = () => field(lens() as Lens<Shape>, 'base') as Lens<ShapeNode>
  // a graph's nodes are where its lights are on a plane, so a sequence has none
  const graphs = () => layer() !== undefined && layerPlane(layer()!.get()) !== null
  const kind = () => regionKind(lens(), layer())
  // a generator is the app's code, so there's none to pick without any, though a shape that names one still says so
  const options = () =>
    (graphs() ? GRAPH_REGION_OPTIONS : SHAPE_OPTIONS).filter((option) => option !== GENERATED || shapeGenerators().length > 0 || kind() === 'External')
  const of = () => selectionOf(props.node.id)

  const change = (next: RegionKind | TemplateKind) => {
    const owner = layer()
    if (next === 'Graph') {
      // only offered for a layer. A graph is there to be traversed, so it comes with a traversal.
      if (!owner) return
      scene.set(withTiming(scene.get()))
      const graph = { traversals: [newTraversal(freshTraversalName(owner.get(), 'walk'), graphNodes(), surface())] }
      return owner.set({ ...owner.get(), region: undefined, graph })
    }
    if (owner && kind() === 'Graph') {
      return owner.set({ ...owner.get(), graph: undefined, region: next === 'None' ? undefined : newShape(next, surface()) })
    }
    if (next === 'None') return lens().set(undefined)
    // a line has no inside to fill, so one with no stroke would draw nothing
    if (owner && isOpen(newShape(next, surface()).base)) {
      const adjust = owner.get().region?.adjust ?? []
      if (!adjust.some((each) => 'Stroke' in each)) {
        const region = owner.get().region ?? newShape(next, surface())
        owner.set({ ...owner.get(), region: { ...region, adjust: [...adjust, newStroke(surface())] } })
      }
    }
    const current = lens().get()
    if (!current) return lens().set(newShape(next, surface()))
    // a generator's outline is its own, with no size to take on: it stays where the shape was placed
    if (next === 'External') return base().set(sizedShape(next, [0, 0]))
    // one shape turned into another takes up the same room, and one in place of a generator's is the size of a new shape
    const across = newShapeSize(surface())
    const size: Vec2 = 'External' in current.base ? [across, across] : shapeSize(base().get(), scene.get())
    if (!isTemplate(next)) return base().set(sizedShape(next, size))
    const style = WAVE_TEMPLATES[next]
    if (style) return base().set(waveShape(size, style))
    // a template's copies are made before the shape is placed, in place of any it had
    const template = templateShape(next, Math.max(...size))
    const placing = (current.transform ?? []).filter((transform) => !COPY_KINDS.some((copy) => copy in transform))
    lens().set({ ...current, base: template.base, transform: [...template.transform, ...placing] })
  }

  return (
    <Controls>
      <Dropdown<RegionKind | TemplateKind>
        label={props.node.label}
        options={options()}
        value={kind()}
        onChange={(next) => {
          if (next === kind()) return
          batch(() => {
            change(next)
            if (next !== 'None') tree.expand(props.node.id)
          })
        }}
      />
      {/* what's selected has its region drawn over the preview */}
      <Show when={visibility && of() && kind() !== 'None'}>
        <EyeButton label="the region in the preview" hidden={!visibility!.shown(of()!)} onChange={(hidden) => visibility!.show(of()!, !hidden)} />
      </Show>
    </Controls>
  )
}

/** A new shape, placed so that it can be seen, at a size that fits. A template's transforms come before the one that places it. */
export function newShape(kind: Exclude<ShapeKind, 'None'> | TemplateKind, surface: Surface): Shape {
  // a sequence runs from 0 to 1, with its points along the middle
  const [width, height] = surface ?? [1, 0]
  const across = newShapeSize(surface)
  const { base, transform } = isTemplate(kind) ? templateShape(kind, across) : { base: sizedShape(kind, [across, across]), transform: [] }
  return { base, transform: [...transform, { Translate: { Fixed: [width / 2, height / 2] } }] }
}

/**
 * The width over the height of a shape that's drawn at only one, which is `null` for one that can be drawn at any. A
 * circle, a spiral, a star, and the templates that revolve can't be stretched.
 */
export function drawnAspect(kind: Exclude<ShapeKind, 'None'> | TemplateKind): number | null {
  if (kind === 'Circle' || kind === 'Spiral' || kind === 'Pinwheel' || kind === 'Propeller') return 1
  if (kind !== 'Star') return null
  const [[left, top], [right, bottom]] = starBounds(LEGS.rest!, LEG_WIDTH.rest!)
  return (right - left) / (bottom - top)
}

/** A shape drawn to fill a box, given by its middle and size. One that can't be stretched is as big as fits in it. */
export function drawnShape(kind: Exclude<ShapeKind, 'None'> | TemplateKind, center: Vec2, size: Vec2): Shape {
  // a box with no width or height leaves nothing to divide by
  const [width, height] = size.map((side) => Math.max(side, 1e-3))
  const across = Math.min(width, height)
  let base: ShapeNode
  let transform: Transform[] = []
  // the middle of the shape's box, in its own space
  let middle: Vec2 = [0, 0]
  if (kind === 'Triangle') {
    // a star of one leg, as wide as its base and as tall as the leg is long, pointing up from the origin
    base = { Star: { radius: { Fixed: height }, legs: { Fixed: 1 }, leg_width: { Fixed: width / height } } }
    middle = [0, -height / 2]
  } else if (kind === 'Star') {
    const [[left, top], [right, bottom]] = starBounds(LEGS.rest!, LEG_WIDTH.rest!)
    const radius = Math.min(width / (right - left), height / (bottom - top))
    base = sizedShape('Star', [radius * 2, radius * 2])
    middle = [((left + right) / 2) * radius, ((top + bottom) / 2) * radius]
  } else if (kind === 'Zigzag' || kind === 'Serpentine') {
    base = waveShape([width, height], WAVE_TEMPLATES[kind]!)
  } else if (kind === 'Pinwheel' || kind === 'Propeller') {
    ;({ base, transform } = templateShape(kind, across))
    // a pinwheel's blades turn around their tips, which are half of it up from the origin
    if (kind === 'Pinwheel') middle = [0, -across / 2]
  } else {
    base = sizedShape(kind, kind === 'Circle' ? [across, across] : [width, height])
  }
  return { base, transform: [...transform, { Translate: { Fixed: [center[0] - middle[0], center[1] - middle[1]] } }] }
}

/**
 * A polygon, line, or curve through `points`, which are on what it's drawn on, placed at the middle of their box so that
 * it moves from there. A curve leaves each point heading from the one before toward the one after.
 */
export function drawnPoints(shape: PointsShape, points: Vec2[]): Shape {
  const [xs, ys] = [points.map(([x]) => x), points.map(([, y]) => y)]
  const middle: Vec2 = [(Math.min(...xs) + Math.max(...xs)) / 2, (Math.min(...ys) + Math.max(...ys)) / 2]
  const local = points.map(([x, y]): Vec2 => [x - middle[0], y - middle[1]])
  const base: ShapeNode =
    shape === 'Polygon'
      ? { Polygon: { points: local.map((point) => ({ Fixed: point })) } }
      : shape === 'Polyline'
        ? { Polyline: { points: local.map((point) => ({ Fixed: point })) } }
        : {
            Bezier: {
              points: local.map((point, index) => {
                const [before, after] = [local[index - 1] ?? point, local[index + 1] ?? point]
                return { point: { Fixed: point }, handle: { Fixed: [(after[0] - before[0]) / 6, (after[1] - before[1]) / 6] as Vec2 } }
              }),
            },
          }
  return { base, transform: [{ Translate: { Fixed: middle } }] }
}

/**
 * The corners of the box around a star of radius 1, as its top left and bottom right. Its first leg points up, and each
 * leg is `legWidth` across at the origin.
 */
function starBounds(legs: number, legWidth: number): [Vec2, Vec2] {
  const points = Array.from({ length: legs }, (_, leg) => {
    const angle = (leg / legs) * Math.PI * 2 - Math.PI / 2
    const [x, y] = [Math.cos(angle), Math.sin(angle)]
    const half = legWidth / 2
    return [
      [x, y],
      [-y * half, x * half],
      [y * half, -x * half],
    ] as Vec2[]
  }).flat()
  const xs = points.map(([x]) => x)
  const ys = points.map(([, y]) => y)
  return [
    [Math.min(...xs), Math.min(...ys)],
    [Math.max(...xs), Math.max(...ys)],
  ]
}

/** A template `across` wide, around the origin */
function templateShape(kind: TemplateKind, across: number): { base: ShapeNode; transform: Transform[] } {
  const length = across / 2
  // a star of one leg, pointing up from its base on the origin
  const triangle = (width: number): ShapeNode => ({ Star: { radius: { Fixed: length }, legs: { Fixed: 1 }, leg_width: { Fixed: width } } })
  const style = WAVE_TEMPLATES[kind]
  if (style) return { base: waveShape([across, across], style), transform: [] }
  if (kind === 'Triangle') return { base: triangle(TRIANGLE_WIDTH), transform: [] }
  if (kind === 'Pinwheel') {
    // its blades are triangles with their tips together
    return {
      base: triangle(PINWHEEL_BLADE_WIDTH),
      transform: [{ Revolve: { center: { Fixed: [0, -length] }, angle: { Fixed: 360 }, count: { Fixed: TEMPLATE_BLADES } } }],
    }
  }
  // each bar through the middle is two blades
  return {
    base: { Rect: { width: { Fixed: across }, height: { Fixed: across * PROPELLER_BLADE_WIDTH } } },
    transform: [{ Revolve: { center: { Fixed: [0, 0] }, angle: { Fixed: 180 }, count: { Fixed: TEMPLATE_BLADES / 2 } } }],
  }
}

/** A shape this wide and tall */
function sizedShape(kind: Exclude<ShapeKind, 'None'>, [width, height]: Vec2): ShapeNode {
  const reach = Math.max(width, height) / 2
  if (kind === 'Circle') return { Circle: { radius: { Fixed: reach } } }
  if (kind === 'Ellipse') return { Ellipse: { rx: { Fixed: width / 2 }, ry: { Fixed: height / 2 } } }
  if (kind === 'Rect') return { Rect: { width: { Fixed: width }, height: { Fixed: height } } }
  if (kind === 'Diamond') {
    return { Diamond: { width: { Fixed: width }, height: { Fixed: height } } }
  }
  if (kind === 'Star') {
    return { Star: { radius: { Fixed: reach }, legs: { Fixed: LEGS.rest! }, leg_width: { Fixed: LEG_WIDTH.rest! } } }
  }
  if (kind === 'Chevron') return { Chevron: { width: { Fixed: width }, height: { Fixed: height }, notch: { Fixed: NOTCH.rest! } } }
  if (kind === 'Spiral') return { Spiral: { radius: { Fixed: reach }, turns: { Fixed: TURNS.rest! }, flare: { Fixed: FLARE.rest! } } }
  if (kind === 'Wave') return waveShape([width, height], 'Smooth')
  // the first the app has, with each setting where the generator has it
  if (kind === 'External') return { External: { generator: shapeGenerators()[0]?.name ?? '' } }
  if (kind === 'Text') {
    const font = contentFonts()[0] ?? ''
    const size = { Fixed: crispTextSize(font, Math.max(Math.round(height / 2), 1)) }
    return { Text: { text: { Fixed: 'TEXT' }, font, size, width: { Fixed: width }, height: { Fixed: height }, align: 'Center', vertical_align: 'Middle', kerning: { Fixed: 0 }, leading: { Fixed: 0 } } }
  }
  // a wave across it, level through each of its points
  const wave: Vec2[] = [
    [-width / 2, height / 2],
    [0, -height / 2],
    [width / 2, height / 2],
  ]
  if (kind === 'Polyline') return { Polyline: { points: wave.map((point) => ({ Fixed: point })) } }
  if (kind === 'Bezier') {
    return { Bezier: { points: wave.map((point) => ({ point: { Fixed: point }, handle: { Fixed: [width / 4, 0] } })) } }
  }
  const corners: Vec2[] = [
    [-width / 2, -height / 2],
    [width / 2, -height / 2],
    [width / 2, height / 2],
    [-width / 2, height / 2],
  ]
  return { Polygon: { points: corners.map((corner) => ({ Fixed: corner })) } }
}

/** A wave this wide and tall, in `style` */
function waveShape([width, height]: Vec2, style: WaveStyle): ShapeNode {
  return { Wave: { width: { Fixed: width }, height: { Fixed: height }, runs: { Fixed: RUNS.rest! }, style } }
}

/** How wide and tall a shape is in its own space */
function shapeSize(base: ShapeNode, scene: Scene): Vec2 {
  const value = (prop: Prop<number>) => startingValue(prop, scene, 0)
  if ('Circle' in base) return [value(base.Circle.radius) * 2, value(base.Circle.radius) * 2]
  if ('Ellipse' in base) return [value(base.Ellipse.rx) * 2, value(base.Ellipse.ry) * 2]
  if ('Rect' in base) return [value(base.Rect.width), value(base.Rect.height)]
  if ('Text' in base) return [value(base.Text.width), value(base.Text.height)]
  if ('Diamond' in base) return [value(base.Diamond.width), value(base.Diamond.height)]
  if ('Chevron' in base) return [value(base.Chevron.width), value(base.Chevron.height)]
  if ('Star' in base) return [value(base.Star.radius) * 2, value(base.Star.radius) * 2]
  if ('Spiral' in base) return [value(base.Spiral.radius) * 2, value(base.Spiral.radius) * 2]
  if ('Wave' in base) return [value(base.Wave.width), value(base.Wave.height)]
  const points = shapePoints(base).map((point) => startingValue(point, scene, [0, 0] as Vec2))
  const reach = (axis: 0 | 1) => Math.max(...points.map((p) => p[axis])) - Math.min(...points.map((p) => p[axis]))
  return [reach(0), reach(1)]
}

/**
 * Adds a point to a polygon halfway back to its first, so it keeps its outline. A line carries on from its end, and a
 * bezier's new point has the end's handle.
 */
function AddPoint(props: { node: Extract<ConfigNode, { kind: 'points' }> }) {
  const tree = useContext(ConfigTreeContext)!
  const scene = useContext(SceneContext)!
  const at = (point: Prop<Vec2>) => startingValue(point, scene.get(), [0, 0] as Vec2)

  const added = (shape: PointsShape): unknown => {
    if (shape === 'Bezier') {
      const list = props.node.lens.get() as BezierPoint[]
      const last = list[list.length - 1]
      const point = pastEnd(at((list[list.length - 2] ?? last).point), at(last.point))
      return { point: { Fixed: point }, handle: { Fixed: at(last.handle) } }
    }
    const list = props.node.lens.get() as Prop<Vec2>[]
    const [first, last] = [at(list[0]), at(list[list.length - 1])]
    if (shape === 'Polyline') return { Fixed: pastEnd(at(list[list.length - 2] ?? list[0]), last) }
    return { Fixed: [(first[0] + last[0]) / 2, (first[1] + last[1]) / 2] }
  }

  const add = () =>
    batch(() => {
      const list = props.node.lens.get() as unknown[]
      ;(props.node.lens as Lens<unknown[]>).set([...list, added(props.node.shape)])
      tree.expand(props.node.id, `${props.node.id}/${list.length}`)
    })
  return <AddButton label="Add point" onClick={add} />
}

/** Half as far again along a line's last stretch */
function pastEnd(before: Vec2, end: Vec2): Vec2 {
  return [end[0] + (end[0] - before[0]) / 2, end[1] + (end[1] - before[1]) / 2]
}

/**
 * A transform that leaves things where they are, until it's changed. A fit starts on the layer's first traversal. A
 * mirror's line starts along the right of a shape `size` across, so its reflection is beside it, with its center at
 * the top corner, clear of the shape's move handle at the middle of that side. A revolve and a bisect start on its
 * middle.
 */
function newTransform(kind: TransformKind, traversals: string[], size: Vec2): Transform {
  if (kind === 'Translate') return { Translate: { Fixed: [0, 0] } }
  if (kind === 'Rotate') return { Rotate: { degrees: { Fixed: 0 }, anchor: { Fixed: [0, 0] } } }
  if (kind === 'Scale') return { Scale: { amount: { Fixed: 1 }, anchor: { Fixed: [0, 0] } } }
  if (kind === 'Mirror') return { Mirror: { center: { Fixed: [size[0] / 2, -size[1] / 2] }, rotation: { Fixed: 0 } } }
  if (kind === 'Revolve') return { Revolve: { center: { Fixed: [0, 0] }, angle: { Fixed: REVOLVE_ANGLE.rest! }, count: { Fixed: COPIES.rest! } } }
  if (kind === 'Bisect') return { Bisect: { center: { Fixed: [0, 0] }, rotation: { Fixed: 0 } } }
  if (kind === 'Shift') return { Shift: { amount: { Fixed: SHIFT.rest! }, rotation: { Fixed: 0 } } }
  if (kind === 'Bulge') return { Bulge: { Fixed: BULGE.rest! } }
  if (kind === 'Round') return { Round: { Fixed: 0 } }
  // a shape is copied beside itself with a gap, and a fill is tiled twice across
  if (kind === 'Repeat') return { Repeat: { Fixed: size[0] > 0 ? [Math.ceil(size[0] * 1.5), 0] : [0.5, 0] } }
  if (kind === 'Polar') return { Polar: {} }
  if (kind === 'Warp') return { Warp: { amount: { Fixed: WARP_AMOUNT.rest! }, scale: { Fixed: 0.5 }, octaves: 2, evolution: { Fixed: 0 }, seed: randomSeed() } }
  return { Fit: traversals[0] ?? '' }
}

/** The row for adding modifiers to a shape or a fill */
export type ModifiersNode = Extract<ConfigNode, { kind: 'modifiers' }>

/** A modifier that can be added: a transform, an adjust, or a stroke, feather, or shadow on a shape's edge */
export type ModifierKind = TransformKind | AdjustKind | 'Stroke' | 'Feather' | 'Shadow'

type ModifierOption = { value: ModifierKind; label: string }

/**
 * The modifiers a row offers, with `traversals` the layer's: transforms, with those only for a drawn shape and a fit
 * only for a layer with traversals to fit to, then transforms only for a fill and adjusts on a fill, then a stroke, feather, or shadow on
 * a drawn shape.
 */
export function modifierOptions(
  node: ModifiersNode,
  traversals: string[],
): { transforms: ModifierOption[]; fillTransforms: ModifierOption[]; adjusts: ModifierOption[]; strokes: ModifierOption[] } {
  return {
    transforms: [...TRANSFORM_OPTIONS, ...(node.shape ? SHAPE_TRANSFORM_OPTIONS : []), ...(traversals.length > 0 ? [FIT_OPTION] : [])],
    // a fill is the node with adjusts
    fillTransforms: node.adjusts ? FILL_TRANSFORM_OPTIONS : [],
    adjusts: node.adjusts ? ADJUST_OPTIONS : [],
    // a line comes with the one stroke it's drawn in, and is cased or made to glow by a shadow rather than another
    strokes: !node.shape ? [] : isOpen(node.shape.get().base) ? SHAPE_ADJUST_OPTIONS.filter((option) => option.value !== 'Stroke') : SHAPE_ADJUST_OPTIONS,
  }
}

export const modifierIcon = (kind: ModifierKind): Icon => (kind === 'Stroke' ? KIND_ICONS.stroke! : MODIFIER_ICONS[kind])

/**
 * Adds a transform, an adjust, or a stroke, feather, or shadow to a row's, after those already there, and selects it. A transform only for
 * shapes goes before a shape's first translate instead, so that it works in the shape's own space and moves with the
 * shape. `surface` and `traversals` are the layer's.
 */
export function addModifier(tree: ConfigTree, scene: Scene, node: ModifiersNode, kind: ModifierKind, surface: Surface, traversals: string[]) {
  const owner = node.id.slice(0, node.id.lastIndexOf('/'))
  // a new modifier is selected and expanded, so its props show
  const show = (id: string) => {
    tree.select(id)
    tree.expand(id)
  }
  batch(() => {
    if (kind === 'Stroke' || kind === 'Feather' || kind === 'Shadow') {
      const strokes = orEmpty(field(node.shape!, 'adjust'))
      const list = strokes.get()
      const adjust = kind === 'Stroke' ? newStroke(surface) : kind === 'Feather' ? newFeather(surface) : newShadow(surface)
      strokes.set([...list, adjust])
      show(`${owner}/adjust/${list.length}`)
    } else if (kind in ADJUSTS) {
      const list = node.adjusts!.get()
      node.adjusts!.set([...list, newAdjust(kind as AdjustKind)])
      show(`${owner}/adjust/${list.length}`)
    } else {
      const list = node.transforms.get()
      const shape = node.shape?.get()
      const size = shape ? shapeSize(shape.base, scene) : ([0, 0] as Vec2)
      const placed = list.findIndex((transform) => 'Translate' in transform)
      // in the shape's own space, so it moves with the shape
      const inOwnSpace = COPY_KINDS.includes(kind as CopyKind) || RESHAPE_KINDS.includes(kind as ReshapeKind)
      const index = inOwnSpace && placed >= 0 ? placed : list.length
      node.transforms.set([...list.slice(0, index), newTransform(kind as TransformKind, traversals, size), ...list.slice(index)])
      tree.insert(`${owner}/transform`, index)
      show(`${owner}/transform/${index}`)
    }
  })
}

/**
 * Adds a transform, an adjust, or a stroke, feather, or shadow, listed together. It says "Modifier" unless given what to show,
 * such as just an icon, and then `label` says what it's for.
 */
function AddModifier(props: { node: ModifiersNode; class?: string; label?: string; children?: JSX.Element }) {
  const tree = useContext(ConfigTreeContext)!
  const scene = useContext(SceneContext)!
  const surface = useContext(SurfaceContext)!
  const traversals = useContext(TraversalsContext)
  const groups = () => {
    const { transforms, fillTransforms, adjusts, strokes } = modifierOptions(props.node, traversals())
    return [transforms, fillTransforms, adjusts, strokes].filter((group) => group.length > 0)
  }
  const add = (kind: ModifierKind) => addModifier(tree, scene.get(), props.node, kind, surface(), traversals())
  return (
    <Controls>
      <AddMenu label={props.label ?? 'Add a modifier'} class={props.class ? `scene-add-modifier ${props.class}` : 'scene-add-modifier'} groups={groups()} onPick={add}>
        {props.children ?? (
          <>
            <icons.Plus aria-hidden="true" />
            Modifier
          </>
        )}
      </AddMenu>
    </Controls>
  )
}

/** Which traversal a fit is to. It runs from where the traversal starts, at -0.5 across, to the light it reaches last, at 0.5. */
function FitPicker(props: { node: Extract<ConfigNode, { kind: 'fit' }> }) {
  const traversals = useContext(TraversalsContext)
  const fitted = () => (props.node.lens.get() as { Fit: string }).Fit
  // one the layer doesn't have anymore stays chosen, so it can be seen and changed
  const options = () => [...new Set([...traversals(), fitted()])].filter((name) => name !== '').map((name) => ({ value: name, label: name }))
  const missing = () => fitted() !== '' && !traversals().includes(fitted())
  return (
    <Controls>
      <Show when={options().length > 0} fallback={<span class="scene-tree-summary scene-note-warn">No traversals to fit to</span>}>
        <Dropdown label="Traversal to fit to" options={options()} value={fitted()} onChange={(name) => props.node.lens.set({ Fit: name })} />
      </Show>
      <Show when={missing()}>
        <span class="scene-tree-summary scene-note-warn" title="The layer has no traversal by this name, so this changes nothing.">
          Missing
        </span>
      </Show>
    </Controls>
  )
}

/** A button with a menu of things to add, which asks which one. Groups are set apart by a line. */
export function AddMenu<T extends string>(props: { label: string; class: string; groups: { value: T; label: string }[][]; onPick: (kind: T) => void; children: JSX.Element }) {
  return (
    <Menu.Root positioning={{ placement: 'bottom-start' }} onSelect={(details) => props.onPick(details.value as T)}>
      <Menu.Trigger class={props.class} aria-label={props.label} title={props.label}>
        {props.children}
      </Menu.Trigger>
      <ScenePortal>
        <Menu.Positioner>
          <Menu.Content class="scene-dropdown-list scene-add-list">
            <For each={props.groups}>
              {(group, index) => (
                <>
                  <Show when={index() > 0}>
                    <Menu.Separator />
                  </Show>
                  <For each={group}>
                    {(option) => (
                      <Menu.Item value={option.value}>
                        <Menu.ItemText>{option.label}</Menu.ItemText>
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
  )
}

/** A named value added to the scene, of the type picked */
function AddValue() {
  const scene = useContext(SceneContext)!
  const add = (kind: NamedValueKind) => scene.set(withNewValue(scene.get(), kind))
  return (
    <AddMenu label="Add value" class="scene-add-modifier" groups={[NAMED_VALUE_KINDS.map((kind) => ({ value: kind, label: NAMED_VALUE_LABELS[kind] }))]} onPick={add}>
      <icons.Plus aria-hidden="true" />
      Value
    </AddMenu>
  )
}

/** Adds to a list, saying what it adds */
function AddButton(props: { label: string; onClick: () => void }) {
  return (
    <button type="button" class="scene-add-modifier" onClick={props.onClick}>
      <icons.Plus aria-hidden="true" />
      {props.label}
    </button>
  )
}
