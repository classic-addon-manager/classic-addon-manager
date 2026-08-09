import { cn } from '@/lib/utils.ts'
import type { AddonManifest, DependencyInfo } from '@/lib/wails'

import { Section } from './Section.tsx'

interface RequiresSectionProps {
  dependencies: DependencyInfo[]
  onDependencyClick: (depManifest: AddonManifest) => void
}

export const RequiresSection = ({ dependencies, onDependencyClick }: RequiresSectionProps) => (
  <Section label={`Requires (${dependencies.length})`}>
    <div className="flex flex-wrap gap-2">
      {dependencies.map(d => (
        <button
          key={d.manifest.name}
          type="button"
          className="inline-flex cursor-pointer items-center gap-2 rounded-full border border-border/60 bg-muted/30 px-3 py-1.5 text-sm transition-colors hover:border-primary/40 hover:bg-muted/60"
          onClick={() => onDependencyClick(d.manifest)}
          aria-label={`View details for ${d.manifest.alias}`}
          title={d.isInstalled ? 'Installed' : 'Not installed'}
        >
          <span
            className={cn(
              'h-2 w-2 shrink-0 rounded-full',
              d.isInstalled ? 'bg-primary' : 'border border-muted-foreground/40'
            )}
          />
          {d.manifest.alias}
        </button>
      ))}
    </div>
  </Section>
)
