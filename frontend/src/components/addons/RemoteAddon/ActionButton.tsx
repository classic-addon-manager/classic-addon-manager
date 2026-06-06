import { CheckIcon, DownloadIcon } from 'lucide-react'

import { Button } from '@/components/ui/button'

interface ActionButtonProps {
  installed: boolean
  compact?: boolean
}

export const ActionButton = ({ installed, compact = false }: ActionButtonProps) => {
  const iconClass = compact ? 'h-4 w-4' : 'h-5 w-5'
  const buttonClass = compact ? 'h-7 w-7' : undefined

  if (installed) {
    return (
      <Button
        variant="ghost"
        size="icon"
        className={`cursor-default text-green-500 focus-visible:ring-0 focus-visible:ring-offset-0 ${buttonClass ?? ''}`}
        disabled={true}
        aria-label="Installed"
        tabIndex={-1}
      >
        <CheckIcon className={iconClass} />
      </Button>
    )
  }

  return (
    <Button
      variant="ghost"
      size="icon"
      aria-label="View details and install"
      className={`cursor-pointer text-muted-foreground hover:text-primary focus-visible:ring-primary/40 ${buttonClass ?? ''}`}
    >
      <DownloadIcon className={iconClass} />
    </Button>
  )
}
