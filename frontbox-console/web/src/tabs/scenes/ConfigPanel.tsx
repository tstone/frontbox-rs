import { Accordion } from '@ark-ui/solid/accordion'
import { createEffect, createSignal, Index, Match, Show, Switch, useContext, type Signal } from 'solid-js'
import { contentImageInfo } from '../../state/contentImages'
import type { Color } from '../../types/generated/Color'
import type { ColorAdjust } from '../../types/generated/ColorAdjust'
import type { ColorSpace } from '../../types/generated/ColorSpace'
import type { FillNode } from '../../types/generated/FillNode'
import type { ImageInfo } from '../../types/generated/ImageInfo'
import type { Layer } from '../../types/generated/Layer'
import type { Prop } from '../../types/generated/Prop'
import type { Scene } from '../../types/generated/Scene'
import type { ShapeNode } from '../../types/generated/ShapeNode'
import type { Stop } from '../../types/generated/Stop'
import type { Transform } from '../../types/generated/Transform'
import { ImagePicker } from './ImagePicker'
import { Dropdown, FieldRow, NumberField, PropField, SceneContext, SurfaceContext } from './SceneFields'
import {
  both,
  field,
  frameRanges,
  HUE,
  item,
  layerLabel,
  moved,
  newShapeSize,
  orElse,
  radius,
  orEmpty,
  pointRanges,
  removed,
  ROTATION,
  scaleRanges,
  sizeRanges,
  startingValue,
  translateRanges,
  UNIT,
  variant,
  type Lens,
  type Range,
  type Ranges,
  type Vec2,
  type TransformSpace,
} from './sceneModel'

type Fill = Layer['fill']
type FillKind = 'Solid' | 'Gradient' | 'Image'
type ImageFill = Extract<FillNode, { Image: unknown }>['Image']
type Shape = NonNullable<Layer['shape']>
type EllipseRadii = Extract<ShapeNode, { Ellipse: unknown }>['Ellipse']
type ShapeKind = 'None' | 'Ellipse' | 'Rect' | 'Polygon'

const WHITE: Color = [255, 255, 255, 255]
const BLACK: Color = [0, 0, 0, 255]
const COLOR_RANGES = both(UNIT)

const VIEWPORT_OPTIONS: { value: 'Sequence' | 'Area'; label: string }[] = [
  { value: 'Sequence', label: 'Sequence (1D)' },
  { value: 'Area', label: 'Area (2D)' },
]
const FILL_OPTIONS: { value: FillKind; label: string }[] = [
  { value: 'Solid', label: 'Solid' },
  { value: 'Gradient', label: 'Gradient' },
  { value: 'Image', label: 'Image' },
]
const SPACE_OPTIONS: { value: ColorSpace; label: string }[] = [
  { value: 'Oklab', label: 'Oklab' },
  { value: 'Rgb', label: 'RGB' },
]
const SHAPE_OPTIONS: { value: ShapeKind; label: string }[] = [
  { value: 'None', label: 'All' },
  { value: 'Ellipse', label: 'Ellipse' },
  { value: 'Rect', label: 'Rectangle' },
  { value: 'Polygon', label: 'Polygon' },
]
const TRANSFORM_OPTIONS: { value: TransformKind; label: string }[] = [
  { value: 'Translate', label: 'Translate' },
  { value: 'Rotate', label: 'Rotate' },
  { value: 'Scale', label: 'Scale' },
]
const ADJUST_OPTIONS: { value: 'HueShift'; label: string }[] = [{ value: 'HueShift', label: 'Hue shift' }]

type TransformKind = 'Translate' | 'Rotate' | 'Scale'

/** A transform that leaves things where they are, until it's changed */
const NEW_TRANSFORMS: Record<TransformKind, Transform> = {
  Translate: { Translate: { Fixed: [0, 0] } },
  Rotate: { Rotate: { Fixed: 0 } },
  Scale: { Scale: { Fixed: [1, 1] } },
}

/** The scene's viewport and layers */
export default function ConfigPanel(props: { openLayer: Signal<number | null> }) {
  return (
    <div class="scene-config">
      <ViewportEditor />
      <LayerList open={props.openLayer} />
    </div>
  )
}

