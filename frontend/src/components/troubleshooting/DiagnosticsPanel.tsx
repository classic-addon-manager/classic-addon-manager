import { AlertCircleIcon, AlertTriangle, ChevronRight, Loader2, RefreshCw } from 'lucide-react'

import { Button } from '@/components/ui/button'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { Skeleton } from '@/components/ui/skeleton'
import { cn } from '@/lib/utils'

export interface DiagnosticData {
  issueCount: number
  groupedIssues: Record<string, Array<{ type: string; error: string; file: string; count: number }>>
}

export const DiagnosticsPanel = ({
  data,
  error,
  hasResult,
  isLoading,
  onRescan,
}: {
  data: DiagnosticData
  error: string
  hasResult: boolean
  isLoading: boolean
  onRescan: () => void
}) => {
  const { issueCount, groupedIssues } = data
  const addonNames = Object.keys(groupedIssues)
  const initialLoading = isLoading && !hasResult
  const stale = isLoading && hasResult
  const staleClass = cn(
    'transition-opacity duration-200',
    stale && 'pointer-events-none opacity-50'
  )

  return (
    <>
      <div className="flex items-center justify-between gap-6 px-4 py-3.5">
        <div className="flex min-w-0 items-center gap-2.5" aria-live="polite">
          {isLoading ? (
            <>
              <Loader2 className="h-3.5 w-3.5 shrink-0 animate-spin text-muted-foreground" />
              <p className="text-sm text-muted-foreground">Scanning addon log...</p>
            </>
          ) : error ? (
            <>
              <span className="h-2 w-2 shrink-0 rounded-full bg-destructive" />
              <p className="text-sm font-medium">Scan failed</p>
            </>
          ) : issueCount === 0 ? (
            <>
              <span className="h-2 w-2 shrink-0 rounded-full bg-emerald-500" />
              <p className="text-sm font-medium">No issues detected</p>
            </>
          ) : (
            <>
              <span className="h-2 w-2 shrink-0 rounded-full bg-destructive" />
              <p className="text-sm font-medium tabular-nums">
                {issueCount} {issueCount === 1 ? 'issue' : 'issues'} across {addonNames.length}{' '}
                {addonNames.length === 1 ? 'addon' : 'addons'}
              </p>
            </>
          )}
        </div>
        <Button variant="ghost" size="sm" onClick={onRescan} disabled={isLoading}>
          <RefreshCw className={cn('h-3.5 w-3.5', isLoading && 'animate-spin')} />
          Rescan
        </Button>
      </div>

      {error && (
        <div className={cn('border-t border-border/60 bg-muted/15 px-4 py-4', staleClass)}>
          <div className="flex gap-2 rounded-lg border border-destructive/20 bg-destructive/10 p-3">
            <AlertCircleIcon className="mt-0.5 h-4 w-4 shrink-0 text-destructive" />
            <div className="space-y-0.5">
              <div className="text-sm font-medium text-destructive">Diagnostics failed</div>
              <p className="text-xs text-destructive/80">{error}</p>
            </div>
          </div>
        </div>
      )}

      {initialLoading ? (
        <div className="divide-y divide-border/60 border-t border-border/60">
          {[0, 1].map(i => (
            <div key={i} className="flex items-center gap-3 px-4 py-3.5">
              <Skeleton className="h-9 w-9 rounded-lg" />
              <div className="min-w-0 flex-1 space-y-2">
                <Skeleton className="h-3.5 w-32" />
                <Skeleton className="h-3 w-20" />
              </div>
            </div>
          ))}
        </div>
      ) : (
        issueCount > 0 && (
          <div
            aria-busy={stale}
            className={cn('divide-y divide-border/60 border-t border-border/60', staleClass)}
          >
            {addonNames.map(addonName => (
              <Collapsible key={addonName}>
                <CollapsibleTrigger className="group flex w-full items-center gap-3 px-4 py-3.5 text-left transition-colors hover:bg-muted/40 focus-visible:bg-muted/40 focus-visible:outline-none">
                  <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-destructive/10 text-destructive">
                    <AlertTriangle className="h-4 w-4" />
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="text-sm font-medium leading-none">
                      {addonName === 'x2ui' ? 'Addon API' : addonName}
                    </div>
                    <p className="mt-1 text-xs text-muted-foreground">
                      {groupedIssues[addonName].length}{' '}
                      {groupedIssues[addonName].length === 1 ? 'issue' : 'issues'}
                    </p>
                  </div>
                  <ChevronRight className="h-3.5 w-3.5 shrink-0 text-muted-foreground transition-transform group-data-[state=open]:rotate-90" />
                </CollapsibleTrigger>
                <CollapsibleContent>
                  <div className="space-y-2 border-t border-border/60 bg-muted/15 px-4 py-3">
                    {groupedIssues[addonName].map((issue, i) => (
                      <div
                        key={`${issue.file}-${issue.error}-${i}`}
                        className="rounded-lg border border-border/60 bg-background/40 px-3 py-2.5"
                      >
                        <div className="flex items-start gap-3">
                          <p className="min-w-0 flex-1 font-mono text-xs break-words text-destructive">
                            {issue.error}
                          </p>
                          {issue.count > 1 && (
                            <span className="shrink-0 text-xs tabular-nums text-muted-foreground">
                              ×{issue.count}
                            </span>
                          )}
                        </div>
                        <p className="mt-1.5 font-mono text-xs break-all text-muted-foreground">
                          <span className="text-foreground/70">{issue.file}</span>
                        </p>
                      </div>
                    ))}
                  </div>
                </CollapsibleContent>
              </Collapsible>
            ))}
          </div>
        )
      )}
    </>
  )
}
