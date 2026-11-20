/** Whether a key pressed on `target` is typing, which a shortcut on that key mustn't take */
export const typingIn = (target: EventTarget | null) =>
  target instanceof HTMLElement &&
  (target.isContentEditable ||
    target instanceof HTMLTextAreaElement ||
    target instanceof HTMLSelectElement ||
    (target instanceof HTMLInputElement &&
      !['button', 'checkbox', 'radio', 'range', 'color', 'file', 'reset', 'submit'].includes(target.type)))