/** The scene's layers, with one open at a time */
function LayerList(props: { open: Signal<number | null> }) {
  const scene = useContext(SceneContext)!
  const layers = field(scene, 'layers')
  // by position, so this follows the open layer as layers around it move and go
  const [open, setOpen] = props.open

  const add = () => {
    layers.set([...layers.get(), { fill: { base: { Solid: { Fixed: WHITE } } } }])
    setOpen(layers.get().length - 1)
  }

  const move = (index: number, by: number) => {
    layers.set(moved(layers.get(), index, by))
    setOpen((current) => (current === index ? index + by : current === index + by ? index : current))
  }

  const remove = (index: number) => {
    layers.set(removed(layers.get(), index))
    setOpen((current) => {
      if (current === null || current === index) return null
      return current > index ? current - 1 : current
    })
  }

  return (
    <>
      <Accordion.Root
        class="scene-layers"
        collapsible
        lazyMount
        unmountOnExit
        value={open() === null ? [] : [String(open())]}
        onValueChange={(details) => setOpen(details.value.length > 0 ? Number(details.value[0]) : null)}
      >
        <Index each={layers.get()}>
          {(layer, index) => (
            <Accordion.Item class="scene-item" value={String(index)}>
              <header>
                <Accordion.ItemTrigger>
                  <Accordion.ItemIndicator>›</Accordion.ItemIndicator>
                  <span class="scene-item-name" title={layerLabel(layer(), index)}>
                    {layerLabel(layer(), index)}
                  </span>
                </Accordion.ItemTrigger>
                <ItemButtons
                  title={layerLabel(layer(), index)}
                  list={layers}
                  index={index}
                  onMove={(by) => move(index, by)}
                  onRemove={() => remove(index)}
                />
              </header>
              <Accordion.ItemContent>
                <div class="scene-config">
                  <LayerEditor lens={item(layers, index)} index={index} />
                </div>
              </Accordion.ItemContent>
            </Accordion.Item>
          )}
        </Index>
      </Accordion.Root>
      <AddButton onClick={add}>Add layer</AddButton>
    </>
  )
}

function ViewportEditor() {
  const scene = useContext(SceneContext)!
  const viewport = field(scene, 'viewport')

  return (
    <section class="scene-group">
      <FieldRow label="Type">
        <Dropdown
          label="Type"
          options={VIEWPORT_OPTIONS}
          value={viewport.get()}
          onChange={(value) => {
            if (value !== viewport.get()) scene.set(onViewport(scene.get(), value))
          }}
        />
      </FieldRow>
    </section>
  )
}

/** A shape is measured in its viewport's units, so it would land somewhere else on another. Each layer goes back to covering all of it. */
function onViewport(scene: Scene, viewport: Scene['viewport']): Scene {
  return { ...scene, viewport, layers: scene.layers.map(({ shape, ...layer }) => layer) }
}

function LayerEditor(props: { lens: Lens<Layer>; index: number }) {
  const name = field(props.lens, 'name')

  return (
    <>
      <section class="scene-group">
        <FieldRow label="Name">
          <input
            class="scene-layer-name"
            aria-label="Layer name"
            placeholder={`Layer ${props.index + 1}`}
            spellcheck={false}
            value={name.get() ?? ''}
            onChange={(e) => {
              name.set(e.currentTarget.value.trim() || undefined)
              e.currentTarget.value = name.get() ?? ''
            }}
          />
        </FieldRow>
      </section>
      <ShapeEditor lens={field(props.lens, 'shape')} />
      <FillEditor lens={field(props.lens, 'fill')} />
    </>
  )
}

