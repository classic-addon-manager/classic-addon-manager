import { useLayoutEffect, useRef } from 'react'

const FADE_CLASS = 'stream-fade-char'

const findLastTextNode = (root: HTMLElement): Text | null => {
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT)
  let lastText: Text | null = null
  let node: Node | null

  while ((node = walker.nextNode())) {
    if ((node.textContent ?? '').length > 0) {
      lastText = node as Text
    }
  }

  return lastText
}

const unwrapFadeSpan = (root: HTMLElement) => {
  const fading = root.querySelector(`.${FADE_CLASS}`)
  if (!fading?.parentNode || !fading.textContent) return

  fading.parentNode.replaceChild(document.createTextNode(fading.textContent), fading)
}

const wrapLatestChar = (root: HTMLElement) => {
  if (root.querySelector(`.${FADE_CLASS}`)) return

  const lastText = findLastTextNode(root)
  if (!lastText?.textContent) return

  const text = lastText.textContent
  const before = text.slice(0, -1)
  const char = text.slice(-1)
  const parent = lastText.parentNode
  if (!parent || !char) return

  const fragment = document.createDocumentFragment()
  if (before) fragment.appendChild(document.createTextNode(before))

  const span = document.createElement('span')
  span.className = FADE_CLASS
  span.textContent = char
  fragment.appendChild(span)

  parent.replaceChild(fragment, lastText)
}

const appendCharToContainer = (root: HTMLElement, char: string) => {
  unwrapFadeSpan(root)

  const lastText = findLastTextNode(root)
  if (!lastText) return

  lastText.textContent = `${lastText.textContent ?? ''}${char}`
  wrapLatestChar(root)
}

const needsFullRender = (prevContent: string, content: string) => {
  if (prevContent === '') return true
  if (!content.startsWith(prevContent)) return true

  const added = content.slice(prevContent.length)
  return /\n/.test(added)
}

interface StreamingMarkdownProps {
  content: string
  messageId: string
  animateLatestWord: boolean
  parseMarkdown: (content: string) => string
}

export const StreamingMarkdown = ({
  content,
  messageId,
  animateLatestWord,
  parseMarkdown,
}: StreamingMarkdownProps) => {
  const containerRef = useRef<HTMLDivElement>(null)
  const prevContentRef = useRef('')
  const messageIdRef = useRef(messageId)

  if (messageIdRef.current !== messageId) {
    messageIdRef.current = messageId
    prevContentRef.current = ''
  }

  useLayoutEffect(() => {
    const container = containerRef.current
    if (!container) return

    if (!animateLatestWord) {
      if (content !== prevContentRef.current) {
        container.innerHTML = parseMarkdown(content)
        prevContentRef.current = content
      }
      return
    }

    if (content === prevContentRef.current) return

    const prevContent = prevContentRef.current

    if (needsFullRender(prevContent, content)) {
      container.innerHTML = parseMarkdown(content)
      if (content.length > 0) {
        wrapLatestChar(container)
      }
    } else {
      const added = content.slice(prevContent.length)
      for (const char of added) {
        appendCharToContainer(container, char)
      }
    }

    prevContentRef.current = content
  }, [content, messageId, animateLatestWord, parseMarkdown])

  return <div ref={containerRef} />
}
