import { AlertTriangleIcon, Code2, LoaderCircle, Plus } from 'lucide-react'
import { useState } from 'react'

import { OwnedAddonCard } from '@/components/developer/OwnedAddonCard'
import { PublishAddonForm } from '@/components/developer/PublishAddonForm'
import { SubmissionRow } from '@/components/developer/SubmissionRow'
import { type OwnedAddonsData, useOwnedAddons } from '@/components/developer/useOwnedAddons'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { cn } from '@/lib/utils'
import { useUserStore } from '@/stores/userStore.ts'

export const Developer = () => {
  const isAuthenticated = useUserStore(s => s.user.discord_id !== '')
  const token = useUserStore(s => s.token)
  const [view, setView] = useState<'list' | 'form'>('list')
  const { data, error, retry } = useOwnedAddons(isAuthenticated && view === 'list', token)

  if (!isAuthenticated) {
    return null
  }

  if (view === 'form') {
    return <PublishAddonForm onClose={() => setView('list')} />
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      <header className="border-b bg-background/95 backdrop-blur supports-backdrop-filter:bg-background/60">
        <div className="container flex h-16 items-center justify-between gap-4 px-4">
          <div className="flex items-center gap-3">
            <div className="rounded-lg bg-primary/10 p-2">
              <Code2 className="h-6 w-6 text-primary" />
            </div>
            <div>
              <h1 className="text-xl font-semibold tracking-tight">Developer</h1>
              <p className="text-sm text-muted-foreground">Your published addons</p>
            </div>
          </div>
          <Button type="button" className="w-32" onClick={() => setView('form')}>
            <Plus />
            New addon
          </Button>
        </div>
      </header>

      <ScrollArea className="min-h-0 flex-1">
        <div className="container mx-auto px-4 py-4">
          {renderListBody(data, error, retry, setView)}
        </div>
      </ScrollArea>
    </div>
  )
}

function renderListBody(
  data: OwnedAddonsData | null,
  error: string | null,
  retry: () => void,
  setView: (view: 'list' | 'form') => void
) {
  if (data === null) {
    if (error === null) {
      return (
        <div className="flex flex-1 flex-col items-center justify-center py-16 text-muted-foreground">
          <LoaderCircle className="size-10 animate-spin opacity-50" strokeWidth={1.5} />
          <p className="mt-3 text-sm">Loading your addons...</p>
        </div>
      )
    }
    return (
      <div className="flex flex-1 flex-col items-center justify-center py-16 text-muted-foreground">
        <AlertTriangleIcon className="size-8 text-destructive" />
        <p className="mt-3 text-sm">{error}</p>
        <Button type="button" className="mt-4" onClick={retry}>
          Retry
        </Button>
      </div>
    )
  }

  if (data.addons.length === 0 && data.submissions.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center p-8 text-center">
        <div className="mb-4 rounded-full bg-primary/10 p-4">
          <Code2 className="size-8 text-primary" />
        </div>
        <h3 className="mb-2 text-xl font-semibold tracking-tight">No addons yet</h3>
        <p className="mb-6 max-w-sm text-sm text-muted-foreground">
          Publish an addon to list it here.
        </p>
        <Button type="button" onClick={() => setView('form')}>
          <Plus />
          New addon
        </Button>
      </div>
    )
  }

  return (
    <>
      {data.addons.length > 0 && (
        <>
          <h2 className="mb-3 text-sm font-semibold tracking-tight">Published</h2>
          <div className="flex flex-col gap-3">
            {data.addons.map(addon => (
              <OwnedAddonCard key={addon.name} addon={addon} />
            ))}
          </div>
        </>
      )}
      {data.submissions.length > 0 && (
        <>
          <h2
            className={cn(
              'mb-3 text-sm font-semibold tracking-tight',
              data.addons.length > 0 && 'mt-8'
            )}
          >
            Submissions
          </h2>
          <div className="flex flex-col gap-2">
            {data.submissions.map(submission => (
              <SubmissionRow key={submission.prNumber} submission={submission} />
            ))}
          </div>
        </>
      )}
    </>
  )
}
