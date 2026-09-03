import { Code2, Plus } from 'lucide-react'
import { useState } from 'react'

import { PublishAddonForm } from '@/components/developer/PublishAddonForm'
import { Button } from '@/components/ui/button'
import { useUserStore } from '@/stores/userStore.ts'

export const Developer = () => {
  const isAuthenticated = useUserStore(s => s.user.discord_id !== '')
  const [view, setView] = useState<'list' | 'form'>('list')

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

      <div className="flex flex-1 flex-col items-center justify-center p-8 text-center">
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
    </div>
  )
}
