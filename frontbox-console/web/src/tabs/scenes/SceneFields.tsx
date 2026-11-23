import { ColorPicker, parseColor, type Color as PickerColor } from '@ark-ui/solid/color-picker'
import { Menu } from '@ark-ui/solid/menu'
import { Dynamic } from 'solid-js/web'
import { createListCollection, Select } from '@ark-ui/solid/select'
import { Slider } from '@ark-ui/solid/slider'
import { Switch as Toggle } from '@ark-ui/solid/switch'
import { createContext, createEffect, createMemo, createSignal, For, Match, Show, splitProps, Switch, useContext, type Accessor, type JSX, type Signal } from 'solid-js'
import ScenePortal, { layerColorClass, type LayerColor } from './ScenePortal'
import InfoTip from '../../components/InfoTip'
import { AudioWaveform, ChartSpline, Dices, Eye, EyeOff, Footprints, Orbit, Pin, Pipette, Variable } from '../../components/icons'
import type { Color } from '../../types/generated/Color'
import { NAMED_COLORS } from '../../types/generated/namedColors'
import type { Prop } from '../../types/generated/Prop'
import type { RenderedShape } from '../../types/generated/RenderedShape'
import type { Layer } from '../../types/generated/Layer'
import type { Curve } from '../../types/generated/Curve'
import type { EnvelopeCurve } from '../../types/generated/EnvelopeCurve'
import type { Modulated } from '../../types/generated/Modulated'
import type { Overlap } from '../../types/generated/Overlap'
import type { Spread } from '../../types/generated/Spread'
import type { VisitResponse } from '../../types/generated/VisitResponse'
import CurvePicker, { CurveGraph, CURVES, CUSTOM_CURVE } from './CurvePicker'
import { curveHeight, field, freshVariableName, fullSpread, HUE, newPick, newSweep, newVisit, variableReads, spreadFits, startingValue, STEPS, UNIT, variant, WAVE_RATE, withVariable, withTiming, type Lens, type Range, type Ranges, type LayerSurfaces, type Scene, type SceneValue, type Selection, type Surface, type Vec2 } from './sceneModel'

/** The scene being edited, for fields whose edits reach past their own place in it */
export const SceneContext = createContext<Lens<Scene>>()

/** The size the scene is being edited for */
export const SurfaceContext = createContext<Accessor<Surface>>()
export const LayerSurfacesContext = createContext<LayerSurfaces>()

/** The names of the traversals in the layer being edited, which a prop can respond to the visits of */
export const TraversalsContext = createContext<Accessor<string[]>>(() => [])

/** Choosing a node of the open layer's graph by clicking it in the preview */
export type NodePicking = {
  /** The lights in the open layer's graph, by name, as the machine last built it */
  nodes: Accessor<string[]>
  /** What a click in the preview chooses a node for, while one is being chosen */
  choosing: Signal<{ choose: (name: string) => void } | null>
  /** The node to point out in the preview */
  highlighted: Signal<string | null>
}

export const NodePickingContext = createContext<NodePicking>()

/**
 * Whether a layer's or mask's region, or its graph, is drawn over the preview while it's selected. Only for the editing,
 * so it isn't part of the scene and hiding it changes nothing the scene shows.
 */
export type RegionVisibility = { shown: (of: Selection) => boolean; show: (of: Selection, shown: boolean) => void }

export const RegionVisibilityContext = createContext<RegionVisibility>()

/**
 * Where a layer's or mask's region stands as the machine last drew it, at the playhead, for fields that need to know
 * where the shape is. `of` is `null` for a shape that isn't a region, or that hasn't been drawn.
 */
export type ShapePoses = { of: (shape: NonNullable<Layer['region']>) => RenderedShape | null; time: Accessor<number> }

export const ShapePosesContext = createContext<ShapePoses>({ of: () => null, time: () => 0 })

function useScene() {
  return useContext(SceneContext)!
}

/** A value's control and the label above it */
export function FieldRow(props: { label: string; children: any; /** Beside the label rather than under it, for a small control */ inline?: boolean }) {
  return (
    <div class="scene-field" classList={{ 'scene-field-inline': props.inline }}>
      <span class="scene-field-label">{props.label}</span>
      {props.children}
    </div>
  )
}

/** Shows or hides a layer or a track */
export function EyeButton(props: { label: string; hidden: boolean; onChange: (hidden: boolean) => void }) {
  const action = () => `${props.hidden ? 'Show' : 'Hide'} ${props.label}`
  return (
    <button
      type="button"
      class="scene-eye"
      aria-label={action()}
      aria-pressed={props.hidden}
      title={action()}
      onClick={() => props.onChange(!props.hidden)}
    >
      <Show when={props.hidden} fallback={<Eye aria-hidden="true" />}>
        <EyeOff aria-hidden="true" />
      </Show>
    </button>
  )
}

