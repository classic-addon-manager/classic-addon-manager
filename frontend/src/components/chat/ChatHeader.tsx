import { Button } from '@/components/ui/button'
import { DialogDescription, DialogTitle } from '@/components/ui/dialog'

interface ChatHeaderProps {
  onClose: () => void
}

export const ChatHeader = ({ onClose }: ChatHeaderProps) => {
  return (
    <div className="border-b border-border/40 px-4 sm:px-6 py-4">
      <div className="flex items-center justify-between">
        <div>
          <DialogTitle className="text-lg font-medium leading-none">
            Daru Informational Network
          </DialogTitle>
          <DialogDescription className="mt-2 text-sm text-muted-foreground">
            The Darus have information, if you have coin.
          </DialogDescription>
        </div>
        <Button variant="ghost" size="icon" onClick={onClose} className="h-6 w-6">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="m18 6-12 12" />
            <path d="m6 6 12 12" />
          </svg>
        </Button>
      </div>
    </div>
  )
}
