import DOMPurify from 'dompurify'
import { marked } from 'marked'

import { toast } from '@/components/ui/toast'

export const generateMessageId = () => {
  return `msg_${Date.now()}_${Math.random().toString(36).substring(2, 9)}`
}

export const parseMarkdown = (content: string) => {
  return DOMPurify.sanitize(marked.parse(content, { async: false }) as string, {
    ADD_ATTR: ['data-url', 'onclick'],
  })
}

export const copyToClipboard = async (text: string) => {
  try {
    await navigator.clipboard.writeText(text)
    toast({
      title: 'Success',
      description: 'Copied to clipboard',
    })
  } catch {
    toast({
      title: 'Error',
      description: 'Failed to copy text',
    })
  }
}
