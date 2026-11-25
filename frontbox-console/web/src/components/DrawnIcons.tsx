import { splitProps, type JSX } from 'solid-js'
import type { LucideProps } from 'lucide-solid'

/**
 * Icons Lucide has nothing that reads well for at a toolbar's size, drawn as Lucide draws its own so they sit among
 * them: on a 24 unit square, in lines 2 wide with round ends.
 */
function drawn(shapes: () => JSX.Element) {
  return (props: LucideProps) => {
    const [own, rest] = splitProps(props, ['size', 'color', 'stroke-width', 'class'])
    return (
      <svg
        xmlns="http://www.w3.org/2000/svg"
        width={own.size ?? 24}
        height={own.size ?? 24}
        viewBox="0 0 24 24"
        fill="none"
        stroke={own.color ?? 'currentColor'}
        stroke-width={own['stroke-width'] ?? 2}
        stroke-linecap="round"
        stroke-linejoin="round"
        class={`lucide ${own.class ?? ''}`}
        {...rest}
      >
        {shapes()}
      </svg>
    )
  }
}

/** A circle overlapping a square, where Lucide's `shapes` fits three shapes too small to make out */
export const CircleSquare = drawn(() => (
  <>
    <circle cx="8.5" cy="8.5" r="6.5" />
    <rect x="10" y="10" width="12" height="12" rx="1.5" />
  </>
))

/** A smooth line from one corner to the other, where Lucide's `spline` has a dot at each end that reads as a phone */
export const Curve = drawn(() => <path d="M3 21C3 12 21 12 21 3" />)

/** A line winding out from the middle, where Lucide has no spiral */
export const Spiral = drawn(() => (
  <polyline points="11.8 11.1 12.0 10.9 12.3 10.7 12.7 10.7 13.2 10.8 13.6 11.1 14.0 11.5 14.2 12.0 14.3 12.6 14.2 13.3 14.0 14.0 13.5 14.6 12.8 15.0 12.0 15.3 11.1 15.4 10.1 15.2 9.3 14.7 8.5 14.0 7.9 13.1 7.6 12.0 7.5 10.8 7.8 9.6 8.5 8.5 9.4 7.5 10.6 6.8 12.0 6.4 13.5 6.5 15.0 6.9 16.3 7.7 17.5 8.9 18.3 10.3 18.7 12.0 18.6 13.8 18.1 15.5 17.1 17.1 15.7 18.4 14.0 19.3 12.0 19.8 9.9 19.7 7.9 19.1 6.1 17.9 4.6 16.3 3.6 14.3 3.1 12.0 3.2 9.7 4.0 7.4 5.3 5.3 7.2 3.7 9.5 2.5 12.0 2.0" />
))

/** A line crossing up and down three times, easing off at each side, as a smooth wave does */
export const Wave = drawn(() => <path d="M2 17C4.4 17 6.3 7 8.7 7S13 17 15.3 17 19.6 7 22 7" />)

/** A line crossing up and down four times, turning sharply at each side */
export const Zigzag = drawn(() => <polyline points="2 17 7 7 12 17 17 7 22 17" />)

/** A line snaking back and forth in square turns */
export const Serpentine = drawn(() => <path d="M3 20V4h6v16h6V4h6v16" />)
