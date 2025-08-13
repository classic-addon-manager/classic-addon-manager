import { Badge } from '@/components/ui/badge.tsx'
import type { DependencyInfo } from '@/lib/dependency-resolver.ts'
import type { AddonManifest } from '@/lib/wails'

interface DependenciesTabProps {
  dependencies: DependencyInfo[]
  onDependencyClick: (depManifest: AddonManifest) => void
}

export const DependenciesTab = ({ dependencies, onDependencyClick }: DependenciesTabProps) => {
  return (
    <>
      <p className="mb-4 text-sm text-muted-foreground">
        This addon requires the following dependencies (including transitive dependencies):
      </p>
      <div className="space-y-3">
        {dependencies.map(d => (
          <button
            key={d.manifest.name}
            className="flex flex-col w-full p-4 border rounded-lg bg-background hover:bg-muted/50 transition-colors text-left focus:outline-none focus:ring-0 focus:ring-offset-0"
            onClick={() => onDependencyClick(d.manifest)}
            aria-label={`View details for ${d.manifest.alias}`}
          >
            <div className="flex justify-between items-start mb-1 gap-2">
              <span className="font-medium text-base flex-1 break-words">{d.manifest.alias}</span>
              {d.isInstalled ? (
                <Badge variant="secondary" className="text-xs whitespace-nowrap">
                  Installed
                </Badge>
              ) : (
                <Badge variant="outline" className="text-xs whitespace-nowrap">
                  Not Installed
                </Badge>
              )}
            </div>
            <p className="text-sm text-muted-foreground mb-2">by {d.manifest.author}</p>
            <p className="text-sm text-foreground/80 line-clamp-2">
              {d.manifest.description || 'No description available.'}
            </p>
          </button>
        ))}
      </div>
    </>
  )
}
