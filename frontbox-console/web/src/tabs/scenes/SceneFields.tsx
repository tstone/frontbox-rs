import { ColorPicker, parseColor, type Color as PickerColor } from '@ark-ui/solid/color-picker'
import { SegmentGroup } from '@ark-ui/solid/segment-group'
import { createListCollection, Select } from '@ark-ui/solid/select'
import { Slider } from '@ark-ui/solid/slider'
import { createContext, createSignal, For, Match, Show, Switch, useContext, type Accessor } from 'solid-js'
import { Portal } from 'solid-js/web'
import type { Color } from '../../types/generated/Color'
import type { Prop } from '../../types/generated/Prop'
import type { Scene } from '../../types/generated/Scene'
import { freshParamName, paramFits, startingValue, withParam, type Lens, type Range, type Ranges, type SceneValue, type Surface, type Vec2 } from './sceneModel'

/** The scene being edited, for fields whose edits reach past their own place in it */
export const SceneContext = createContext<Lens<Scene>>()

/** The size the scene is being edited for */
export const SurfaceContext = createContext<Accessor<Surface>>()

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

/** A slider and a box for typing a value. A value from the box or the file past the range widens the slider. */
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
          <Slider.Thumb index={0}>
            <Slider.HiddenInput />
          </Slider.Thumb>
        </Slider.Control>
      </Slider.Root>
      <input
        type="number"
        aria-label={props.label}
        step={props.range.step}
        value={props.value}
        onInput={(e) => {
          if (Number.isFinite(e.currentTarget.valueAsNumber)) props.onChange(e.currentTarget.valueAsNumber)
        }}
      />
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

type PropKind = 'Fixed' | 'Param' | 'Track'
const PROP_KINDS: PropKind[] = ['Fixed', 'Param', 'Track']

/** A prop: whether it's fixed, read from a parameter, or keyframed, and the control for that */
export function PropField<T extends SceneValue>(props: {
  label: string
  /** What to call a parameter made from this prop */
  paramName: string
  lens: Lens<Prop<T>>
  /** For a prop with nothing to take a value from */
  fallback: T
  ranges: Ranges
}) {
  const scene = useScene()
  const prop = () => props.lens.get()
  const kind = (): PropKind => ('Fixed' in prop() ? 'Fixed' : 'Param' in prop() ? 'Param' : 'Track')
  const value = () => startingValue(prop(), scene.get(), props.fallback)
  const paramName = () => (prop() as { Param: string }).Param

  const choose = (next: PropKind) => {
    if (next === kind()) return
    const current = value()
    if (next === 'Fixed') props.lens.set({ Fixed: current })
    if (next === 'Track') props.lens.set({ Track: { keys: [{ at: 0, value: current, curve: 'Linear' }], cycle: 'Once' } })
    if (next === 'Param') {
      const name = freshParamName(scene.get(), props.paramName)
      scene.set(withParam(scene.get(), name, current))
      props.lens.set({ Param: name })
    }
  }

  const rename = (name: string) => {
    if (name === '' || name === paramName()) return
    if (scene.get().params?.[name] === undefined) scene.set(withParam(scene.get(), name, value()))
    props.lens.set({ Param: name })
  }

  return (
    <div class="scene-field scene-prop">
      <div class="scene-prop-header">
        <span class="scene-field-label">{props.label}</span>
        <SegmentGroup.Root
          class="scene-prop-kind"
          aria-label={`${props.label} source`}
          value={kind()}
          onValueChange={(details) => choose(details.value as PropKind)}
        >
          <SegmentGroup.Indicator />
          <For each={PROP_KINDS}>
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
          <p class="scene-note">Keyframes can't be edited here yet.</p>
        </Match>
      </Switch>
    </div>
  )
}