/**
 * After a choice in the sidebar that can add fields below it, such as a fill's type or a prop's source, scrolls the
 * sidebar down to show the rest of the section it's in, as far as that keeps the choice itself in view.
 */
export function revealBelow(choice: Element) {
  // after the fields the choice adds have been laid out
  requestAnimationFrame(() => {
    if (!choice.isConnected || !choice.closest('.scene-sidebar-tabs')) return
    const section = choice.closest('.scene-prop-under, .scene-part, .scene-prop, .scene-properties-node')
    const scroller = scrollParent(choice)
    if (!section || !scroller) return
    const view = scroller.getBoundingClientRect()
    const MARGIN = 12
    const past = section.getBoundingClientRect().bottom + MARGIN - view.bottom
    const room = choice.getBoundingClientRect().top - MARGIN - view.top
    // the sticky tabs cover the top of the sidebar
    const tabs = scroller.querySelector('.scene-sidebar-tab-list')?.getBoundingClientRect().height ?? 0
    const by = Math.min(past, room - tabs)
    if (by > 0) scroller.scrollBy({ top: by, behavior: 'smooth' })
  })
}

/** The nearest element above one that scrolls */
function scrollParent(element: Element): Element | null {
  for (let parent = element.parentElement; parent; parent = parent.parentElement) {
    const overflow = getComputedStyle(parent).overflowY
    if ((overflow === 'auto' || overflow === 'scroll') && parent.scrollHeight > parent.clientHeight) return parent
  }
  return null
}

/** Picks one of an enum's values. Options with a `group` are listed under its name, with the groups in the order they first come. */
export function Dropdown<T extends string>(props: {
  label: string
  /** `color` marks an option in the color of the layers it's for */
  options: { value: T; label: string; group?: string; color?: LayerColor }[]
  value: T
  onChange: (value: T) => void
  /** As wide as its value rather than its row, with a list as wide as the options need */
  compact?: boolean
}) {
  let root!: HTMLDivElement
  const collection = createMemo(() => createListCollection({ items: props.options }))
  const groups = createMemo(() => [...new Set(props.options.map((option) => option.group))])
  const inGroup = (group: string | undefined) => collection().items.filter((option) => option.group === group)
  const item = (option: { value: T; label: string; color?: LayerColor }) => (
    <Select.Item item={option} class={layerColorClass(option.color)}>
      <Select.ItemText>{option.label}</Select.ItemText>
      <Select.ItemIndicator>
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path d="M3 8.5l3.5 3.5L13 5" />
        </svg>
      </Select.ItemIndicator>
    </Select.Item>
  )
  return (
    <Select.Root
      ref={root}
      class="scene-dropdown"
      classList={{ 'scene-dropdown-compact': props.compact }}
      collection={collection()}
      value={[props.value]}
      onValueChange={(details) => {
        props.onChange(details.value[0] as T)
        revealBelow(root)
      }}
      positioning={props.compact ? { placement: 'bottom-end' } : { sameWidth: true }}
    >
      <Select.Control>
        <Select.Trigger aria-label={props.label}>
          <Select.ValueText />
          <Select.Indicator>
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <path d="M4 6l4 4 4-4" />
            </svg>
          </Select.Indicator>
        </Select.Trigger>
      </Select.Control>
      <ScenePortal>
        <Select.Positioner>
          <Select.Content class="scene-dropdown-list">
            <For each={groups()}>
              {(group) => (
                <Show when={group !== undefined} fallback={<For each={inGroup(undefined)}>{item}</For>}>
                  <Select.ItemGroup>
                    <Select.ItemGroupLabel>{group}</Select.ItemGroupLabel>
                    <For each={inGroup(group)}>{item}</For>
                  </Select.ItemGroup>
                </Show>
              )}
            </For>
          </Select.Content>
        </Select.Positioner>
      </ScenePortal>
      <Select.HiddenSelect />
    </Select.Root>
  )
}

/**
 * A box for typing a number, which is given each number as it's typed. Writing to a box moves its caret and drops text
 * like `0.` that's on its way to a number, so it's only written when the value becomes something it doesn't already say,
 * and tidied up when it's left.
 */
export function NumberBox(props: Omit<JSX.InputHTMLAttributes<HTMLInputElement>, 'value'> & { value: number; onNumber: (value: number) => void }) {
  const [own, input] = splitProps(props, ['value', 'onNumber'])
  const value = createMemo(() => own.value)
  let box!: HTMLInputElement

  createEffect(() => {
    if (box.valueAsNumber !== value()) box.value = String(value())
  })

  return (
    <input
      {...input}
      ref={box}
      type="number"
      onInput={() => {
        if (Number.isFinite(box.valueAsNumber)) own.onNumber(box.valueAsNumber)
      }}
      onBlur={() => (box.value = String(value()))}
    />
  )
}

