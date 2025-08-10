import supportDaruAlt from '@/assets/images/support_daru_alt_sm.webp'
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar'
import { useUserStore } from '@/stores/userStore'

import type { ChatMessageType } from './types'

interface ChatMessageProps {
  message: ChatMessageType
  isAnimating: boolean
  onCopyMessage: (content: string) => void
  parseMarkdown: (content: string) => string
}

export const ChatMessage = ({
  message,
  isAnimating,
  onCopyMessage,
  parseMarkdown,
}: ChatMessageProps) => {
  const { user } = useUserStore()

  return (
    <div
      key={message.id}
      className={`flex items-start gap-3 px-4 ${message.role === 'user' ? 'justify-end' : ''} ${
        isAnimating ? 'animate-in fade-in-0 slide-in-from-bottom-2 duration-600' : ''
      }`}
    >
      {message.role === 'assistant' && (
        <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-primary/10 overflow-hidden">
          <img src={supportDaruAlt} alt="Daru Assistant" className="w-full h-full object-cover" />
        </div>
      )}

      <div
        className={`flex flex-col gap-2 ${
          message.role === 'assistant' ? 'max-w-[calc(100%-3.5rem)] w-full' : 'max-w-[80%]'
        }`}
      >
        <div
          className={`relative inline-block rounded-lg px-3 py-1.5 text-sm chat-message group ${
            message.role === 'assistant' ? 'assistant-message bg-muted/50' : 'bg-primary text-black'
          }`}
        >
          {message.role === 'assistant' ? (
            <>
              <div
                dangerouslySetInnerHTML={{
                  __html: parseMarkdown(message.content),
                }}
              />
              <button
                className="copy-button absolute top-1 right-1 p-1 rounded-md text-muted-foreground/60 hover:text-muted-foreground hover:bg-secondary/50 transition-all duration-150 opacity-0 group-hover:opacity-100"
                onClick={() => onCopyMessage(message.content)}
              >
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  width="14"
                  height="14"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  <rect width="8" height="4" x="8" y="2" rx="1" ry="1" />
                  <path d="M8 4H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2" />
                  <path d="M16 4h2a2 2 0 0 1 2 2v4" />
                  <path d="M21 14H11" />
                  <path d="m15 10-4 4 4 4" />
                </svg>
              </button>
            </>
          ) : (
            message.content
          )}
        </div>
      </div>

      {message.role === 'user' && (
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
      )}
    </div>
  )
}