function FillEditor(props: { lens: Lens<Fill> }) {
  const base = field(props.lens, 'base')
  const kind = (): FillKind => {
    const current = base.get()
    return 'Solid' in current ? 'Solid' : 'Gradient' in current ? 'Gradient' : 'Image'
  }

  const setKind = (next: FillKind) => {
    const current = base.get()
    const color = 'Solid' in current ? current.Solid : 'Gradient' in current ? current.Gradient.stops[0]?.color : undefined
    if (next === kind()) return
    if (next === 'Solid') {
      base.set({ Solid: color ?? { Fixed: WHITE } })
    } else if (next === 'Gradient') {
      const stops = [
        { at: 0, color: color ?? { Fixed: WHITE } },
        { at: 1, color: { Fixed: BLACK } },
      ]
      base.set({ Gradient: { stops, space: 'Oklab' } })
    } else {
      base.set({ Image: { name: '' } })
    }
  }

  return (
    <section class="scene-group">
      <FieldRow label="Fill">
        <Dropdown label="Fill" options={FILL_OPTIONS} value={kind()} onChange={setKind} />
      </FieldRow>
      <Switch>
        <Match when={kind() === 'Solid'}>
          <PropField label="Color" paramName="color" fallback={WHITE} ranges={COLOR_RANGES} lens={variant(base, 'Solid')} />
        </Match>
        <Match when={kind() === 'Gradient'}>
          <GradientEditor lens={variant(base, 'Gradient')} />
        </Match>
        <Match when={kind() === 'Image'}>
          <ImageEditor lens={variant(base, 'Image')} />
        </Match>
      </Switch>
      <TransformList lens={orEmpty(field(props.lens, 'transform'))} space="fill" />
      <AdjustList lens={orEmpty(field(props.lens, 'adjust'))} />
    </section>
  )
}

function ImageEditor(props: { lens: Lens<ImageFill> }) {
  const name = () => props.lens.get().name
  const info = () => contentImageInfo(name())
  const frame = () => props.lens.get().frame

  // a frame's number means nothing in another image, but where a track or a param leads is still wanted
  const choose = (next: string) => {
    if (next === name()) return
    const current = frame()
    props.lens.set(current && !('Fixed' in current) ? { name: next, frame: current } : { name: next })
  }

  // one that's already moving stays in reach, so it can be undone after picking an image that isn't animated
  const hasFrames = () => (info()?.frames ?? 1) > 1 || (frame() !== undefined && !('Fixed' in frame()!))
  const changes = () => hasFrames() && frame() !== undefined && !('Fixed' in frame()!) && !('Param' in frame()!)

  return (
    <>
      <ImagePicker lens={{ get: name, set: choose }} />
      <Show when={info()}>{(known) => <p class="scene-note">{describeImage(known())}</p>}</Show>
      <Show when={hasFrames()}>
        <PropField
          label="Frame"
          paramName="frame"
          fallback={0}
          ranges={frameRanges(info()?.frames)}
          lens={orElse(field(props.lens, 'frame'), { Fixed: 0 })}
        />
      </Show>
      <Show when={changes() && info()}>
        {(known) => (
          <p class="scene-note">
            A fraction of a frame is dropped, so going up to {known().frames} gives every frame the same time.
          </p>
        )}
      </Show>
    </>
  )
}

function describeImage({ width, height, frames, duration }: ImageInfo) {
  const size = `${width} × ${height}`
  if (frames <= 1) return size
  const length = duration > 0 ? `, ${duration.toFixed(2)} s long` : ''
  return `${size}, ${frames} frames (0 to ${frames - 1})${length}`
}

function GradientEditor(props: { lens: Lens<{ stops: Stop[]; space: ColorSpace }> }) {
  const stops = field(props.lens, 'stops')

  // the format needs stops in order of `at`, so each stays between its neighbors
  const stopRange = (index: number): Range => ({
    min: stops.get()[index - 1]?.at ?? UNIT.min,
    max: stops.get()[index + 1]?.at ?? UNIT.max,
    step: UNIT.step,
  })

  // the stops trade places and the positions stay, as the format needs them in order of `at`
  const moveStop = (index: number, by: number) => {
    const list = stops.get()
    stops.set(moved(list, index, by).map((stop, i) => ({ ...stop, at: list[i].at })))
  }

  // in the widest gap, in the color of the stop before it, so adding one doesn't change much
  const addStop = () => {
    const list = stops.get()
    let index = 1
    for (let i = 1; i < list.length; i++) {
      if (list[i].at - list[i - 1].at > list[index].at - list[index - 1].at) index = i
    }
    const [before, after] = [list[index - 1], list[index]]
    const added: Stop = { at: ((before?.at ?? 0) + (after?.at ?? 1)) / 2, color: before?.color ?? { Fixed: WHITE } }
    stops.set([...list.slice(0, index), added, ...list.slice(index)])
  }

  return (
    <>
      <FieldRow label="Mix in">
        <Dropdown
          label="Mix in"
          options={SPACE_OPTIONS}
          value={props.lens.get().space}
          onChange={field(props.lens, 'space').set}
        />
      </FieldRow>
      <Index each={stops.get()}>
        {(_, index) => (
          <ListItem
            title={`Stop ${index + 1}`}
            list={stops}
            index={index}
            canRemove={stops.get().length > 1}
            onMove={(by) => moveStop(index, by)}
          >
            <FieldRow label="At">
              <NumberField
                label={`Stop ${index + 1} position`}
                range={stopRange(index)}
                value={stops.get()[index].at}
                onChange={field(item(stops, index), 'at').set}
              />
            </FieldRow>
            <PropField
              label="Color"
              paramName="stop_color"
              fallback={WHITE}
              ranges={COLOR_RANGES}
              lens={field(item(stops, index), 'color')}
            />
          </ListItem>
        )}
      </Index>
      <AddButton onClick={addStop}>Add stop</AddButton>
    </>
  )
}