/**
 * A slider and a box for typing a value. A value from the box or the file past the range widens the slider, and double
 * clicking the slider's handle resets it to the range's `rest`.
 */
export function NumberField(props: { label: string; value: number; onChange: (value: number) => void; range: Range }) {
  return (
    <div class="scene-number">
      <Slider.Root
        aria-label={[props.label]}
        min={Math.min(props.range.min, props.value)}
        max={Math.max(props.range.max, props.value)}
        step={props.range.step}
        value={[props.value]}
        onValueChange={(details) => {
          if (details.value[0] !== props.value) props.onChange(details.value[0])
        }}
      >
        <Slider.Control>
          <Slider.Track>
            <Slider.Range />
          </Slider.Track>
          <Slider.Thumb
            index={0}
            onDblClick={() => {
              if (props.range.rest !== undefined) props.onChange(props.range.rest)
            }}
          >
            <Slider.HiddenInput />
          </Slider.Thumb>
        </Slider.Control>
      </Slider.Root>
      <NumberBox aria-label={props.label} step={props.range.step} value={props.value} onNumber={props.onChange} />
    </div>
  )
}

/**
 * The lowest and highest of a span, as a slider with two ends and a box for typing each. As with `NumberField`, a value
 * from a box or the file past the range widens the slider. `track` draws the slider's track for those limits.
 */
function RangeField(props: {
  label: string
  value: [number, number]
  onChange: (value: [number, number]) => void
  range: Range
  track?: (min: number, max: number) => string
}) {
  const ends = () => [Math.min(...props.value), Math.max(...props.value)]
  const min = () => Math.min(props.range.min, ends()[0])
  const max = () => Math.max(props.range.max, ends()[1])

  const box = (index: 0 | 1, name: string) => (
    <NumberBox
      aria-label={`${props.label} ${name}`}
      step={props.range.step}
      value={props.value[index]}
      onNumber={(typed) => {
        const next: [number, number] = [...props.value]
        next[index] = typed
        props.onChange(next)
      }}
    />
  )

  return (
    <div class="scene-number scene-range" classList={{ 'scene-range-drawn': props.track !== undefined }}>
      {box(0, 'lowest')}
      <Slider.Root
        aria-label={[`${props.label} lowest`, `${props.label} highest`]}
        min={min()}
        max={max()}
        step={props.range.step}
        value={ends()}
        onValueChange={({ value: [low, high] }) => {
          if (low !== ends()[0] || high !== ends()[1]) props.onChange([low, high])
        }}
      >
        <Slider.Control>
          <Slider.Track style={{ background: props.track?.(min(), max()) }}>
            <Slider.Range />
          </Slider.Track>
          <Slider.Thumb index={0}>
            <Slider.HiddenInput />
          </Slider.Thumb>
          <Slider.Thumb index={1}>
            <Slider.HiddenInput />
          </Slider.Thumb>
        </Slider.Control>
      </Slider.Root>
      {box(1, 'highest')}
    </div>
  )
}

/** The hues from `min` to `max`, which can be more than one turn or start below zero */
function hueTrack(min: number, max: number) {
  const stops = Array.from({ length: 7 }, (_, i) => `hsl(${min + ((max - min) * i) / 6} 100% 50%)`)
  return `linear-gradient(to right, ${stops.join(', ')})`
}

const brightnessTrack = () => 'linear-gradient(to right, #000, #fff)'

const opacityTrack = () => 'linear-gradient(to right, transparent, #fff)'

/** What a random prop can pick from, or a wave moves through, with a control for each thing about it that varies */
function SpreadField(props: { label: string; lens: Lens<Spread>; ranges: Ranges }) {
  const spread = () => props.lens.get()
  const float = () => variant(props.lens, 'Float')
  const vec2 = () => variant(props.lens, 'Vec2')
  const color = () => variant(props.lens, 'Color')

  const axis = (name: 'x' | 'y', index: 0 | 1) => (
    <div class="scene-axis">
      <span aria-hidden="true">{name}</span>
      <RangeField
        label={`${props.label} ${name}`}
        range={props.ranges[index]}
        value={field(vec2(), name).get()}
        onChange={field(vec2(), name).set}
      />
    </div>
  )

  return (
    <Switch>
      <Match when={'Float' in spread()}>
        <RangeField label={props.label} range={props.ranges[0]} value={float().get()} onChange={float().set} />
      </Match>
      <Match when={'Vec2' in spread()}>
        {axis('x', 0)}
        {axis('y', 1)}
      </Match>
      <Match when={'Color' in spread()}>
        <FieldRow label="Hue">
          <RangeField
            label={`${props.label} hue`}
            range={HUE}
            track={hueTrack}
            value={field(color(), 'hue').get()}
            onChange={field(color(), 'hue').set}
          />
        </FieldRow>
        <FieldRow label="Brightness">
          <RangeField
            label={`${props.label} brightness`}
            range={UNIT}
            track={brightnessTrack}
            value={field(color(), 'brightness').get()}
            onChange={field(color(), 'brightness').set}
          />
        </FieldRow>
        <FieldRow label="Opacity">
          <RangeField
            label={`${props.label} opacity`}
            range={UNIT}
            track={opacityTrack}
            value={field(color(), 'alpha').get()}
            onChange={field(color(), 'alpha').set}
          />
        </FieldRow>
      </Match>
    </Switch>
  )
}

