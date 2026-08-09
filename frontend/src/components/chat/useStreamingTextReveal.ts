import { useCallback, useEffect, useRef, useState } from 'react'

const REVEAL_TICK_MS = 16
/** How far behind the real content the reveal is allowed to trail. */
const CATCHUP_WINDOW_MS = 1000

const charsPerTick = (backlog: number) => {
  const ticksAvailable = CATCHUP_WINDOW_MS / REVEAL_TICK_MS
  return Math.max(1, Math.ceil(backlog / ticksAvailable))
}

export const useStreamingTextReveal = (text: string, isStreaming: boolean) => {
  const [cursor, setCursor] = useState(0)
  const [prevText, setPrevText] = useState(text)
  const cursorRef = useRef(0)
  const wasStreamingRef = useRef(isStreaming)
  const textRef = useRef(text)

  textRef.current = text

  if (isStreaming) {
    wasStreamingRef.current = true
  }

  if (prevText !== text) {
    setPrevText(text)
    if (!text.startsWith(prevText)) {
      cursorRef.current = 0
      setCursor(0)
    }
  }

  const completeReveal = useCallback(() => {
    cursorRef.current = textRef.current.length
    setCursor(cursorRef.current)
  }, [])

  useEffect(() => {
    const targetCursor = text.length

    if (!wasStreamingRef.current) {
      cursorRef.current = targetCursor
      setCursor(targetCursor)
      return
    }

    if (cursorRef.current >= targetCursor) {
      return
    }

    /* The rate is fixed for this batch, the next chunk re-runs the effect and
     * re-sizes it against the new backlog. */
    const step = charsPerTick(targetCursor - cursorRef.current)

    const revealNextBatch = () => {
      cursorRef.current = Math.min(targetCursor, cursorRef.current + step)
      setCursor(cursorRef.current)
      return cursorRef.current >= targetCursor
    }

    if (revealNextBatch()) return

    const intervalId = setInterval(() => {
      if (revealNextBatch()) {
        clearInterval(intervalId)
      }
    }, REVEAL_TICK_MS)

    return () => clearInterval(intervalId)
  }, [text])

  const targetCursor = text.length

  if (!wasStreamingRef.current) {
    return { displayedText: text, isRevealing: false, completeReveal }
  }

  const displayedText = text.slice(0, cursor)
  const isRevealing = isStreaming || cursor < targetCursor

  return { displayedText, isRevealing, completeReveal }
}
