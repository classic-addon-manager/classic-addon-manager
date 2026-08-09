import { useEffect, useRef, useState } from 'react'

import { Readme } from '@/components/shared/Readme'
import { Skeleton } from '@/components/ui/skeleton.tsx'
import { cn } from '@/lib/utils.ts'

import { Section } from './Section.tsx'

const COLLAPSED_MAX_HEIGHT = 280

interface DescriptionSectionProps {
  readme: string
  isLoading: boolean
}

export const DescriptionSection = ({ readme, isLoading }: DescriptionSectionProps) => {
  const [isExpanded, setIsExpanded] = useState(false)
  const [isOverflowing, setIsOverflowing] = useState(false)
  const contentRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const content = contentRef.current
    if (!content) return

    // Markdown images resolve after mount, so remeasure whenever the rendered height changes.
    const observer = new ResizeObserver(() => {
      setIsOverflowing(content.scrollHeight > COLLAPSED_MAX_HEIGHT)
    })
    observer.observe(content)
    return () => observer.disconnect()
  }, [readme, isLoading])

  if (isLoading) {
    return (
      <Section label="Description">
        <div className="space-y-2">
          <Skeleton className="h-4 w-2/3" />
          <Skeleton className="h-4 w-full" />
          <Skeleton className="h-4 w-full" />
          <Skeleton className="h-4 w-4/5" />
        </div>
      </Section>
    )
  }

  return (
    <Section label="Description">
      <div className={cn('relative', !isExpanded && 'max-h-[280px] overflow-hidden')}>
        <div ref={contentRef} className="max-w-none text-sm">
          <Readme readme={readme} />
        </div>
        {!isExpanded && isOverflowing && (
          <div className="pointer-events-none absolute inset-x-0 bottom-0 h-20 bg-gradient-to-t from-background to-transparent" />
        )}
      </div>

      {isOverflowing && (
        <button
          type="button"
          className="mt-2 cursor-pointer text-xs font-medium text-primary transition-colors hover:text-primary/80"
          onClick={() => setIsExpanded(expanded => !expanded)}
        >
          {isExpanded ? 'Show less' : 'Show more'}
        </button>
      )}
    </Section>
  )
}
