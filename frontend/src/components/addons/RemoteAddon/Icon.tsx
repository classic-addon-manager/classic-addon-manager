import { BlocksIcon } from 'lucide-react'

import type { AddonManifest } from '@/lib/wails'

interface IconProps {
  manifest: AddonManifest
  iconUrl: string
  hasIcon: boolean
  onIconError: () => void
  prominent?: boolean
}

export const Icon = ({ manifest, iconUrl, hasIcon, onIconError, prominent = false }: IconProps) => {
  const sizeClass = prominent ? 'h-14 w-14 rounded-xl' : 'h-10 w-10 rounded-lg'
  const fallbackIconClass = prominent ? 'h-7 w-7' : 'h-5 w-5'
  const containerClass = prominent
    ? `flex items-center justify-center ${sizeClass} bg-background border border-border/50 shadow-sm`
    : `flex items-center justify-center ${sizeClass} bg-background border border-border/50 shadow-xs`

  if (hasIcon) {
    return (
      <img
        className={`${sizeClass} object-cover border border-border/50 ${prominent ? 'shadow-sm' : 'shadow-xs'}`}
        src={iconUrl}
        alt={`${manifest.alias} icon`}
        loading="lazy"
        onError={onIconError}
      />
    )
  }

  return (
    <div className={containerClass}>
      <BlocksIcon className={`${fallbackIconClass} opacity-40 stroke-[1.5]`} />
    </div>
  )
}