/** A layer's shape, or with `path` the shape a sweep goes around, which can't be left out */
export function ShapeEditor(props: { lens: Lens<Shape | undefined> } | { lens: Lens<Shape>; path: true }) {
  const path = 'path' in props
  const label = path ? 'Path' : 'Shape'
  const surface = useContext(SurfaceContext)!
  const scene = useContext(SceneContext)!
  const lens = props.lens as Lens<Shape | undefined>
  const shape = props.lens as Lens<Shape>
  const base = field(shape, 'base') as Lens<ShapeNode>
  const kind = (): ShapeKind => {
    const current = lens.get()
    return current ? (Object.keys(current.base)[0] as ShapeKind) : 'None'
  }

  // placed so that it can be seen, at a size that fits
  const newShape = (next: Exclude<ShapeKind, 'None'>): Shape => {
    // a sequence runs from 0 to 1, with its points along the middle
    const [width, height] = surface() ?? [1, 0]
    const across = newShapeSize(surface())
    return {
      base: sizedShape(next, [across, across]),
      transform: [{ Scale: { Fixed: [1, 1] } }, { Translate: { Fixed: [width / 2, height / 2] } }],
    }
  }

  const setKind = (next: ShapeKind) => {
    if (next === kind()) return
    if (next === 'None') return lens.set(undefined)
    // one shape turned into another takes up the same room
    if (lens.get()) base.set(sizedShape(next, shapeSize(base.get(), scene.get())))
    else lens.set(newShape(next))
  }

  const ranges = () => sizeRanges(surface())

  return (
    <section class="scene-group">
      <FieldRow label={label}>
        <Dropdown label={label} options={path ? SHAPE_OPTIONS.slice(1) : SHAPE_OPTIONS} value={kind()} onChange={setKind} />
      </FieldRow>
      <Switch>
        <Match when={kind() === 'Ellipse'}>
          <EllipseFields ranges={ranges()} lens={variant(base, 'Ellipse')} />
        </Match>
        <Match when={kind() === 'Rect'}>
          <PropField label="Width" paramName="width" fallback={0} ranges={both(ranges()[0])} lens={field(variant(base, 'Rect'), 'width')} />
          <PropField label="Height" paramName="height" fallback={0} ranges={both(ranges()[1])} lens={field(variant(base, 'Rect'), 'height')} />
        </Match>
        <Match when={kind() === 'Polygon'}>
          <PolygonEditor lens={field(variant(base, 'Polygon'), 'points')} />
        </Match>
      </Switch>
      <Show when={kind() !== 'None'}>
        <TransformList lens={orEmpty(field(shape, 'transform'))} space="shape" />
      </Show>
    </section>
  )
}

/**
 * An ellipse's radii, and a button that locks them together into a circle. Whichever one changes takes the other with
 * it, wherever the change came from, as a handle on the shape only knows of its own radius. A file has no way to say
 * they're locked, so this lasts as long as the editor is open. While they're locked both are on the wider of their two
 * ranges, so that the same value puts their sliders in the same spot.
 */
