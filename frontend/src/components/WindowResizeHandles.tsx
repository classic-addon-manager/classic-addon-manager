/**
 * Frameless Wails windows have no native resize borders. These thin edge strips
 * sit above the UI for resize; scrollbars are inset (see scroll-area.tsx) so
 * they don't overlap this zone.
 */
export function WindowResizeHandles() {
  return (
    <div className="pointer-events-none fixed inset-0 z-9998" aria-hidden>
      <div className="window-resize-edge window-resize-left" />
      <div className="window-resize-edge window-resize-right" />
      <div className="window-resize-edge window-resize-bottom" />
      <div className="window-resize-edge window-resize-top-left" />
      <div className="window-resize-edge window-resize-top-right" />
      <div className="window-resize-edge window-resize-bottom-left" />
      <div className="window-resize-edge window-resize-bottom-right" />
    </div>
  )
}
