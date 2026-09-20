import { AlertTriangleIcon, Code2, LoaderCircle, Plus } from 'lucide-react'
import { useState } from 'react'

import { type DeveloperScope, DeveloperToolbar } from '@/components/developer/DeveloperToolbar'
import { DeveloperWorkspace } from '@/components/developer/DeveloperWorkspace'
import { PublishAddonForm } from '@/components/developer/PublishAddonForm'
import { type OwnedAddonsData, useOwnedAddons } from '@/components/developer/useOwnedAddons'
import { Button } from '@/components/ui/button'
import { useUserStore } from '@/stores/userStore.ts'

export const Developer = () => {
  const discordId = useUserStore(s => s.user.discord_id)
  const isAuthenticated = discordId !== ''
  const [view, setView] = useState<'list' | 'form'>('list')
  const [selection, setSelection] = useState<string | null>(null)
  const [scope, setScope] = useState<DeveloperScope>('all')
  const { data, error, retry, removeSubmission, lastAttemptFailed } = useOwnedAddons(
    isAuthenticated && view === 'list'
  )

  const dropSubmission = (id: number) => {
    removeSubmission(id)
    if (selection === `submission:${id}`) setSelection(null)
  }

  const changeScope = (nextScope: DeveloperScope) => {
    setScope(nextScope)
    setSelection(null)
  }

  const openForm = () => setView('form')
  const closeForm = () => setView('list')

  if (!isAuthenticated) {
    return null
  }

  if (view === 'form') {
    return <PublishAddonForm onClose={closeForm} />
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      {data !== null && (
        <DeveloperToolbar
          data={data}
          lastAttemptFailed={lastAttemptFailed}
          onRefresh={retry}
          onNewAddon={openForm}
          scope={scope}
          onScopeChange={changeScope}
        />
      )}

      {renderListBody(
        data,
        error,
        retry,
        openForm,
        discordId,
        selection,
        setSelection,
        dropSubmission,
        scope
      )}
    </div>
  )
}

function renderListBody(
  data: OwnedAddonsData | null,
  error: string | null,
  retry: () => Promise<void>,
  openForm: () => void,
  accountKey: string,
  selection: string | null,
  onSelectionChange: (key: string) => void,
  onDropSubmission: (id: number) => void,
  scope: DeveloperScope
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
        <Button type="button" onClick={() => openForm()}>
          <Plus />
          New addon
        </Button>
      </div>
    )
  }

  return (
    <DeveloperWorkspace
      key={accountKey}
      data={data}
      selection={selection}
      onSelectionChange={onSelectionChange}
      onRefresh={retry}
      onDropSubmission={onDropSubmission}
      scope={scope}
    />
  )
}
