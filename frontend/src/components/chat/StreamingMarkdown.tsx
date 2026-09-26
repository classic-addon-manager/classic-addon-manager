import { useLayoutEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'

const RENDER_INTERVAL_MS = 100
const FADE_DURATION_MS = 200
const BLOCK_TAGS = [
  'P',
  'DIV',
  'UL',
  'OL',
  'LI',
  'BLOCKQUOTE',
  'PRE',
  'H1',
  'H2',
  'H3',
  'H4',
  'H5',
  'H6',
  'TABLE',
  'THEAD',
  'TBODY',
  'TR',
  'TH',
  'TD',
]

interface StreamingMarkdownProps {
  content: string
  messageId: string
  animateLatestWord: boolean
  parseMarkdown: (content: string) => string
}

interface ParsedContent {
  content: string
  html: string
  messageId: string
  parseMarkdown: StreamingMarkdownProps['parseMarkdown']
}

const findTailContainer = (html: HTMLElement): Element => {
  let container: Element = html
  let lastChild = container.lastChild

  while (lastChild) {
    if (lastChild.nodeType === Node.TEXT_NODE) {
      const structural = ['DIV', 'UL', 'OL', 'LI', 'PRE', 'BLOCKQUOTE'].includes(container.tagName)
      if (lastChild.textContent?.trim() || !structural) break
      lastChild = lastChild.previousSibling
      continue
    }

    if (lastChild.nodeType === Node.ELEMENT_NODE) {
      const element = lastChild as Element
      const isBlock = BLOCK_TAGS.includes(element.tagName)
      const isCodeBlock = element.tagName === 'CODE' && container.tagName === 'PRE'
      if (!isBlock && !isCodeBlock) break
      container = element
      lastChild = container.lastChild
      continue
    }

    lastChild = lastChild.previousSibling
  }

  return container
}

export const StreamingMarkdown = ({
  content,
  messageId,
  animateLatestWord,
  parseMarkdown,
}: StreamingMarkdownProps) => {
  const [parsed, setParsed] = useState<ParsedContent>(() => ({
    content: animateLatestWord ? '' : content,
    html: animateLatestWord ? '' : parseMarkdown(content),
    messageId,
    parseMarkdown,
  }))
  const latestRef = useRef({ content, messageId, animateLatestWord, parseMarkdown })
  const snapshotsRef = useRef<{ content: string; at: number }[]>([])
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const lastRenderAtRef = useRef(0)
  const parsedRef = useRef(parsed)
  const previousContentRef = useRef(content)
  const htmlRef = useRef<HTMLDivElement>(null)
  const arrivalsRef = useRef<number[]>([])
  const [tailHost] = useState(() => document.createElement('span'))

  useLayoutEffect(() => {
    const html = htmlRef.current
    if (!html || html.contains(tailHost)) return

    findTailContainer(html).appendChild(tailHost)
  }, [parsed, tailHost])

  useLayoutEffect(() => {
    for (const span of tailHost.querySelectorAll<HTMLElement>('.stream-fade-char')) {
      const arrivedAt = arrivalsRef.current[Number(span.dataset.index)]
      if (arrivedAt === undefined) continue
      for (const animation of span.getAnimations()) {
        if (animation.startTime !== arrivedAt) animation.startTime = arrivedAt
      }
    }
  })

  useLayoutEffect(() => {
    latestRef.current = { content, messageId, animateLatestWord, parseMarkdown }

    const clearTimer = () => {
      if (timerRef.current !== null) {
        clearTimeout(timerRef.current)
        timerRef.current = null
      }
    }

    const render = (nextContent: string) => {
      const latest = latestRef.current
      const nextParsed = {
        content: nextContent,
        html: latest.parseMarkdown(nextContent),
        messageId: latest.messageId,
        parseMarkdown: latest.parseMarkdown,
      }
      parsedRef.current = nextParsed
      lastRenderAtRef.current = Date.now()
      setParsed(nextParsed)
    }

    const schedule = () => {
      if (timerRef.current !== null || snapshotsRef.current.length === 0) return

      const nextAt = Math.max(
        lastRenderAtRef.current + RENDER_INTERVAL_MS,
        snapshotsRef.current[0].at + FADE_DURATION_MS
      )
      timerRef.current = setTimeout(
        () => {
          timerRef.current = null
          const eligibleAt = Date.now() - FADE_DURATION_MS
          const snapshots = snapshotsRef.current
          let eligibleCount = 0
          while (eligibleCount < snapshots.length && snapshots[eligibleCount].at <= eligibleAt) {
            eligibleCount++
          }

          if (eligibleCount > 0) {
            const nextContent = snapshots[eligibleCount - 1].content.trimEnd()
            snapshots.splice(0, eligibleCount)
            render(nextContent)
          }
          schedule()
        },
        Math.max(0, nextAt - Date.now())
      )
    }

    const previous = parsedRef.current
    const contentChanged = !content.startsWith(previousContentRef.current)
    previousContentRef.current = content
    if (
      !animateLatestWord ||
      previous.messageId !== messageId ||
      previous.parseMarkdown !== parseMarkdown ||
      contentChanged
    ) {
      clearTimer()
      snapshotsRef.current = []
      arrivalsRef.current = []
      render(animateLatestWord ? '' : content)
    }

    if (animateLatestWord) {
      const now = Number(document.timeline.currentTime ?? performance.now())
      for (let i = arrivalsRef.current.length; i < content.length; i++) {
        arrivalsRef.current.push(now)
      }
    }

    if (animateLatestWord && content !== parsedRef.current.content) {
      snapshotsRef.current.push({ content, at: Date.now() })
      schedule()
    }
  }, [content, messageId, animateLatestWord, parseMarkdown])

  useLayoutEffect(
    () => () => {
      if (timerRef.current !== null) {
        clearTimeout(timerRef.current)
        timerRef.current = null
      }
    },
    []
  )

  const current =
    parsed.messageId === messageId && parsed.parseMarkdown === parseMarkdown ? parsed : null
  const parsedContent = current?.content ?? ''
  const tail =
    animateLatestWord && content.startsWith(parsedContent)
      ? content.slice(parsedContent.length)
      : ''
  let tailOffset = parsedContent.length
  const tailChars = Array.from(tail).map(char => {
    const offset = tailOffset
    tailOffset += char.length
    return { char, offset }
  })

  return (
    <div>
      <div
        ref={htmlRef}
        className="stream-markdown-html"
        dangerouslySetInnerHTML={{ __html: current?.html ?? '' }}
      />
      {createPortal(
        <span className="stream-markdown-tail">
          {tailChars.map(({ char, offset }) =>
            char === '\n' ? (
              <br key={offset} />
            ) : (
              <span className="stream-fade-char" key={offset} data-index={offset}>
                {char}
              </span>
            )
          )}
        </span>,
        tailHost
      )}
    </div>
  )
}
