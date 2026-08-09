import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar'
import { cn } from '@/lib/utils'
import { useUserStore } from '@/stores/userStore'

import type { ChatMessageType } from './types'

interface UserMessageProps {
  message: ChatMessageType
  isAnimating: boolean
}

export const UserMessage = ({ message, isAnimating }: UserMessageProps) => {
  const { user } = useUserStore()

  return (
    <div
      className={cn(
        'flex items-start justify-end gap-3',
        isAnimating && 'animate-in fade-in-0 slide-in-from-bottom-2 duration-600'
      )}
    >
      <div className="max-w-[80%] rounded-2xl bg-primary px-4 py-2 text-sm text-primary-foreground break-words whitespace-pre-wrap select-text">
        {message.content}
      </div>

      <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-secondary">
        <Avatar className="h-full w-full">
          <AvatarImage
            src={`https://cdn.discordapp.com/avatars/${user.discord_id}/${user.avatar}.png`}
            alt={user.username}
            className="h-full w-full object-cover"
          />
          <AvatarFallback className="text-xs">
            {user.username.substring(0, 2).toUpperCase()}
          </AvatarFallback>
        </Avatar>
      </div>
    </div>
  )
}
