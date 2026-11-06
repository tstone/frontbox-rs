import { Accordion } from '@ark-ui/solid/accordion'
import { Index, Match, Show, Switch, useContext, type Signal } from 'solid-js'
import type { Color } from '../../types/generated/Color'
import type { ColorAdjust } from '../../types/generated/ColorAdjust'
import type { ColorSpace } from '../../types/generated/ColorSpace'
import type { Layer } from '../../types/generated/Layer'
import type { Prop } from '../../types/generated/Prop'
import type { Scene } from '../../types/generated/Scene'
import type { ShapeNode } from '../../types/generated/ShapeNode'
import type { Stop } from '../../types/generated/Stop'
import type { Warp } from '../../types/generated/Warp'
import { Dropdown, FieldRow, NumberField, PropField, SceneContext, SurfaceContext } from './SceneFields'
import {
  both,
  field,
  HUE,
  item,
  moved,
  orEmpty,
  POLYGON_POINT,
  removed,
  ROTATION,
  scaleRanges,
  startingValue,
  translateRanges,
  UNIT,
  variant,
  type Lens,
  type Range,
  type Vec2,
  type WarpSpace,
} from './sceneModel'

type Paint = Layer['paint']
type Shape = NonNullable<Layer['shape']>

const WHITE: Color = [255, 255, 255, 255]
const BLACK: Color = [0, 0, 0, 255]
const COLOR_RANGES = both(UNIT)

const VIEWPORT_OPTIONS: { value: 'Sequence' | 'Area'; label: string }[] = [
  { value: 'Sequence', label: 'Sequence (1D)' },
  { value: 'Area', label: 'Area (2D)' },
]
const PAINT_OPTIONS: { value: 'Solid' | 'Gradient'; label: string }[] = [
  { value: 'Solid', label: 'Solid' },
  { value: 'Gradient', label: 'Gradient' },
]
const SPACE_OPTIONS: { value: ColorSpace; label: string }[] = [
  { value: 'Oklab', label: 'Oklab' },
  { value: 'Rgb', label: 'RGB' },
]
const SHAPE_OPTIONS: { value: 'None' | 'Ellipse' | 'Polygon'; label: string }[] = [
  { value: 'None', label: 'Whole viewport' },
  { value: 'Ellipse', label: 'Ellipse' },
  { value: 'Polygon', label: 'Polygon' },
]
const WARP_OPTIONS: { value: WarpKind; label: string }[] = [
  { value: 'Translate', label: 'Translate' },
  { value: 'Rotate', label: 'Rotate' },
  { value: 'Scale', label: 'Scale' },
]
const ADJUST_OPTIONS: { value: 'HueShift'; label: string }[] = [{ value: 'HueShift', label: 'Hue shift' }]

type WarpKind = 'Translate' | 'Rotate' | 'Scale'