function EllipseFields(props: { lens: Lens<EllipseRadii>; ranges: Ranges }) {
  const [locked, setLocked] = createSignal(false)
  const r1 = field(props.lens, 'r1')
  const r2 = field(props.lens, 'r2')
  const same = (a: Prop<number>, b: Prop<number>) => JSON.stringify(a) === JSON.stringify(b)

  let lastR1 = r1.get()
  createEffect(() => {
    const [first, second] = [r1.get(), r2.get()]
    if (locked() && !same(first, second)) {
      const radius = same(first, lastR1) ? second : first
      props.lens.set({ r1: radius, r2: radius })
    }
    lastR1 = first
  })

  const toggle = () => {
    if (!locked()) props.lens.set({ r1: r1.get(), r2: r1.get() })
    setLocked(!locked())
  }

  const range = (own: Range) => {
    const [across, down] = props.ranges
    return both(radius(!locked() ? own : across.max > down.max ? across : down))
  }

  const lock = (
    <button
      type="button"
      class="scene-radii-lock"
      aria-pressed={locked()}
      aria-label="Lock R1 and R2 together"
      title={locked() ? 'R1 and R2 change together, keeping this a circle' : 'Lock R1 and R2 together to make a circle'}
      onClick={toggle}
    >
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <path d="M3.5 7.5h9v6h-9z" />
        <path d={locked() ? 'M5.5 7.5V5a2.5 2.5 0 0 1 5 0v2.5' : 'M5.5 7.5V5a2.5 2.5 0 0 1 4.75-1.1'} />
      </svg>
    </button>
  )

  return (
    <>
      <PropField label="R1" paramName="r1" fallback={0} ranges={range(props.ranges[1])} lens={r1} />
      <PropField label="R2" paramName="r2" fallback={0} ranges={range(props.ranges[0])} lens={r2} action={lock} />
    </>
  )
}

/** A shape this wide and tall */
function sizedShape(kind: Exclude<ShapeKind, 'None'>, [width, height]: Vec2): ShapeNode {
  if (kind === 'Ellipse') return { Ellipse: { r1: { Fixed: height / 2 }, r2: { Fixed: width / 2 } } }
  if (kind === 'Rect') return { Rect: { width: { Fixed: width }, height: { Fixed: height } } }
  const corners: Vec2[] = [
    [-width / 2, -height / 2],
    [width / 2, -height / 2],
    [width / 2, height / 2],
    [-width / 2, height / 2],
  ]
  return { Polygon: { points: corners.map((corner) => ({ Fixed: corner })) } }
}

/** How wide and tall a shape is in its own space */
function shapeSize(base: ShapeNode, scene: Scene): Vec2 {
  const value = (prop: Prop<number>) => startingValue(prop, scene, 0)
  if ('Ellipse' in base) return [value(base.Ellipse.r2) * 2, value(base.Ellipse.r1) * 2]
  if ('Rect' in base) return [value(base.Rect.width), value(base.Rect.height)]
  const points = base.Polygon.points.map((point) => startingValue(point, scene, [0, 0] as Vec2))
  const reach = (axis: 0 | 1) => Math.max(...points.map((p) => p[axis])) - Math.min(...points.map((p) => p[axis]))
  return [reach(0), reach(1)]
}

function PolygonEditor(props: { lens: Lens<Prop<Vec2>[]> }) {
  const scene = useContext(SceneContext)!
  const surface = useContext(SurfaceContext)!
  const points = props.lens

  // halfway back to the first point, so the polygon keeps its outline
  const addPoint = () => {
    const list = points.get()
    const at = (point: Prop<Vec2>) => startingValue(point, scene.get(), [0, 0])
    const [first, last] = [at(list[0]), at(list[list.length - 1])]
    points.set([...list, { Fixed: [(first[0] + last[0]) / 2, (first[1] + last[1]) / 2] }])
  }

  return (
    <>
      <Index each={points.get()}>
        {(_, index) => (
          <ListItem title={`Point ${index + 1}`} list={points} index={index} canRemove={points.get().length > 3}>
            <PropField
              label="Position"
              paramName="point"
              fallback={[0, 0] as Vec2}
              ranges={pointRanges(surface())}
              lens={item(points, index)}
            />
          </ListItem>
        )}
      </Index>
      <AddButton onClick={addPoint}>Add point</AddButton>
    </>
  )
}

