import aacLogo from '@/assets/images/aac-logo-wide.png'
import { Sidebar } from '@/components/sidebar'
import { useAtomValue } from 'jotai'
import { activeComponentAtom } from '@/atoms/sidebarAtoms'

export default function UI() {
  const ActiveComponent = useAtomValue(activeComponentAtom)
  return (
    <div className="grid min-h-screen w-full md:grid-cols-[220px_1fr]">
      <div className="bg-muted/40 border-r">
        <div className="flex h-full max-h-screen flex-col gap-2">
          <div className="flex h-14 items-center border-b px-4">
            <div className="flex items-center gap-2 font-semibold">
              <img src={aacLogo} alt="AAC Logo" className="h-12 w-auto" />
            </div>
          </div>

          <Sidebar />
        </div>
      </div>

      {/* Active component area */}
      <div className="flex flex-col">
        <ActiveComponent />
      </div>
    </div>
  )
}
