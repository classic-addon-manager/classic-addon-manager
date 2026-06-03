import { useAtomValue } from 'jotai'

import { activeComponentAtom } from '@/atoms/sidebarAtoms'
import { AppUpdateDialog } from '@/components/AppUpdateDialog'
import { Sidebar } from '@/components/sidebar'
import { SupportProjectDialog } from '@/components/SupportProjectDialog'
import { TitleBar } from '@/components/TitleBar'
import { WindowResizeHandles } from '@/components/WindowResizeHandles'

export default function UI() {
  const ActiveComponent = useAtomValue(activeComponentAtom)
  return (
    <>
      <AppUpdateDialog />
      <SupportProjectDialog />
      <WindowResizeHandles />
      <div className="flex h-full min-h-0 w-full flex-col overflow-hidden bg-background">
        <TitleBar />
        <div className="grid min-h-0 flex-1 w-full md:grid-cols-[220px_1fr]">
          <div className="border-r bg-muted/40">
            <div className="flex h-full min-h-0 flex-col py-2">
              <Sidebar />
            </div>
          </div>

          <div className="flex min-h-0 flex-col overflow-hidden">
            <ActiveComponent />
          </div>
        </div>
      </div>
    </>
  )
}
