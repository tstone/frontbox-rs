import { createEffect, createMemo, splitProps, type JSX } from 'solid-js'

/**
 * A box for typing a number, which is given each number as it's typed. Writing to a box moves its caret and drops text
 * like `0.` that's on its way to a number, so it's only written when the value becomes something it doesn't already say,
 * and tidied up when it's left.
 */
export default function NumberBox(props: Omit<JSX.InputHTMLAttributes<HTMLInputElement>, 'value'> & { value: number; onNumber: (value: number) => void }) {
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
