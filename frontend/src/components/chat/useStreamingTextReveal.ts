import { useCallback, useEffect, useState } from 'react'

const REVEAL_TICK_MS = 16
/** How far behind the real content the reveal is allowed to trail. */
const CATCHUP_WINDOW_MS = 1000

const charsPerTick = (backlog: number) => {
  const ticksAvailable = CATCHUP_WINDOW_MS / REVEAL_TICK_MS
  return Math.max(1, Math.ceil(backlog / ticksAvailable))
}

export const useStreamingTextReveal = (text: string, isStreaming: boolean) => {
  const [reveal, setReveal] = useState(() => ({
    text,
    cursor: isStreaming ? 0 : text.length,
    hasStreamed: isStreaming,
    step: charsPerTick(text.length),
  }))

  if (reveal.text !== text || (isStreaming && !reveal.hasStreamed)) {
    const hasStreamed = reveal.hasStreamed || isStreaming
    const cursor = !hasStreamed ? text.length : text.startsWith(reveal.text) ? reveal.cursor : 0

    setReveal({
      text,
      cursor,
      hasStreamed,
      step: charsPerTick(text.length - cursor),
    })
  }

  const completeReveal = useCallback(() => {
    setReveal(current =>
      current.cursor === current.text.length ? current : { ...current, cursor: current.text.length }
    )
  }, [])

  const { cursor, hasStreamed, step } = reveal
  const isCatchingUp = hasStreamed && cursor < text.length

  useEffect(() => {
    if (!isCatchingUp) return

    // Keep this batch's rate fixed until a new chunk changes the backlog.
    const intervalId = setInterval(() => {
      setReveal(current => {
        if (current.text !== text || current.cursor >= text.length) return current
        return { ...current, cursor: Math.min(text.length, current.cursor + step) }
      })
    }, REVEAL_TICK_MS)

    return () => clearInterval(intervalId)
  }, [text, step, isCatchingUp])

  const displayedText = hasStreamed ? text.slice(0, cursor) : text
  const isRevealing = hasStreamed && (isStreaming || isCatchingUp)

  return { displayedText, isRevealing, completeReveal }
}
