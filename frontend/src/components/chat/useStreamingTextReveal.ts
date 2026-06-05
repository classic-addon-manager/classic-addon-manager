import { useEffect, useRef, useState } from 'react'

const CHAR_REVEAL_MS = 10

export const useStreamingTextReveal = (text: string, isStreaming: boolean) => {
  const [cursor, setCursor] = useState(0)
  const [prevText, setPrevText] = useState(text)
  const cursorRef = useRef(0)
  const wasStreamingRef = useRef(isStreaming)

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

    const revealNextChar = () => {
      cursorRef.current += 1
      setCursor(cursorRef.current)
    }

    revealNextChar()
    if (cursorRef.current >= targetCursor) return

    const intervalId = setInterval(() => {
      revealNextChar()
      if (cursorRef.current >= targetCursor) {
        clearInterval(intervalId)
      }
    }, CHAR_REVEAL_MS)

    return () => clearInterval(intervalId)
  }, [text])

  const targetCursor = text.length

  if (!wasStreamingRef.current) {
    return { displayedText: text, isRevealing: false }
  }

  const displayedText = text.slice(0, cursor)
  const isRevealing = isStreaming || cursor < targetCursor

  return { displayedText, isRevealing }
}