/** A warp that leaves things where they are, until it's changed */
const NEW_WARPS: Record<WarpKind, Warp> = {
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
    layers.set([...layers.get(), { paint: { base: { Solid: { Fixed: WHITE } } } }])
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
          {(_, index) => (
            <Accordion.Item class="scene-item" value={String(index)}>
              <header>
                <Accordion.ItemTrigger>
                  <Accordion.ItemIndicator>›</Accordion.ItemIndicator>
                  Layer {index + 1}
                </Accordion.ItemTrigger>
                <ItemButtons
                  title={`Layer ${index + 1}`}
                  list={layers}
                  index={index}
                  onMove={(by) => move(index, by)}
                  onRemove={() => remove(index)}
                />
              </header>
              <Accordion.ItemContent>
                <div class="scene-config">
                  <LayerEditor lens={item(layers, index)} />
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
      <FieldRow label="Viewport">
        <Dropdown
          label="Viewport"
          options={VIEWPORT_OPTIONS}
          value={viewport.get()}
          onChange={(value) => {
            if (value === viewport.get()) return
            if (value === 'Area') viewport.set('Area')
            else scene.set(asSequence(scene.get()))
          }}
        />
      </FieldRow>
    </section>
  )
}

/** A sequence's layers cover all of it, as shapes can't be edited on one */
function asSequence(scene: Scene): Scene {
  return { ...scene, viewport: 'Sequence', layers: scene.layers.map(({ paint }) => ({ paint })) }
}

function LayerEditor(props: { lens: Lens<Layer> }) {
  const scene = useContext(SceneContext)!
  return (
    <>
      <Show when={scene.get().viewport !== 'Sequence'}>
        <ShapeEditor lens={field(props.lens, 'shape')} />
      </Show>
      <PaintEditor lens={field(props.lens, 'paint')} />
    </>
  )
}

function PaintEditor(props: { lens: Lens<Paint> }) {
  const base = field(props.lens, 'base')
  const isSolid = () => 'Solid' in base.get()

  const setKind = (kind: 'Solid' | 'Gradient') => {
    const current = base.get()
    if (kind === 'Solid' && 'Gradient' in current) {
      base.set({ Solid: current.Gradient.stops[0]?.color ?? { Fixed: WHITE } })
    } else if (kind === 'Gradient' && 'Solid' in current) {
      const stops = [
        { at: 0, color: current.Solid },
        { at: 1, color: { Fixed: BLACK } },
      ]
      base.set({ Gradient: { stops, space: 'Oklab' } })
    }
  }

  return (
    <section class="scene-group">
      <FieldRow label="Paint">
        <Dropdown label="Paint" options={PAINT_OPTIONS} value={isSolid() ? 'Solid' : 'Gradient'} onChange={setKind} />
      </FieldRow>
      <Show when={isSolid()} fallback={<GradientEditor lens={variant(base, 'Gradient')} />}>
        <PropField label="Color" paramName="color" fallback={WHITE} ranges={COLOR_RANGES} lens={variant(base, 'Solid')} />
      </Show>
      <WarpList lens={orEmpty(field(props.lens, 'warp'))} space="paint" />
      <AdjustList lens={orEmpty(field(props.lens, 'adjust'))} />
    </section>
  )
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

function ShapeEditor(props: { lens: Lens<Shape | undefined> }) {
  const surface = useContext(SurfaceContext)!
  const shape = props.lens as Lens<Shape>
  const base = field(shape, 'base')
  const kind = () => {
    const current = props.lens.get()
    if (!current) return 'None'
    return current.base === 'Ellipse' ? 'Ellipse' : 'Polygon'
  }

  // sized and placed so that it can be seen, as a shape has neither of its own
  const newShape = (shapeBase: ShapeNode): Shape => {
    // only an Area has shapes to edit
    const [width, height] = surface()!
    const across = Math.min(width, height) / 2
    return {
      base: shapeBase,
      warp: [{ Scale: { Fixed: [across, across] } }, { Translate: { Fixed: [width / 2, height / 2] } }],
    }
  }

  const setKind = (next: 'None' | 'Ellipse' | 'Polygon') => {
    if (next === kind()) return
    if (next === 'None') return props.lens.set(undefined)
    const nextBase: ShapeNode = next === 'Ellipse' ? 'Ellipse' : { Polygon: { points: unitSquare() } }
    if (props.lens.get()) base.set(nextBase)
    else props.lens.set(newShape(nextBase))
  }

  return (
    <section class="scene-group">
      <FieldRow label="Shape">
        <Dropdown label="Shape" options={SHAPE_OPTIONS} value={kind()} onChange={setKind} />
      </FieldRow>
      <Show when={kind() === 'Polygon'}>
        <PolygonEditor lens={field(variant(base as Lens<ShapeNode>, 'Polygon'), 'points')} />
      </Show>
      <Show when={kind() !== 'None'}>
        <WarpList lens={orEmpty(field(shape, 'warp'))} space="shape" />
      </Show>
    </section>
  )
}

function unitSquare(): Prop<Vec2>[] {
  const corners: Vec2[] = [
    [-0.5, -0.5],
    [0.5, -0.5],
    [0.5, 0.5],
    [-0.5, 0.5],
  ]
  return corners.map((corner) => ({ Fixed: corner }))
}

function PolygonEditor(props: { lens: Lens<Prop<Vec2>[]> }) {
  const scene = useContext(SceneContext)!
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
              ranges={both(POLYGON_POINT)}
              lens={item(points, index)}
            />
          </ListItem>
        )}
      </Index>
      <AddButton onClick={addPoint}>Add point</AddButton>
    </>
  )
}

function WarpList(props: { lens: Lens<Warp[]>; space: WarpSpace }) {
  return (
    <>
      <Index each={props.lens.get()}>
        {(_, index) => (
          <ListItem title="Warp" list={props.lens} index={index}>
            <WarpEditor lens={item(props.lens, index)} space={props.space} />
          </ListItem>
        )}
      </Index>
      <AddButton onClick={() => props.lens.set([...props.lens.get(), NEW_WARPS.Translate])}>Add warp</AddButton>
    </>
  )
}

function WarpEditor(props: { lens: Lens<Warp>; space: WarpSpace }) {
  const surface = useContext(SurfaceContext)!
  const kind = (): WarpKind => {
    const warp = props.lens.get()
    return 'Translate' in warp ? 'Translate' : 'Rotate' in warp ? 'Rotate' : 'Scale'
  }
  return (
    <>
      <Dropdown
        label="Kind of warp"
        options={WARP_OPTIONS}
        value={kind()}
        onChange={(next) => {
          if (next !== kind()) props.lens.set(NEW_WARPS[next])
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
            ranges={scaleRanges(surface(), props.space)}
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
