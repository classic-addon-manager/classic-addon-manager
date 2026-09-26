import { AddonIconImage } from '@/components/shared/AddonIconImage'
import type { AddonManifest } from '@/lib/wails'

interface IconProps {
  manifest: AddonManifest
  iconUrl: string | null
  prominent?: boolean
}

export const Icon = ({ manifest, iconUrl, prominent = false }: IconProps) => {
  const sizeClass = prominent ? 'h-14 w-14 rounded-xl' : 'h-10 w-10 rounded-lg'
  const shadowClass = prominent ? 'shadow-sm' : 'shadow-xs'
  const fallbackIconClass = prominent ? 'h-7 w-7' : 'h-5 w-5'

  return (
    <AddonIconImage
      src={iconUrl}
      alt={`${manifest.alias} icon`}
      imageClassName={`${sizeClass} object-cover border border-border/50 ${shadowClass}`}
      fallbackClassName={`flex items-center justify-center ${sizeClass} bg-background border border-border/50 ${shadowClass}`}
      fallbackIconClassName={`${fallbackIconClass} opacity-40 stroke-[1.5]`}
    />
  )
}
