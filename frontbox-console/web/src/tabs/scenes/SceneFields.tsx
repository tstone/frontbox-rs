import { ColorPicker, parseColor, type Color as PickerColor } from '@ark-ui/solid/color-picker'
import { SegmentGroup } from '@ark-ui/solid/segment-group'
import { createListCollection, Select } from '@ark-ui/solid/select'
import { Slider } from '@ark-ui/solid/slider'
import { Switch as Toggle } from '@ark-ui/solid/switch'
import { createContext, createEffect, createMemo, createSignal, For, Match, Show, splitProps, Switch, useContext, type Accessor, type JSX, type Signal } from 'solid-js'
import { Portal } from 'solid-js/web'
import InfoTip from '../../components/InfoTip'
import { ShapeEditor } from './ConfigPanel'
import type { Color } from '../../types/generated/Color'
import type { Prop } from '../../types/generated/Prop'
import type { Spread } from '../../types/generated/Spread'
import { field, freshParamName, fullSpread, HUE, newPick, newSweep, paramFits, spreadFits, startingValue, UNIT, variant, withParam, withTiming, type Lens, type Range, type Ranges, type Scene, type SceneValue, type Surface, type Vec2 } from './sceneModel'

/** The scene being edited, for fields whose edits reach past their own place in it */
export const SceneContext = createContext<Lens<Scene>>()

/** The size the scene is being edited for */
export const SurfaceContext = createContext<Accessor<Surface>>()

/** The prop waiting to be shown to whoever asked, until its field comes up and takes it */
export const RevealContext = createContext<Signal<Prop<SceneValue> | null>>()

function useScene() {
  return useContext(SceneContext)!
}

/** A value's control and the label above it */
export function FieldRow(props: { label: string; children: any }) {
  return (
    <div class="scene-field">
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
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <path d="M1.5 8S4 3.5 8 3.5 14.5 8 14.5 8 12 12.5 8 12.5 1.5 8 1.5 8z" />
        <Show when={props.hidden} fallback={<circle cx="8" cy="8" r="1.75" />}>
          <path d="M3 13 13 3" />
        </Show>
      </svg>
    </button>
  )
}