function Vec2Field(props: { label: string; lens: Lens<Vec2>; ranges: Ranges }) {
  const axis = (index: 0 | 1, name: string) => (
    <div class="scene-axis">
      <span aria-hidden="true">{name}</span>
      <NumberField
        label={`${props.label} ${name}`}
        range={props.ranges[index]}
        value={props.lens.get()[index]}
        onChange={(value) => {
          const next: Vec2 = [...props.lens.get()]
          next[index] = value
          props.lens.set(next)
        }}
      />
    </div>
  )
  return (
    <>
      {axis(0, 'x')}
      {axis(1, 'y')}
    </>
  )
}

/** A swatch that opens the picker, which also takes a hex value */
export function ColorField(props: { label: string; lens: Lens<Color>; class?: string }) {
  // what the picker last chose. The scene only keeps red, green, and blue, which lose the hue of a white, a black, or a
  // gray, so the picker keeps its own color for as long as the scene still agrees with it. Its format has to be one with
  // a hue too, as it turns the value it's given into that format.
  const [picked, setPicked] = createSignal<PickerColor>()
  const value = () => {
    const color = picked()
    if (color && toScene(color).every((channel, i) => channel === props.lens.get()[i])) return color
    const [r, g, b, a] = props.lens.get()
    return parseColor(`rgba(${r}, ${g}, ${b}, ${a / 255})`)
  }

  const channelSlider = (channel: 'hue' | 'alpha') => (
    <ColorPicker.ChannelSlider channel={channel}>
      <ColorPicker.ChannelSliderTrack />
      <ColorPicker.ChannelSliderThumb />
    </ColorPicker.ChannelSlider>
  )

  return (
    <ColorPicker.Root
      class={props.class ?? 'scene-color'}
      format="hsba"
      value={value()}
      onValueChange={(details) => {
        setPicked(details.value)
        props.lens.set(toScene(details.value))
      }}
    >
      <ColorPicker.Control>
        <ColorPicker.Trigger aria-label={props.label}>
          <ColorPicker.ValueSwatch />
        </ColorPicker.Trigger>
      </ColorPicker.Control>
      <ScenePortal>
        <ColorPicker.Positioner>
          <ColorPicker.Content class="scene-color-picker">
            <ColorPicker.Area>
              <ColorPicker.AreaBackground />
              <ColorPicker.AreaThumb />
            </ColorPicker.Area>
            {channelSlider('hue')}
            {channelSlider('alpha')}
            <div class="scene-color-hex">
              <ColorPicker.ChannelInput channel="hex" aria-label={`${props.label} as hex`} />
              {/* takes a color from anywhere on screen, such as a pixel of an image in the preview, where the browser can */}
              <Show when={'EyeDropper' in window}>
                <ColorPicker.EyeDropperTrigger class="scene-icon-button scene-eyedropper" aria-label="Pick a color from the screen" title="Pick a color from the screen, such as from the preview">
                  <Pipette aria-hidden="true" />
                </ColorPicker.EyeDropperTrigger>
              </Show>
            </div>
            <ColorPicker.SwatchGroup>
              <For each={NAMED_COLORS}>
                {({ name, hex }) => (
                  <ColorPicker.SwatchTrigger value={hex} title={name} aria-label={name}>
                    <ColorPicker.Swatch value={hex} />
                  </ColorPicker.SwatchTrigger>
                )}
              </For>
            </ColorPicker.SwatchGroup>
          </ColorPicker.Content>
        </ColorPicker.Positioner>
      </ScenePortal>
      <ColorPicker.HiddenInput />
    </ColorPicker.Root>
  )
}

