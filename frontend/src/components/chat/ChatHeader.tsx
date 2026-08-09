import { MessageSquarePlusIcon, XIcon } from 'lucide-react'

import { Button } from '@/components/ui/button'
import { DialogDescription, DialogTitle } from '@/components/ui/dialog'

interface ChatHeaderProps {
  onClose: () => void
  onNewConversation: () => void
  canStartNewConversation: boolean
}

export const ChatHeader = ({
  onClose,
  onNewConversation,
  canStartNewConversation,
}: ChatHeaderProps) => {
  return (
    <div className="border-b border-border/40 px-4 py-4 sm:px-6">
      <div className="flex items-center justify-between">
        <div>
          <DialogTitle className="text-lg leading-none font-medium">
            Daru Informational Network
          </DialogTitle>
          <DialogDescription className="mt-2 text-sm text-muted-foreground">
            The Darus have information, if you have coin.
          </DialogDescription>
        </div>
        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            size="icon"
            onClick={onNewConversation}
            disabled={!canStartNewConversation}
            aria-label="New conversation"
            title="New conversation"
            className="size-7 text-muted-foreground hover:text-foreground"
          >
            <MessageSquarePlusIcon className="h-4 w-4" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            onClick={onClose}
            aria-label="Close chat"
            title="Close chat"
            className="size-7"
          >
            <XIcon className="h-4 w-4" />
          </Button>
        </div>
      </div>
    </div>
  )
}
