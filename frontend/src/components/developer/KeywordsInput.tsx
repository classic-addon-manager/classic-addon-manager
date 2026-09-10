import { X } from 'lucide-react'
import { type ChangeEvent, type KeyboardEvent, useState } from 'react'

import { Badge } from '@/components/ui/badge'
import { cn } from '@/lib/utils'

interface KeywordsInputProps {
  id?: string
  value: string[]
  onChange: (value: string[]) => void
  invalid?: boolean
  disabled?: boolean
}

export const KeywordsInput = ({
  id,
  value,
  onChange,
  invalid = false,
  disabled = false,
}: KeywordsInputProps) => {
  const [draft, setDraft] = useState('')

  const addTokens = (parts: string[]) => {
    const added = parts.filter(Boolean)
    if (added.length === 0) return
    onChange([...value, ...added])
  }

  const commitDraft = () => {
    const next = draft.trim()
    if (!next) {
      if (draft) setDraft('')
      return
    }
    addTokens([next])
    setDraft('')
  }

  const handleChange = (event: ChangeEvent<HTMLInputElement>) => {
    const next = event.target.value
    if (!/\s/.test(next)) {
      setDraft(next)
      return
    }
    const parts = next.split(/\s+/)
    const pending = /\s$/.test(next) ? '' : (parts.pop() ?? '')
    addTokens(parts)
    setDraft(pending)
  }

  const handleKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.nativeEvent.isComposing) return
    if (event.key === 'Enter') {
      event.preventDefault()
      commitDraft()
      return
    }
    if (event.key === 'Backspace' && draft === '' && value.length > 0) {
      event.preventDefault()
      onChange(value.slice(0, -1))
    }
  }

  return (
    <div
      className={cn(
        'dark:bg-input/30 border-input flex min-h-9 w-full flex-wrap items-center gap-1.5 rounded-md border bg-transparent px-3 py-1 shadow-xs transition-[color,box-shadow]',
        'focus-within:border-ring focus-within:ring-ring/50 focus-within:ring-[3px]',
        invalid &&
          'border-destructive focus-within:border-destructive focus-within:ring-destructive/20 dark:focus-within:ring-destructive/40',
        disabled && 'pointer-events-none cursor-not-allowed opacity-50'
      )}
    >
      {value.map((token, index) => (
        <Badge key={`${index}-${token}`} variant="secondary" className="gap-1 rounded-full pr-1">
          {token}
          <button
            type="button"
            disabled={disabled}
            onClick={() => onChange(value.filter((_, itemIndex) => itemIndex !== index))}
            className="rounded-full p-0.5 hover:bg-background/50 disabled:pointer-events-none"
            aria-label={`Remove ${token}`}
          >
            <X className="size-3" />
          </button>
        </Badge>
      ))}
      <input
        id={id}
        value={draft}
        disabled={disabled}
        aria-invalid={invalid || undefined}
        onChange={handleChange}
        onKeyDown={handleKeyDown}
        onBlur={commitDraft}
        className="placeholder:text-muted-foreground min-w-[8rem] flex-1 bg-transparent text-sm outline-none disabled:cursor-not-allowed"
      />
    </div>
  )
}