function toScene(color: PickerColor): Color {
  const rgba = color.toFormat('rgba')
  const channel = (name: 'red' | 'green' | 'blue') => Math.round(rgba.getChannelValue(name))
  return [channel('red'), channel('green'), channel('blue'), Math.round(rgba.getChannelValue('alpha') * 255)]
}

/** The control for a value of any type. Both a fixed prop and a variable's default use this. */
export function ValueField(props: { label: string; lens: Lens<SceneValue>; ranges: Ranges }) {
  const kind = () => {
    const value = props.lens.get()
    if (typeof value === 'boolean') return 'bool'
    if (typeof value === 'number') return 'number'
    return value.length === 2 ? 'vec2' : 'color'
  }
  return (
    <Switch>
      <Match when={kind() === 'bool'}>
        <Toggle.Root class="scene-switch" checked={props.lens.get() as boolean} onCheckedChange={(details) => props.lens.set(details.checked)}>
          <Toggle.Control>
            <Toggle.Thumb />
          </Toggle.Control>
          <Toggle.HiddenInput aria-label={props.label} />
        </Toggle.Root>
      </Match>
      <Match when={kind() === 'number'}>
        <NumberField
          label={props.label}
          range={props.ranges[0]}
          value={props.lens.get() as number}
          onChange={props.lens.set}
        />
      </Match>
      <Match when={kind() === 'vec2'}>
        <Vec2Field label={props.label} lens={props.lens as Lens<Vec2>} ranges={props.ranges} />
      </Match>
      <Match when={kind() === 'color'}>
        <ColorField label={props.label} lens={props.lens as Lens<Color>} />
      </Match>
    </Switch>
  )
}

export type PropKind = 'Fixed' | 'Variable' | 'Keyframes' | 'Random' | 'Modulated' | 'Sweep' | 'Visit'
const PROP_KINDS: PropKind[] = ['Fixed', 'Variable', 'Keyframes', 'Random', 'Modulated', 'Sweep', 'Visit']

export const PROP_KIND_LABELS: Record<PropKind, string> = {
  Fixed: 'Fixed',
  Variable: 'Variable',
  Keyframes: 'Keyframes',
  Random: 'Random',
  Modulated: 'Modulated',
  Sweep: 'Sweep',
  Visit: 'Visit',
}

/** A prop's icon says where its value comes from */
export const SOURCE_ICONS: Record<PropKind, typeof Pin> = {
  Fixed: Pin,
  Variable: Variable,
  Keyframes: ChartSpline,
  Random: Dices,
  Modulated: AudioWaveform,
  Sweep: Orbit,
  Visit: Footprints,
}

/** What each source does, in the menu that picks one */
const PROP_KIND_HINTS: Record<PropKind, string> = {
  Fixed: 'One value that stays the same',
  Variable: 'Read from a variable the game sets',
  Keyframes: 'Changes over time, by keys on its timeline row',
  Random: 'A value picked at random, at times on its timeline row',
  Modulated: 'Moves back and forth in a wave',
  Sweep: 'Follows a path, drawn under it',
  Visit: 'Reacts when a traversal reaches a light',
}

export const propKind = (prop: Prop<SceneValue>) => Object.keys(prop)[0] as PropKind

type PropProps<T extends SceneValue> = {
  label: string
  /** What to call a variable made from this prop */
  variableName: string
  lens: Lens<Prop<T>>
  /** For a prop with nothing to take a value from */
  fallback: T
  ranges: Ranges
}

/**
 * A prop: whether it's fixed, read from a variable, keyframed, picked at random, moved in a wave, swept along a path,
 * or set off by visits, and the control for that. A sweep's path is under the prop's row in the tree, and selected there to edit it.
 */
