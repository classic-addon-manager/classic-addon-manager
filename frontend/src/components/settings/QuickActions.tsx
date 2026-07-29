import { ChevronRight, Database, FolderOpen } from 'lucide-react'

import { toast } from '@/components/ui/toast.tsx'
import { cn, getErrorMessage } from '@/lib/utils'
import { ApplicationService } from '@/lib/wails'

const actions = [
  {
    id: 'cache',
    label: 'Cache location',
    description: 'Temporary files and downloaded addon packages',
    icon: FolderOpen,
    handler: () => ApplicationService.OpenCacheDir(),
  },
  {
    id: 'data',
    label: 'Data location',
    description: 'Application config and persistent data',
    icon: Database,
    handler: () => ApplicationService.OpenDataDir(),
  },
] as const

export const QuickActions = () => {
  const run = async (label: string, handler: () => Promise<void>) => {
    try {
      await handler()
    } catch (error: unknown) {
      toast({
        title: 'Error',
        description: getErrorMessage(error, `Failed to open ${label}`),
      })
    }
  }

  return (
    <div className="divide-y divide-border/60">
      {actions.map(({ id, label, description, icon: Icon, handler }, index) => (
        <button
          key={id}
          type="button"
          onClick={() => run(label, handler)}
          className={cn(
            'group flex w-full items-center gap-3 px-4 py-3.5 text-left transition-colors',
            'hover:bg-muted/40 focus-visible:bg-muted/40 focus-visible:outline-none',
            index === 0 && 'rounded-t-xl',
            index === actions.length - 1 && 'rounded-b-xl'
          )}
        >
          <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-muted/60 text-muted-foreground transition-colors group-hover:bg-primary/10 group-hover:text-primary">
            <Icon className="h-4 w-4" />
          </div>
          <div className="min-w-0 flex-1">
            <div className="text-sm font-medium leading-none">{label}</div>
            <p className="mt-1 truncate text-xs text-muted-foreground">{description}</p>
          </div>
          <span className="flex shrink-0 items-center gap-1 text-xs text-muted-foreground transition-colors group-hover:text-foreground">
            Open
            <ChevronRight className="h-3.5 w-3.5 opacity-60 transition-transform group-hover:translate-x-0.5" />
          </span>
        </button>
      ))}
    </div>
  )
}
