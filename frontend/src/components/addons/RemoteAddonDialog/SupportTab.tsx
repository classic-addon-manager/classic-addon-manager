import { Browser } from '@wailsio/runtime'
import { HeartIcon } from 'lucide-react'

import { Button } from '@/components/ui/button.tsx'
import type { AddonManifest } from '@/lib/wails'

interface SupportTabProps {
  manifest: AddonManifest
}

export const SupportTab = ({ manifest }: SupportTabProps) => {
  if (!manifest.kofi) return null

  return (
    <div className="flex flex-col items-center justify-center text-center space-y-4 p-4 border rounded-lg bg-secondary/30">
      <HeartIcon className="w-10 h-10 text-red-500" />
      <p className="text-sm text-muted-foreground">
        Enjoying {manifest.alias}? Show your appreciation by buying {manifest.author} a coffee!
      </p>
      <Button
        variant="default"
        onClick={() => Browser.OpenURL(`https://ko-fi.com/${manifest.kofi}`)}
      >
        <HeartIcon className="w-4 h-4 mr-2" />
        Support {manifest.author} on Ko-fi
      </Button>
    </div>
  )
}