export function PropField<T extends SceneValue>(
  props: PropProps<T> & {
    /** Leaves out the label and where the value comes from, for a field under a row that already has them */
    bare?: boolean
  },
) {
  const scene = useScene()
  const prop = () => props.lens.get()
  const kind = () => propKind(prop())
  const value = () => startingValue(prop(), scene.get(), props.fallback)
  const variableName = () => (prop() as { Variable: string }).Variable

  const rename = (name: string) => {
    if (name === '' || name === variableName()) return
    if (scene.get().variables?.[name] === undefined) scene.set(withVariable(scene.get(), name, value()))
    props.lens.set({ Variable: name })
  }

  return (
    <div class="scene-field scene-prop">
      <Show when={!props.bare}>
        <div class="scene-prop-header">
          <span class="scene-prop-name">
            <PropSource {...props} />
            <span class="scene-field-label">{props.label}</span>
          </span>
        </div>
      </Show>
      <Switch>
        <Match when={kind() === 'Fixed'}>
          <ValueField
            label={props.label}
            ranges={props.ranges}
            lens={{ get: () => value(), set: (next) => props.lens.set({ Fixed: next as T }) }}
          />
        </Match>
        <Match when={kind() === 'Variable'}>
          <input
            class="scene-variable-name"
            aria-label={`${props.label} variable name`}
            spellcheck={false}
            value={variableName()}
            onChange={(e) => {
              rename(e.currentTarget.value.trim())
              e.currentTarget.value = variableName()
            }}
          />
          <Show when={!variableReads(scene.get(), variableName(), props.fallback)}>
            <p class="scene-note scene-note-warn">No variable of this type has that name, so this reads as zero.</p>
          </Show>
        </Match>
        <Match when={kind() === 'Keyframes'}>
          <p class="scene-note">Its keyframes are on its row in the timeline.</p>
        </Match>
        <Match when={kind() === 'Random'}>
          <SpreadField
            label={props.label}
            ranges={props.ranges}
            lens={field(variant(props.lens, 'Random'), 'spread')}
          />
          <Show when={!spreadFits((prop() as { Random: { spread: Spread } }).Random.spread, props.fallback)}>
            <p class="scene-note scene-note-warn">This range is for another type of value, so this reads as zero.</p>
          </Show>
          <ForceUniqueField label={props.label} lens={field(variant(props.lens, 'Random'), 'force_unique')} />
          <p class="scene-note">When to pick a new value is on its row in the timeline.</p>
        </Match>
        <Match when={kind() === 'Modulated'}>
          <ModulatedField label={props.label} ranges={props.ranges} fallback={props.fallback} lens={variant(props.lens as Lens<Prop<SceneValue>>, 'Modulated')} />
        </Match>
        <Match when={kind() === 'Sweep'}>
          <p class="scene-note">Its path is under it, and how far along the path it is, from 0 to 1. Keyframing that from 0 to 1 goes around once.</p>
        </Match>
        <Match when={kind() === 'Visit'}>
          <VisitField label={props.label} ranges={props.ranges} lens={variant(props.lens as Lens<Prop<SceneValue>>, 'Visit')} />
        </Match>
      </Switch>
    </div>
  )
}

/**
 * Where a prop takes its value from, as an icon that opens a menu of the others. Switching it starts the new way at the
 * value the prop starts at now.
 */
export function PropSource<T extends SceneValue>(props: PropProps<T>) {
  const scene = useScene()
  const prop = () => props.lens.get()
  const kind = () => propKind(prop())
  const value = () => startingValue(prop(), scene.get(), props.fallback)
  // only a position can follow a path, though a scene that has one anywhere else can still be switched off it
  const isPosition = () => Array.isArray(props.fallback) && props.fallback.length === 2
  // visits are only there to respond to in a layer with traversals
  const traversals = useContext(TraversalsContext)
  const kinds = () =>
    PROP_KINDS.filter((option) => (option !== 'Sweep' || isPosition() || kind() === 'Sweep') && (option !== 'Visit' || traversals().length > 0 || kind() === 'Visit'))

  const choose = (next: PropKind) => {
    if (next === kind()) return
    const current = value()
    if (next === 'Fixed') props.lens.set({ Fixed: current })
    if (next === 'Keyframes') {
      scene.set(withTiming(scene.get()))
      props.lens.set({ Keyframes: { keys: [{ at: 0, value: current, curve: 'Linear' }], cycle: 'Once' } })
    }
    if (next === 'Random') {
      scene.set(withTiming(scene.get()))
      const seed = Math.floor(Math.random() * 2 ** 32)
      props.lens.set({ Random: { picks: [newPick(0)], spread: fullSpread(current, props.ranges), seed, force_unique: true } })
    }
    if (next === 'Modulated') {
      scene.set(withTiming(scene.get()))
      props.lens.set({ Modulated: { envelope: { attack: { curve: 'Linear' } }, spread: fullSpread(current, props.ranges), rate: 1, phase: { Fixed: 0 } } })
    }
    if (next === 'Sweep') {
      const timed = withTiming(scene.get())
      scene.set(timed)
      props.lens.set({ Sweep: newSweep(current as Vec2, props.ranges, timed.timing!.duration) })
    }
    if (next === 'Variable') {
      const name = freshVariableName(scene.get(), props.variableName)
      scene.set(withVariable(scene.get(), name, current))
      props.lens.set({ Variable: name })
    }
    if (next === 'Visit') {
      scene.set(withTiming(scene.get()))
      props.lens.set({ Visit: newVisit(traversals()[0] ?? '', current, props.ranges) as VisitResponse<T> })
    }
  }

  let trigger!: HTMLButtonElement
  return (
    <Menu.Root positioning={{ placement: 'bottom-start' }}>
      <Menu.Trigger ref={trigger} class="scene-icon-button scene-prop-source" data-kind={kind()} aria-label={`${props.label} source: ${PROP_KIND_LABELS[kind()]}`} title={`${PROP_KIND_LABELS[kind()]}: ${PROP_KIND_HINTS[kind()].toLowerCase()}. Click to change.`}>
        <Dynamic component={SOURCE_ICONS[kind()]} aria-hidden="true" />
      </Menu.Trigger>
      <ScenePortal>
        <Menu.Positioner>
          <Menu.Content class="scene-dropdown-list scene-source-menu">
            <Menu.RadioItemGroup
              value={kind()}
              onValueChange={(details) => {
                choose(details.value as PropKind)
                revealBelow(trigger)
              }}
            >
              <For each={kinds()}>
                {(option) => (
                  <Menu.RadioItem value={option}>
                    <Dynamic component={SOURCE_ICONS[option]} aria-hidden="true" />
                    <span class="scene-source-option">
                      <Menu.ItemText>{PROP_KIND_LABELS[option]}</Menu.ItemText>
                      <span class="scene-source-hint">{PROP_KIND_HINTS[option]}</span>
                    </span>
                  </Menu.RadioItem>
                )}
              </For>
            </Menu.RadioItemGroup>
          </Menu.Content>
        </Menu.Positioner>
      </ScenePortal>
    </Menu.Root>
  )
}

