// What the popups (menus and pickers) share: closing when the user goes elsewhere, and the keyboard
// navigation of a menu.

/**
 * While a popup is open: closes it on a press outside `inside()` (usually the panel and the button
 * that opens it), on a window resize and, with `scroll`, on a scroll outside it (except right after
 * opening, when the page may still be bringing the button into view). Returns the cleanup, so an
 * `$effect` can return it: `$effect(() => { if (open) return dismissOnOutside(...) })`.
 */
export function dismissOnOutside(
  inside: () => readonly (Node | null | undefined)[],
  close: () => void,
  options: { scroll?: boolean } = {},
): () => void {
  const openedAt = performance.now()
  const isInside = (target: EventTarget | null) =>
    target instanceof Node && inside().some((element) => element?.contains(target))
  const onPointer = (event: PointerEvent) => {
    if (!isInside(event.target)) close()
  }
  const onScroll = (event: Event) => {
    if (performance.now() - openedAt >= 250 && !isInside(event.target)) close()
  }
  const onResize = () => close()
  document.addEventListener('pointerdown', onPointer, true)
  if (options.scroll) document.addEventListener('scroll', onScroll, true)
  window.addEventListener('resize', onResize)
  return () => {
    document.removeEventListener('pointerdown', onPointer, true)
    document.removeEventListener('scroll', onScroll, true)
    window.removeEventListener('resize', onResize)
  }
}

/**
 * Keyboard of an open menu: the arrows, Home and End move between its enabled items
 * (`[role^="menuitem"]`, wrapping around); Escape closes it and gives the focus back to its button;
 * Tab closes it.
 */
export function menuKeydown(event: KeyboardEvent, menu: HTMLElement | undefined, close: (refocus: boolean) => void) {
  const items = [...(menu?.querySelectorAll<HTMLElement>('[role^="menuitem"]:not(:disabled)') ?? [])]
  const index = items.indexOf(document.activeElement as HTMLElement)
  const focus = (i: number) => items[i]?.focus({ preventScroll: true })
  if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
    event.preventDefault()
    const step = event.key === 'ArrowDown' ? 1 : -1
    focus((index + step + items.length) % items.length)
  } else if (event.key === 'Home' || event.key === 'End') {
    event.preventDefault()
    focus(event.key === 'Home' ? 0 : items.length - 1)
  } else if (event.key === 'Escape') {
    event.preventDefault()
    close(true)
  } else if (event.key === 'Tab') {
    close(false)
  }
}
