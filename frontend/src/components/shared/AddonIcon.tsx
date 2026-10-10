import { type CSSProperties, useState } from 'react'

import { hueFromString } from '@/lib/hue'
import { cn } from '@/lib/utils'

type AddonIconSize = 'sm' | 'md' | 'lg'

interface AddonIconProps {
  /** Addon display name, used for the alt text and the monogram. */
  name: string
  src: string | null
  size?: AddonIconSize
  className?: string
}

const sizeClasses: Record<AddonIconSize, string> = {
  sm: 'h-9 w-9 rounded-lg text-sm',
  md: 'h-10 w-10 rounded-lg text-[15px]',
  lg: 'h-14 w-14 rounded-xl text-xl',
}

const initials = (name: string): string => {
  const words = name
    .replace(/[^\p{L}\p{N} ]/gu, ' ')
    .trim()
    .split(/\s+/)
    .filter(Boolean)

  if (words.length === 0) return '?'
  if (words.length === 1) return words[0].slice(0, 2)
  return words[0][0] + words[1][0]
}

/** An addon's icon, or a monogram coloured from its name when it has none or it fails to load. */
export const AddonIcon = ({ name, src, size = 'md', className }: AddonIconProps) => {
  // Remembering the failed URL instead of a flag lets a new URL retry without an effect.
  const [failedSrc, setFailedSrc] = useState<string | null>(null)

  if (src !== null && src !== failedSrc) {
    return (
      <img
        className={cn(
          'shrink-0 border border-border/50 object-cover shadow-xs',
          sizeClasses[size],
          className
        )}
        src={src}
        alt={`${name} icon`}
        loading="lazy"
        onError={() => setFailedSrc(src)}
      />
    )
  }

  return (
    <div
      aria-hidden="true"
      style={{ '--h': hueFromString(name) } as CSSProperties}
      className={cn(
        'flex shrink-0 items-center justify-center font-semibold uppercase tracking-tight text-[oklch(0.88_0.1_var(--h))] bg-[linear-gradient(140deg,oklch(0.36_0.09_var(--h)),oklch(0.24_0.06_var(--h)))] inset-ring inset-ring-[oklch(0.6_0.1_var(--h)/0.28)]',
        sizeClasses[size],
        className
      )}
    >
      {initials(name)}
    </div>
  )
}