const OVERLAP_OPTIONS: { value: Overlap; label: string }[] = [
  { value: 'Max', label: 'Strongest' },
  { value: 'Add', label: 'Add up' },
]

/** A curve from the list, or from the curves drawn out beside it. One written by hand in the file shows as Custom. */
export function CurveField(props: { label: string; lens: Lens<Curve> }) {
  const chosen = () => {
    const value = props.lens.get()
    return typeof value === 'string' ? value : CUSTOM_CURVE
  }
  return (
    <div class="scene-field-line">
      <Dropdown
        label={props.label}
        options={[...CURVES, ...(chosen() === CUSTOM_CURVE ? [CUSTOM_CURVE] : [])].map((value) => ({ value, label: value }))}
        value={chosen()}
        onChange={(next) => {
          if (next !== CUSTOM_CURVE) props.lens.set(next as Curve)
        }}
      />
      <CurvePicker label={props.label} value={props.lens.get()} onPick={props.lens.set} />
    </div>
  )
}

/** A response to a traversal's visits, from `from` to `to` and back, timed in the traversal's steps */
function VisitField(props: { label: string; lens: Lens<VisitResponse<SceneValue>>; ranges: Ranges }) {
  const traversals = useContext(TraversalsContext)
  const response = () => props.lens.get()
  // one the layer doesn't have anymore stays chosen, so it can be seen and changed
  const options = () => [...new Set([...traversals(), response().traversal])].filter((name) => name !== '').map((name) => ({ value: name, label: name }))
  const stays = () => response().smooth_out === undefined
  const curveField = (key: 'curve_in' | 'curve_out', label: string) => (
    <FieldRow label={label}>
      <CurveField label={`${props.label} ${label.toLowerCase()}`} lens={field(props.lens, key)} />
    </FieldRow>
  )

  return (
    <>
      <FieldRow label="Traversal">
        <Show when={options().length > 0} fallback={<p class="scene-note scene-note-warn">The layer has no traversals to respond to.</p>}>
          <Dropdown label={`${props.label} traversal`} options={options()} value={response().traversal} onChange={field(props.lens, 'traversal').set} />
        </Show>
      </FieldRow>
      <Show when={response().traversal !== '' && !traversals().includes(response().traversal)}>
        <p class="scene-note scene-note-warn">The layer has no traversal by this name, so this stays at From.</p>
      </Show>
      <FieldRow label="From">
        <ValueField label={`${props.label} from`} ranges={props.ranges} lens={field(props.lens, 'from')} />
      </FieldRow>
      <FieldRow label="To">
        <ValueField label={`${props.label} to`} ranges={props.ranges} lens={field(props.lens, 'to')} />
      </FieldRow>
      <FieldRow label="Smooth in">
        <NumberField label={`${props.label} smooth in`} range={STEPS} value={response().smooth_in} onChange={field(props.lens, 'smooth_in').set} />
      </FieldRow>
      <FieldRow label="Hold">
        <NumberField label={`${props.label} hold`} range={STEPS} value={response().hold} onChange={field(props.lens, 'hold').set} />
      </FieldRow>
      <div class="scene-unique">
        <Toggle.Root class="scene-switch" checked={stays()} onCheckedChange={(details) => field(props.lens, 'smooth_out').set(details.checked ? undefined : 1)}>
          <Toggle.Control>
            <Toggle.Thumb />
          </Toggle.Control>
          <Toggle.Label>Stays at To</Toggle.Label>
          <Toggle.HiddenInput aria-label={`${props.label} stays at To`} />
        </Toggle.Root>
      </div>
      <Show when={!stays()}>
        <FieldRow label="Smooth out">
          <NumberField label={`${props.label} smooth out`} range={STEPS} value={response().smooth_out ?? 0} onChange={field(props.lens, 'smooth_out').set} />
        </FieldRow>
      </Show>
      {curveField('curve_in', 'Curve in')}
      <Show when={!stays()}>{curveField('curve_out', 'Curve out')}</Show>
      <p class="scene-note">
        Times are in the traversal's steps, so they keep their shape at any speed. For a traversal that goes by distance, a step is one unit of travel.
      </p>
      <FieldRow label="Overlapping visits">
        <Dropdown label={`${props.label} overlapping visits`} options={OVERLAP_OPTIONS} value={response().overlap} onChange={field(props.lens, 'overlap').set} />
      </FieldRow>
    </>
  )
}