/** Picks one of an enum's values */
export function Dropdown<T extends string>(props: {
  label: string
  options: { value: T; label: string }[]
  value: T
  onChange: (value: T) => void
}) {
  const collection = createListCollection({ items: props.options })
  return (
    <Select.Root
      class="scene-dropdown"
      collection={collection}
      value={[props.value]}
      onValueChange={(details) => props.onChange(details.value[0] as T)}
      positioning={{ sameWidth: true }}
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
      <Portal>
        <Select.Positioner>
          <Select.Content class="scene-dropdown-list">
            <For each={collection.items}>
              {(option) => (
                <Select.Item item={option}>
                  <Select.ItemText>{option.label}</Select.ItemText>
                  <Select.ItemIndicator>
                    <svg viewBox="0 0 16 16" aria-hidden="true">
                      <path d="M3 8.5l3.5 3.5L13 5" />
                    </svg>
                  </Select.ItemIndicator>
                </Select.Item>
              )}
            </For>
          </Select.Content>
        </Select.Positioner>
      </Portal>
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

/** What a random prop can pick from, with a control for each thing about it that varies */
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

function ColorField(props: { label: string; lens: Lens<Color> }) {
  // what the picker last chose. The scene only keeps red, green, and blue, which lose the hue of a black or a gray, so
  // the picker keeps its own color for as long as the scene still agrees with it.
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
      class="scene-color"
      format="rgba"
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
        <ColorPicker.ChannelInput channel="hex" aria-label={`${props.label} as hex`} />
      </ColorPicker.Control>
      <Portal>
        <ColorPicker.Positioner>
          <ColorPicker.Content class="scene-color-picker">
            <ColorPicker.Area>
              <ColorPicker.AreaBackground />
              <ColorPicker.AreaThumb />
            </ColorPicker.Area>
            {channelSlider('hue')}
            {channelSlider('alpha')}
          </ColorPicker.Content>
        </ColorPicker.Positioner>
      </Portal>
      <ColorPicker.HiddenInput />
    </ColorPicker.Root>
  )
}

function toScene(color: PickerColor): Color {
  const rgba = color.toFormat('rgba')
  const channel = (name: 'red' | 'green' | 'blue') => Math.round(rgba.getChannelValue(name))
  return [channel('red'), channel('green'), channel('blue'), Math.round(rgba.getChannelValue('alpha') * 255)]
}

/** The control for a value of any type. Both a fixed prop and a parameter's default use this. */
export function ValueField(props: { label: string; lens: Lens<SceneValue>; ranges: Ranges }) {
  const kind = () => {
    const value = props.lens.get()
    if (typeof value === 'number') return 'number'
    return value.length === 2 ? 'vec2' : 'color'
  }
  return (
    <Switch>
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

type PropKind = 'Fixed' | 'Param' | 'Track' | 'Random' | 'Sweep'
const PROP_KINDS: PropKind[] = ['Fixed', 'Param', 'Track', 'Random', 'Sweep']

/** A prop: whether it's fixed, read from a parameter, keyframed, picked at random, or swept along a path, and the control for that */
export function PropField<T extends SceneValue>(props: {
  label: string
  /** What to call a parameter made from this prop */
  paramName: string
  lens: Lens<Prop<T>>
  /** For a prop with nothing to take a value from */
  fallback: T
  ranges: Ranges
  /** For a pair of numbers that isn't a position, such as a scale */
  notPosition?: boolean
}) {
  const scene = useScene()
  const [revealing, setRevealing] = useContext(RevealContext)!
  const [pinged, setPinged] = createSignal(false)
  let element!: HTMLDivElement
  const prop = () => props.lens.get()
  const kind = () => Object.keys(prop())[0] as PropKind
  const value = () => startingValue(prop(), scene.get(), props.fallback)
  const paramName = () => (prop() as { Param: string }).Param
  // only a position can follow a path, though a scene that has one anywhere else can still be switched off it
  const isPosition = () => Array.isArray(props.fallback) && props.fallback.length === 2 && !props.notPosition
  const kinds = () => PROP_KINDS.filter((option) => option !== 'Sweep' || isPosition() || kind() === 'Sweep')

  const choose = (next: PropKind) => {
    if (next === kind()) return
    const current = value()
    if (next === 'Fixed') props.lens.set({ Fixed: current })
    if (next === 'Track') {
      scene.set(withTiming(scene.get()))
      props.lens.set({ Track: { keys: [{ at: 0, value: current, curve: 'Linear' }], cycle: 'Once' } })
    }
    if (next === 'Random') {
      scene.set(withTiming(scene.get()))
      const seed = Math.floor(Math.random() * 2 ** 32)
      props.lens.set({ Random: { picks: [newPick(0)], spread: fullSpread(current, props.ranges), seed, force_unique: true } })
    }
    if (next === 'Sweep') {
      const timed = withTiming(scene.get())
      scene.set(timed)
      props.lens.set({ Sweep: newSweep(current as Vec2, props.ranges, timed.timing!.duration) })
    }
    if (next === 'Param') {
      const name = freshParamName(scene.get(), props.paramName)
      scene.set(withParam(scene.get(), name, current))
      props.lens.set({ Param: name })
    }
  }

  // a field can come up for this, in a layer that's only now opening
  createEffect(() => {
    if (revealing() === null || revealing() !== prop()) return
    setRevealing(null)
    setPinged(true)
    requestAnimationFrame(() => element.scrollIntoView({ block: 'center', behavior: 'smooth' }))
  })

  const rename = (name: string) => {
    if (name === '' || name === paramName()) return
    if (scene.get().params?.[name] === undefined) scene.set(withParam(scene.get(), name, value()))
    props.lens.set({ Param: name })
  }

  return (
    <div
      class="scene-field scene-prop"
      classList={{ 'scene-ping': pinged() }}
      ref={element}
      onAnimationEnd={(e) => {
        if (e.target === e.currentTarget) setPinged(false)
      }}
    >
      <div class="scene-prop-header">
        <span class="scene-field-label">{props.label}</span>
        <SegmentGroup.Root
          class="scene-prop-kind"
          aria-label={`${props.label} source`}
          value={kind()}
          onValueChange={(details) => choose(details.value as PropKind)}
        >
          <SegmentGroup.Indicator />
          <For each={kinds()}>
            {(option) => (
              <SegmentGroup.Item value={option}>
                <SegmentGroup.ItemText>{option}</SegmentGroup.ItemText>
                <SegmentGroup.ItemControl />
                <SegmentGroup.ItemHiddenInput />
              </SegmentGroup.Item>
            )}
          </For>
        </SegmentGroup.Root>
      </div>
      <Switch>
        <Match when={kind() === 'Fixed'}>
          <ValueField
            label={props.label}
            ranges={props.ranges}
            lens={{ get: () => value(), set: (next) => props.lens.set({ Fixed: next as T }) }}
          />
        </Match>
        <Match when={kind() === 'Param'}>
          <input
            class="scene-param-name"
            aria-label={`${props.label} parameter name`}
            spellcheck={false}
            value={paramName()}
            onChange={(e) => {
              rename(e.currentTarget.value.trim())
              e.currentTarget.value = paramName()
            }}
          />
          <Show when={!paramFits(scene.get().params?.[paramName()], props.fallback)}>
            <p class="scene-note scene-note-warn">No parameter of this type has that name, so this reads as zero.</p>
          </Show>
        </Match>
        <Match when={kind() === 'Track'}>
          <p class="scene-note">Keyframes are on the timeline below.</p>
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
          <p class="scene-note">When to pick a new value is on the timeline below.</p>
        </Match>
        <Match when={kind() === 'Sweep'}>
          <ShapeEditor path lens={field(variant(props.lens, 'Sweep'), 'path')} />
          <p class="scene-note">When it goes around is on the timeline below.</p>
        </Match>
      </Switch>
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
