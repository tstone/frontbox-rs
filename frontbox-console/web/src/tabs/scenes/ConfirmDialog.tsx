import { Dialog } from '@ark-ui/solid/dialog'
import { Portal } from 'solid-js/web'

/** Checks before something that can't be taken back */
export default function ConfirmDialog(props: {
  open: boolean
  title: string
  description: string
  cancel: string
  action: string
  onCancel: () => void
  onConfirm: () => void
}) {
  return (
    <Dialog.Root lazyMount unmountOnExit role="alertdialog" open={props.open} onOpenChange={(details) => !details.open && props.onCancel()}>
      <Portal>
        <Dialog.Backdrop class="scene-dialog-backdrop" />
        <Dialog.Positioner class="scene-dialog-positioner">
          <Dialog.Content class="scene-dialog scene-dialog-small">
            <header>
              <Dialog.Title>{props.title}</Dialog.Title>
            </header>
            <Dialog.Description class="scene-note">{props.description}</Dialog.Description>
            <footer>
              <Dialog.CloseTrigger class="scene-button">{props.cancel}</Dialog.CloseTrigger>
              <button type="button" class="scene-button" onClick={props.onConfirm}>
                {props.action}
              </button>
            </footer>
          </Dialog.Content>
        </Dialog.Positioner>
      </Portal>
    </Dialog.Root>
  )
}