/** A wave's attack and its decay, the range it moves through, and how many times it goes by each cycle. Its phase is a prop under it. */
function ModulatedField(props: { label: string; lens: Lens<Modulated>; ranges: Ranges; fallback: SceneValue }) {
  const envelope = field(props.lens, 'envelope')

  return (
    <>
      <div class="scene-envelope">
        <EnvelopeCurveField label={`${props.label} attack`} name="Attack" value={envelope.get().attack} onChange={field(envelope, 'attack').set} />
        <EnvelopeCurveField optional label={`${props.label} decay`} name="Decay" value={envelope.get().decay} onChange={field(envelope, 'decay').set} />
      </div>
      <SpreadField label={props.label} ranges={props.ranges} lens={field(props.lens, 'spread')} />
      <Show when={!spreadFits(props.lens.get().spread, props.fallback)}>
        <p class="scene-note scene-note-warn">This range is for another type of value, so this reads as zero.</p>
      </Show>
      <FieldRow label="Rate">
        <NumberField label={`${props.label} rate`} range={WAVE_RATE} value={props.lens.get().rate} onChange={field(props.lens, 'rate').set} />
      </FieldRow>
      <p class="scene-note">
        The attack takes the whole wave, or the first half of it with a decay after it. Rate is waves per cycle. The phase under it shifts the wave along, and keyframing it can speed the wave up or
        slow it down.
      </p>
    </>
  )
}

/**
 * One curve of a wave drawn out, which opens the curves to pick from, and a switch that plays it backwards. Only a
 * decay can be left out, which is what `undefined` stands for, and shows as a box to add one.
 */
function EnvelopeCurveField<T extends EnvelopeCurve | undefined>(props: { label: string; name: string; value: T; onChange: (value: T) => void; optional?: boolean }) {
  const reversed = () => props.value?.reversed ?? false
  const height = (phase: number) => (props.value ? curveHeight(props.value.curve, reversed() ? 1 - phase : phase) : 0)

  return (
    <div class="scene-envelope-part">
      <span class="scene-field-label">{props.name}</span>
      <CurvePicker
        class="scene-envelope-curve"
        label={props.label}
        value={props.value?.curve}
        onPick={(curve) => props.onChange({ ...props.value, curve } as T)}
        onRemove={props.optional && props.value ? () => props.onChange(undefined as T) : undefined}
      >
        <Show when={props.value} fallback={<span class="scene-envelope-add">+</span>}>
          <CurveGraph height={height} />
        </Show>
      </CurvePicker>
      <Show when={props.value}>
        <Toggle.Root class="scene-switch" checked={reversed()} onCheckedChange={(details) => props.onChange({ ...props.value!, reversed: details.checked || undefined } as T)}>
          <Toggle.Control>
            <Toggle.Thumb />
          </Toggle.Control>
          <Toggle.Label>Reverse</Toggle.Label>
          <Toggle.HiddenInput aria-label={`Reverse ${props.label}`} />
        </Toggle.Root>
      </Show>
    </div>
  )
}

function ForceUniqueField(props: { label: string; lens: Lens<boolean> }) {
  return (
    <div class="scene-unique">
      <Toggle.Root
        class="scene-switch"
        checked={props.lens.get()}
        onCheckedChange={(details) => props.lens.set(details.checked)}
      >
        <Toggle.Control>
          <Toggle.Thumb />
        </Toggle.Control>
        <Toggle.Label>Force unique</Toggle.Label>
        <Toggle.HiddenInput aria-label={`${props.label} force unique`} />
      </Toggle.Root>
      <InfoTip text="Each new pick has to be different from the one before it, so the value never stays the same across a pick. Turn this off to let a value repeat." />
    </div>
  )
}
