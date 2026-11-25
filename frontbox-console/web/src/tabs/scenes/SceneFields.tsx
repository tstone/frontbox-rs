import { ColorPicker, parseColor, type Color as PickerColor } from '@ark-ui/solid/color-picker'
import { Menu } from '@ark-ui/solid/menu'
import { Dynamic } from 'solid-js/web'
import { createListCollection, Select } from '@ark-ui/solid/select'
import { Slider } from '@ark-ui/solid/slider'
import { Switch as Toggle } from '@ark-ui/solid/switch'
import { SegmentGroup } from '@ark-ui/solid/segment-group'
import { createContext, createMemo, createSignal, For, Index, Match, Show, Switch, useContext, type Accessor, type Signal } from 'solid-js'
import ScenePortal, { layerColorClass, type LayerColor } from './ScenePortal'
import { AudioLines, AudioWaveform, ChartSpline, Dices, Eye, EyeOff, Footprints, Orbit, Pin, Pipette, Plus, SlidersHorizontal, Variable, X } from '../../components/icons'
import type { Color } from '../../types/generated/Color'
import { NAMED_COLORS } from '../../types/generated/namedColors'
import type { Prop } from '../../types/generated/Prop'
import type { RenderedShape } from '../../types/generated/RenderedShape'
import type { Layer } from '../../types/generated/Layer'
import type { Curve } from '../../types/generated/Curve'
import type { ExternalEnvelope } from '../../types/generated/ExternalEnvelope'
import type { Modulated } from '../../types/generated/Modulated'
import type { Modulator } from '../../types/generated/Modulator'
import type { Overlap } from '../../types/generated/Overlap'
import type { Spread } from '../../types/generated/Spread'
import type { Value } from '../../types/generated/Value'
import type { VisitResponse } from '../../types/generated/VisitResponse'
import CurvePicker, { CurveGraph } from './CurvePicker'
import { curveHeight, curveText, PLAIN_CURVES } from './curves'
import NumberBox from './NumberBox'
import { field, freshVariableName, fullSpread, HUE, item, newPick, newSweep, newVisit, poolOf, removed, SLEW, soundLabel, unwrapValue, variableReads, spreadFits, startingValue, STEPS, variant, WAVE_RATE, withNewModulator, withSoundNamed, withVariable, withTiming, wrapValue, type Lens, type PropValue,type Range, type Ranges, type LayerSurfaces, type Scene, type SceneValue, type Selection, type Surface, type Vec2 } from './sceneModel'

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
 * Whether a layer's region, or its graph, is drawn over the preview while it's selected. Only for the editing,
 * so it isn't part of the scene and hiding it changes nothing the scene shows.
 */
export type RegionVisibility = { shown: (of: Selection) => boolean; show: (of: Selection, shown: boolean) => void }

export const RegionVisibilityContext = createContext<RegionVisibility>()

/**
 * Where a layer's region stands as the machine last drew it, at the playhead, for fields that need to know
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

/**
 * What a random prop can pick from, or a wave moves through: a range, with a control for each thing about it that
 * varies, or a pool of set values. A color range only varies in hue: how bright or see-through it is is up to the
 * layer's adjusts. A pool's colors are whole colors.
 */
function SpreadField(props: { label: string; lens: Lens<Spread>; ranges: Ranges; like: SceneValue }) {
  const spread = () => props.lens.get()
  const float = () => variant(props.lens, 'Float')
  const vec2 = () => variant(props.lens, 'Vec2')
  const color = () => variant(props.lens, 'Color')
  const mode = () => ('Pool' in spread() ? 'Pool' : 'Range')
  const choose = (next: string | null) => {
    if (!next || next === mode()) return
    props.lens.set(next === 'Pool' ? poolOf(spread(), props.like, props.ranges) : fullSpread(props.like, props.ranges))
  }

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
    <>
      <SegmentGroup.Root class="scene-prop-kind scene-spread-mode" aria-label={`${props.label} picks from`} value={mode()} onValueChange={(details) => choose(details.value)}>
        <SegmentGroup.Indicator />
        <For each={['Range', 'Pool']}>
          {(option) => (
            <SegmentGroup.Item value={option} title={option === 'Range' ? 'Anywhere between a low and a high value' : 'One of a set of values'}>
              <SegmentGroup.ItemText>{option}</SegmentGroup.ItemText>
              <SegmentGroup.ItemControl />
              <SegmentGroup.ItemHiddenInput />
            </SegmentGroup.Item>
          )}
        </For>
      </SegmentGroup.Root>
      <Switch>
        <Match when={'Pool' in spread()}>
          <PoolField label={props.label} lens={variant(props.lens, 'Pool')} ranges={props.ranges} like={props.like} />
        </Match>
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
        </Match>
      </Switch>
      <Show when={!spreadFits(spread(), props.like)}>
        <p class="scene-note scene-note-warn">
          {'Pool' in spread()
            ? (spread() as { Pool: Value[] }).Pool.length === 0
              ? 'The pool is empty, so this reads as zero.'
              : 'Some of these values are for another type of value, and read as zero.'
            : 'This range is for another type of value, so this reads as zero.'}
        </p>
      </Show>
    </>
  )
}

