import { CheckIcon, CopyIcon } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'

import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'

const COPIED_FEEDBACK_MS = 1500

interface MessageActionsProps {
  content: string
  onCopyMessage: (content: string) => Promise<boolean>
}

export const MessageActions = ({ content, onCopyMessage }: MessageActionsProps) => {
  const [hasCopied, setHasCopied] = useState(false)
  const resetTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  useEffect(() => {
    return () => {
      if (resetTimeoutRef.current) clearTimeout(resetTimeoutRef.current)
    }
  }, [])

  const handleCopy = async () => {
    const copied = await onCopyMessage(content)
    if (!copied) return

    setHasCopied(true)
    if (resetTimeoutRef.current) clearTimeout(resetTimeoutRef.current)
    resetTimeoutRef.current = setTimeout(() => setHasCopied(false), COPIED_FEEDBACK_MS)
  }

  return (
    <div className="flex items-center gap-1">
      <Button
        variant="ghost"
        size="icon"
        aria-label={hasCopied ? 'Copied' : 'Copy answer'}
        title={hasCopied ? 'Copied' : 'Copy answer'}
        onClick={handleCopy}
        className={cn(
          'size-7 text-muted-foreground/60 opacity-0 transition-opacity hover:text-muted-foreground focus-visible:opacity-100 group-hover:opacity-100',
          hasCopied && 'text-primary opacity-100'
        )}
      >
        {hasCopied ? <CheckIcon className="h-3.5 w-3.5" /> : <CopyIcon className="h-3.5 w-3.5" />}
      </Button>
    </div>
  )
}
