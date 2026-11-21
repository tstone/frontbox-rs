/**
 * Click handlers for a tree branch's row, for a tree with `expandOnClick={false}`: a click on a row that isn't selected
 * only selects it, and a click on the selected row opens or closes it. The arrow always opens or closes it. Clicks in a
 * field or button in the row are left to it.
 *
 * Uses `onMouseDown` and `onClick`, so it can be spread beside handlers on `onPointerDown`.
 */
export function selectThenToggle(isSelected: () => boolean, toggle: () => void) {
  // whether it was selected before this click, which has selected it by the time `onClick` runs
  let wasSelected = false
  return {
    onMouseDown: () => {
      wasSelected = isSelected()
    },
    onClick: (event: MouseEvent) => {
      const target = event.target as Element
      if (target.closest('[data-part="branch-indicator"]')) toggle()
      else if (wasSelected && !target.closest('input, textarea, select, button, [contenteditable]')) toggle()
    },
  }
}