/** The values a pool picks from, each with its own control, in the order a wave steps through them */
function PoolField(props: { label: string; lens: Lens<Value[]>; ranges: Ranges; like: SceneValue }) {
  // a value of another type shows as a new one of the right type, which editing it replaces it with
  const valueLens = (index: number): Lens<SceneValue> => {
    const value = item(props.lens, index)
    return {
      get: () => {
        const current = value.get()
        return spreadFits({ Pool: [current] }, props.like) ? unwrapValue(current as PropValue) : props.like
      },
      set: (next) => value.set(wrapValue(next)),
    }
  }
  const add = () => {
    const values = props.lens.get()
    props.lens.set([...values, values[values.length - 1] ?? wrapValue(props.like)])
  }

  return (
    <div class="scene-pool">
      <Index each={props.lens.get()}>
        {(_, index) => (
          <div class="scene-pool-value">
            <div class="scene-pool-control">
              <ValueField label={`${props.label} ${index + 1}`} ranges={props.ranges} lens={valueLens(index)} />
            </div>
            <button
              type="button"
              class="scene-icon-button"
              aria-label={`Remove ${props.label} ${index + 1}`}
              title="Remove this value"
              disabled={props.lens.get().length <= 1}
              onClick={() => props.lens.set(removed(props.lens.get(), index))}
            >
              <X aria-hidden="true" />
            </button>
          </div>
        )}
      </Index>
      <button type="button" class="scene-add-modifier" onClick={add}>
        <Plus aria-hidden="true" />
        Add value
      </button>
    </div>
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

export type PropKind = 'Fixed' | 'Variable' | 'Keyframes' | 'Random' | 'Modulated' | 'Sweep' | 'Visit' | 'ExternalEnvelope'
const PROP_KINDS: PropKind[] = ['Fixed', 'Variable', 'Keyframes', 'Random', 'Modulated', 'Sweep', 'Visit', 'ExternalEnvelope']

export const PROP_KIND_LABELS: Record<PropKind, string> = {
  Fixed: 'Fixed',
  Variable: 'Variable',
  Keyframes: 'Keyframes',
  Random: 'Random',
  Modulated: 'Modulated',
  Sweep: 'Sweep',
  Visit: 'Visit',
  ExternalEnvelope: 'Envelope',
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
  ExternalEnvelope: AudioLines,
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
  ExternalEnvelope: "Follows the loudness of one of the scene's sounds",
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
 * set off by visits, or following a sound, and the control for that. A sweep's path is under the prop's row in the tree, and selected there to edit it.
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
            like={props.fallback}
            lens={field(variant(props.lens, 'Random'), 'spread')}
          />
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
        <Match when={kind() === 'ExternalEnvelope'}>
          <ExternalEnvelopeField label={props.label} ranges={props.ranges} fallback={props.fallback} lens={variant(props.lens as Lens<Prop<SceneValue>>, 'ExternalEnvelope')} />
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
  // a prop whose variant was just switched away from, such as a solid fill's color when it becomes noise, reads nothing
  // for as long as it's still on show, which is until its row is taken down
  const kind = () => (prop() === undefined ? 'Fixed' : propKind(prop()))
  const value = () => startingValue(prop(), scene.get(), props.fallback)
  // only a position can follow a path, though a scene that has one anywhere else can still be switched off it
  const isPosition = () => Array.isArray(props.fallback) && props.fallback.length === 2
  // visits are only there to respond to in a layer with traversals, and an envelope in a scene with sounds
  const traversals = useContext(TraversalsContext)
  const kinds = () =>
    PROP_KINDS.filter(
      (option) =>
        (option !== 'Sweep' || isPosition() || kind() === 'Sweep') &&
        (option !== 'Visit' || traversals().length > 0 || kind() === 'Visit') &&
        (option !== 'ExternalEnvelope' || scene.get().sounds.length > 0 || kind() === 'ExternalEnvelope'),
    )

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
      props.lens.set({ Random: { picks: [newPick(0)], spread: fullSpread(current, props.ranges), seed } })
    }
    if (next === 'Modulated') {
      // moves by the scene's first modulator, or a new one if it has none yet
      let timed = withTiming(scene.get())
      let modulator = Object.keys(timed.modulators ?? {})[0]
      if (modulator === undefined) [timed, modulator] = withNewModulator(timed)
      scene.set(timed)
      props.lens.set({ Modulated: { modulator, spread: fullSpread(current, props.ranges), phase: { Fixed: 0 } } })
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
    if (next === 'ExternalEnvelope') {
      // follows the scene's first sound, which is given a name to be followed by if it has none
      const [named, layer] = withSoundNamed(withTiming(scene.get()), 0)
      scene.set(named)
      props.lens.set({ ExternalEnvelope: { source: { Sound: { layer } }, spread: fullSpread(current, props.ranges), release: 0.1 } })
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

export { NumberBox }

const OVERLAP_OPTIONS: { value: Overlap; label: string }[] = [
  { value: 'Max', label: 'Strongest' },
  { value: 'Add', label: 'Add up' },
]

/** Stands for a curve that isn't in the dropdown's list, such as one made from others, so the dropdown can show it */
const OTHER_CURVE = 'other'

/**
 * A curve from the list, or from the curves drawn out beside it. One that isn't in the list, such as one made from
 * others, shows written out, and is changed from the curves drawn out.
 */
export function CurveField(props: { label: string; lens: Lens<Curve> }) {
  const chosen = () => {
    const value = props.lens.get()
    return typeof value === 'string' ? value : OTHER_CURVE
  }
  const options = () => [...PLAIN_CURVES.map((value) => ({ value: value as string, label: value as string })), ...(chosen() === OTHER_CURVE ? [{ value: OTHER_CURVE, label: curveText(props.lens.get()) }] : [])]
  return (
    <div class="scene-field-line">
      <Dropdown
        label={props.label}
        options={options()}
        value={chosen()}
        onChange={(next) => {
          if (next !== OTHER_CURVE) props.lens.set(next as Curve)
        }}
      />
      <CurvePicker label={props.label} value={props.lens.get()} onChange={props.lens.set} />
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

/** Opens the sidebar's Modulators tab at the modulator named */
export const ShowModulatorContext = createContext<(name: string) => void>(() => {})

/**
 * Which of the scene's modulators a prop moves by, with a button to make a new one for it, and the range it moves
 * through. The modulator's shape and rate are in the Modulators tab, and the prop's phase is a prop under it.
 */
function ModulatedField(props: { label: string; lens: Lens<Modulated>; ranges: Ranges; fallback: SceneValue }) {
  const scene = useScene()
  const showModulator = useContext(ShowModulatorContext)
  const chosen = () => props.lens.get().modulator
  const names = () => Object.keys(scene.get().modulators ?? {})
  const missing = () => !names().includes(chosen())
  // one the scene doesn't have anymore stays chosen, so it can be seen and changed
  const options = () => [...names(), ...(missing() ? [chosen()] : [])].map((name) => ({ value: name, label: name }))
  const addModulator = () => {
    const [next, name] = withNewModulator(scene.get())
    scene.set(next)
    field(props.lens, 'modulator').set(name)
    showModulator(name)
  }

  return (
    <>
      <div class="scene-modulator-pick">
        <Dropdown label={`${props.label} modulator`} options={options()} value={chosen()} onChange={field(props.lens, 'modulator').set} />
        <button type="button" class="scene-icon-button" aria-label={`Edit modulator ${chosen()}`} title="Edit this modulator" disabled={missing()} onClick={() => showModulator(chosen())}>
          <SlidersHorizontal aria-hidden="true" />
        </button>
        <button type="button" class="scene-icon-button" aria-label={`New modulator for ${props.label}`} title="New modulator" onClick={addModulator}>
          <Plus aria-hidden="true" />
        </button>
      </div>
      <Show when={missing()}>
        <p class="scene-note scene-note-warn">The scene has no modulator by that name, so this reads as zero.</p>
      </Show>
      <SpreadField label={props.label} ranges={props.ranges} like={props.fallback} lens={field(props.lens, 'spread')} />
      <p class="scene-note">The modulator sets the wave's shape and rate, and can be shared. The phase under it shifts this prop's wave along, and keyframing it can speed the wave up or slow it down.</p>
    </>
  )
}

/**
 * Which of the scene's sounds an envelope follows, the range it moves through, and how fast it can rise and fall.
 * Following a sound that has no name gives it one, as an envelope follows a sound by its name.
 */
function ExternalEnvelopeField(props: { label: string; lens: Lens<ExternalEnvelope>; ranges: Ranges; fallback: SceneValue }) {
  const scene = useScene()
  const chosen = () => props.lens.get().source.Sound.layer
  const sounds = () => scene.get().sounds
  const index = () => sounds().findIndex((sound) => sound.name === chosen())
  // one the scene doesn't have anymore stays chosen, so it can be seen and changed
  const options = () => [
    ...sounds().map((sound, at) => ({ value: String(at), label: soundLabel(sound, at) })),
    ...(index() < 0 ? [{ value: 'missing', label: chosen() }] : []),
  ]
  const follow = (value: string) => {
    if (value === 'missing') return
    const [named, layer] = withSoundNamed(scene.get(), Number(value))
    scene.set(named)
    field(props.lens, 'source').set({ Sound: { ...props.lens.get().source.Sound, layer } })
  }

  return (
    <>
      <Dropdown label={`${props.label} sound`} options={options()} value={index() < 0 ? 'missing' : String(index())} onChange={follow} />
      <Show when={index() < 0}>
        <p class="scene-note scene-note-warn">The scene has no sound by that name, so this stays at the low end of its range.</p>
      </Show>
      <SpreadField label={props.label} ranges={props.ranges} like={props.fallback} lens={field(props.lens, 'spread')} />
      <FieldRow label="Attack">
        <NumberField label={`${props.label} attack`} range={SLEW} value={props.lens.get().attack ?? 0} onChange={(attack) => field(props.lens, 'attack').set(attack || undefined)} />
      </FieldRow>
      <FieldRow label="Release">
        <NumberField label={`${props.label} release`} range={SLEW} value={props.lens.get().release ?? 0} onChange={(release) => field(props.lens, 'release').set(release || undefined)} />
      </FieldRow>
      <p class="scene-note">Moves from the low end of its range when the sound is silent to the high end at its loudest, wherever the sound is on the timeline. Attack and release are the fewest seconds it takes to rise all the way and fall all the way, smoothing out flicker. How loud the sound is set to play doesn't change it.</p>
    </>
  )
}

/** One of the scene's modulators: its attack and decay, and how many times its wave goes by each cycle */
export function ModulatorFields(props: { label: string; lens: Lens<Modulator> }) {
  const waveform = field(props.lens, 'waveform')
  return (
    <>
      <div class="scene-waveform">
        <WaveformCurveField label={`${props.label} attack`} name="Attack" value={waveform.get().attack} onChange={field(waveform, 'attack').set} />
        <WaveformCurveField optional label={`${props.label} decay`} name="Decay" value={waveform.get().decay} onChange={field(waveform, 'decay').set} />
      </div>
      <FieldRow label="Rate">
        <NumberField label={`${props.label} rate`} range={WAVE_RATE} value={props.lens.get().rate} onChange={(rate) => field(props.lens, 'rate').set(Math.max(WAVE_RATE.min, rate))} />
      </FieldRow>
    </>
  )
}

/**
 * One curve of a wave drawn out, which opens the curves to pick from. Only a decay can be left out, which is what
 * `undefined` stands for: it shows as a box to add one, and once added has an X to remove it on hover.
 */
function WaveformCurveField<T extends Curve | undefined>(props: { label: string; name: string; value: T; onChange: (value: T) => void; optional?: boolean }) {
  return (
    <div class="scene-waveform-part">
      <span class="scene-field-label">{props.name}</span>
      <div class="scene-curve-removable">
        <CurvePicker class="scene-waveform-curve" label={props.label} value={props.value} onChange={(curve) => props.onChange(curve as T)}>
          <Show when={props.value} fallback={<span class="scene-waveform-add">+</span>}>
            {(curve) => <CurveGraph height={(phase) => curveHeight(curve(), phase)} />}
          </Show>
        </CurvePicker>
        <Show when={props.optional && props.value}>
          <button type="button" class="scene-curve-remove" aria-label={`Remove ${props.label}`} title="Remove" onClick={() => props.onChange(undefined as T)}>
            <X aria-hidden="true" />
          </button>
        </Show>
      </div>
    </div>
  )
}
