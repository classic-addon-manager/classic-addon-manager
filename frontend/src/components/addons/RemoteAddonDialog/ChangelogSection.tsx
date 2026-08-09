import { ChevronDownIcon } from 'lucide-react'
import { useEffect, useState } from 'react'

import { Readme } from '@/components/shared/Readme'
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '@/components/ui/collapsible.tsx'
import { Skeleton } from '@/components/ui/skeleton.tsx'
import { cn, daysAgo } from '@/lib/utils.ts'
import type { Release } from '@/lib/wails'

import { Section } from './Section.tsx'

const RECENT_RELEASE_DAYS = 14

interface ChangelogSectionProps {
  release: Release | null
  changelog: string
  isLoading: boolean
}

export const ChangelogSection = ({ release, changelog, isLoading }: ChangelogSectionProps) => {
  const isRecent = release ? daysAgo(release.published_at) < RECENT_RELEASE_DAYS : false
  const [isOpen, setIsOpen] = useState(isRecent)

  useEffect(() => {
    setIsOpen(isRecent)
  }, [isRecent])

  if (isLoading) {
    return (
      <Section label="What's New">
        <Skeleton className="h-6 w-1/2" />
      </Section>
    )
  }

  if (!release) {
    return (
      <Section label="What's New">
        <p className="text-sm font-medium text-muted-foreground">
          {changelog || 'No release available'}
        </p>
        {changelog === 'No change log was provided' && (
          <p className="mt-1 text-xs text-muted-foreground">
            The author hasn't provided release notes.
          </p>
        )}
        {changelog === 'Error loading change log' && (
          <p className="mt-1 text-xs text-destructive">Could not fetch release details.</p>
        )}
      </Section>
    )
  }

  return (
    <Section label="What's New">
      <Collapsible open={isOpen} onOpenChange={setIsOpen}>
        <CollapsibleTrigger className="flex w-full cursor-pointer items-center justify-between gap-2 text-sm font-medium transition-colors hover:text-primary">
          <span>What's new in {release.tag_name}</span>
          <ChevronDownIcon
            className={cn(
              'h-4 w-4 shrink-0 text-muted-foreground transition-transform',
              isOpen && 'rotate-180'
            )}
          />
        </CollapsibleTrigger>
        <CollapsibleContent>
          <div className="mt-3 rounded-lg bg-muted/40 p-4 text-sm text-foreground/90">
            <Readme readme={changelog} />
          </div>
        </CollapsibleContent>
      </Collapsible>
    </Section>
  )
}
