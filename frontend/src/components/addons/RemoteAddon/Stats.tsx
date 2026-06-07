import { DownloadIcon, HeartIcon } from 'lucide-react'

import type { AddonManifest } from '@/lib/wails'

interface StatsProps {
  manifest: AddonManifest
  compact?: boolean
}

export const Stats = ({ manifest, compact = false }: StatsProps) => (
  <div
    className={`flex items-center gap-2 text-muted-foreground ${compact ? 'text-[10px]' : 'text-xs'}`}
  >
    <div className="flex items-center gap-0.5" title={`${manifest.downloads} downloads`}>
      <DownloadIcon className={compact ? 'h-2.5 w-2.5' : 'w-3 h-3'} /> {manifest.downloads}
    </div>
    {manifest.like_percentage && (
      <div className="flex items-center gap-0.5" title={`${manifest.like_percentage}% likes`}>
        <HeartIcon className={compact ? 'h-2.5 w-2.5' : 'w-3 h-3'} /> {manifest.like_percentage}%
      </div>
    )}
  </div>
)