function TransformList(props: { lens: Lens<Transform[]>; space: TransformSpace }) {
  return (
    <>
      <Index each={props.lens.get()}>
        {(_, index) => (
          <ListItem title="Transform" list={props.lens} index={index}>
            <TransformEditor lens={item(props.lens, index)} space={props.space} />
          </ListItem>
        )}
      </Index>
      <AddButton onClick={() => props.lens.set([...props.lens.get(), NEW_TRANSFORMS.Translate])}>Add transform</AddButton>
    </>
  )
}

function TransformEditor(props: { lens: Lens<Transform>; space: TransformSpace }) {
  const surface = useContext(SurfaceContext)!
  const kind = () => Object.keys(props.lens.get())[0] as TransformKind
  return (
    <>
      <Dropdown
        label="Kind of transform"
        options={TRANSFORM_OPTIONS}
        value={kind()}
        onChange={(next) => {
          if (next !== kind()) props.lens.set(NEW_TRANSFORMS[next])
        }}
      />
      <Switch>
        <Match when={kind() === 'Translate'}>
          <PropField
            label="Offset"
            paramName="offset"
            fallback={[0, 0] as Vec2}
            ranges={translateRanges(surface(), props.space)}
            lens={variant(props.lens, 'Translate')}
          />
        </Match>
        <Match when={kind() === 'Rotate'}>
          <PropField
            label="Degrees"
            paramName="rotation"
            fallback={0}
            ranges={both(ROTATION)}
            lens={variant(props.lens, 'Rotate')}
          />
        </Match>
        <Match when={kind() === 'Scale'}>
          <PropField
            label="Scale"
            paramName="scale"
            fallback={[1, 1] as Vec2}
            ranges={scaleRanges()}
            lens={variant(props.lens, 'Scale')}
          />
        </Match>
      </Switch>
    </>
  )
}

function AdjustList(props: { lens: Lens<ColorAdjust[]> }) {
  return (
    <>
      <Index each={props.lens.get()}>
        {(_, index) => (
          <ListItem title="Adjust" list={props.lens} index={index}>
            <Dropdown label="Kind of adjust" options={ADJUST_OPTIONS} value="HueShift" onChange={() => {}} />
            <PropField
              label="Degrees"
              paramName="hue_shift"
              fallback={0}
              ranges={both(HUE)}
              lens={variant(item(props.lens, index), 'HueShift')}
            />
          </ListItem>
        )}
      </Index>
      <AddButton onClick={() => props.lens.set([...props.lens.get(), { HueShift: { Fixed: 0 } }])}>Add adjust</AddButton>
    </>
  )
}

type ItemButtonsProps<T> = {
  title: string
  list: Lens<T[]>
  index: number
  canRemove?: boolean
  /** For a list that can't just be rearranged */
  onMove?: (by: number) => void
  onRemove?: () => void
}

/** A thing in a list, with the buttons for moving and removing it */
function ListItem<T>(props: ItemButtonsProps<T> & { children: any }) {
  return (
    <div class="scene-item">
      <header>
        <span>{props.title}</span>
        <ItemButtons {...props} />
      </header>
      {props.children}
    </div>
  )
}

function ItemButtons<T>(props: ItemButtonsProps<T>) {
  const move = (by: number) => {
    if (props.onMove) props.onMove(by)
    else props.list.set(moved(props.list.get(), props.index, by))
  }
  const remove = () => {
    if (props.onRemove) props.onRemove()
    else props.list.set(removed(props.list.get(), props.index))
  }
  return (
    <>
      <button
        type="button"
        aria-label={`Move ${props.title} up`}
        disabled={props.index === 0}
        onClick={() => move(-1)}
      >
        ↑
      </button>
      <button
        type="button"
        aria-label={`Move ${props.title} down`}
        disabled={props.index === props.list.get().length - 1}
        onClick={() => move(1)}
      >
        ↓
      </button>
      <button type="button" aria-label={`Remove ${props.title}`} disabled={props.canRemove === false} onClick={remove}>
        ×
      </button>
    </>
  )
}

function AddButton(props: { onClick: () => void; children: any }) {
  return (
    <button type="button" class="scene-add" onClick={props.onClick}>
      {props.children}
    </button>
  )
}
