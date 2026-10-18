import { Button } from '@/components/ui/button'
import { type Deprecation, deprecationMessage } from '@/lib/deprecation'
import { cn } from '@/lib/utils'
import type { AddonManifest } from '@/lib/wails'

interface DeprecatedBannerProps {
  deprecation: Deprecation | null
  replacement?: AddonManifest | null
  /** Opens the replacement addon, the button only shows when given. */
  onViewReplacement?: (manifest: AddonManifest) => void
  className?: string
}

export const DeprecatedBanner = ({
  deprecation,
  replacement = null,
  onViewReplacement,
  className,
}: DeprecatedBannerProps) => {
  if (!deprecation) return null

  return (
    <div
      role="alert"
      className={cn(
        'flex items-start gap-2.5 bg-destructive px-6 py-3 text-sm text-white shadow',
        className
      )}
    >
      <span
        aria-hidden
        className="mt-0.5 inline-flex size-4 shrink-0 items-center justify-center rounded-full bg-white text-[11px] font-bold leading-none text-destructive"
      >
        !
      </span>
      <p className="flex-1">{deprecationMessage(deprecation, replacement)}</p>
      {replacement && onViewReplacement && (
        <Button
          size="sm"
          variant="secondary"
          className="h-7 shrink-0"
          onClick={() => onViewReplacement(replacement)}
        >
          View {replacement.alias}
        </Button>
      )}
    </div>
  )
}
