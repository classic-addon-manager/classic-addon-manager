import type { CSSProperties } from 'react'

import { Badge } from '@/components/ui/badge'
import { hueFromString } from '@/lib/hue'

interface TagDotsProps {
  tags: string[]
  maxTags: number
}

export const TagDots = ({ tags, maxTags }: TagDotsProps) => {
  if (tags.length === 0) return null

  const visibleTags = tags.slice(0, maxTags)
  const hiddenCount = tags.length - visibleTags.length

  return (
    <div
      className="flex items-center gap-1.5 overflow-hidden whitespace-nowrap text-xs text-muted-foreground"
      title={tags.join(', ')}
    >
      {visibleTags.map((tag, index) => (
        <Badge
          key={`${tag}-${index}`}
          variant="outline"
          style={{ '--h': hueFromString(tag) } as CSSProperties}
          className="border-[oklch(0.72_0.14_var(--h)/0.35)] bg-[oklch(0.72_0.14_var(--h)/0.12)] px-1.5 py-0 text-[11px] text-[oklch(0.82_0.1_var(--h))]"
        >
          {tag}
        </Badge>
      ))}
      {hiddenCount > 0 && <span className="text-muted-foreground/60">+{hiddenCount}</span>}
    </div>
  )
}
