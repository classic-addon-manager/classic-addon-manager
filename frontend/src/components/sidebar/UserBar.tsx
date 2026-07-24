import { Browser, Events } from '@wailsio/runtime'
import {
  AlertTriangleIcon,
  CheckIcon,
  ChevronUpIcon,
  LogOutIcon,
  MessageCircleQuestionIcon,
} from 'lucide-react'
import type { WailsEvent } from 'node_modules/@wailsio/runtime/types/events'
import { useEffect, useState } from 'react'

import { AIChatDialog } from '@/components/AIChatDialog'
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { toast } from '@/components/ui/toast'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { cn } from '@/lib/utils'
import { useUserStore } from '@/stores/userStore.ts'

const DISCORD_AUTH_URL =
  'https://discord.com/oauth2/authorize?client_id=1331010099916836914&response_type=code&redirect_uri=https%3A%2F%2Faac.gaijin.dev%2Fauth%2Fdiscord%2Fcallback2&scope=identify'

const DiscordIcon = ({ className }: { className?: string }) => (
  <svg viewBox="0 -28.5 256 256" className={className} aria-hidden>
    <path
      d="M216.856339,16.5966031 C200.285002,8.84328665 182.566144,3.2084988 164.041564,0 C161.766523,4.11318106 159.108624,9.64549908 157.276099,14.0464379 C137.583995,11.0849896 118.072967,11.0849896 98.7430163,14.0464379 C96.9108417,9.64549908 94.1925838,4.11318106 91.8971895,0 C73.3526068,3.2084988 55.6133949,8.86399117 39.0420583,16.6376612 C5.61752293,67.146514 -3.4433191,116.400813 1.08711069,164.955721 C23.2560196,181.510915 44.7403634,191.567697 65.8621325,198.148576 C71.0772151,190.971126 75.7283628,183.341335 79.7352139,175.300261 C72.104019,172.400575 64.7949724,168.822202 57.8887866,164.667963 C59.7209612,163.310589 61.5131304,161.891452 63.2445898,160.431257 C105.36741,180.133187 151.134928,180.133187 192.754523,160.431257 C194.506336,161.891452 196.298154,163.310589 198.110326,164.667963 C191.183787,168.842556 183.854737,172.420929 176.223542,175.320965 C180.230393,183.341335 184.861538,190.991831 190.096624,198.16893 C211.238746,191.588051 232.743023,181.531619 254.911949,164.955721 C260.227747,108.668201 245.831087,59.8662432 216.856339,16.5966031 Z M85.4738752,135.09489 C72.8290281,135.09489 62.4592217,123.290155 62.4592217,108.914901 C62.4592217,94.5396472 72.607595,82.7145587 85.4738752,82.7145587 C98.3405064,82.7145587 108.709962,94.5189427 108.488529,108.914901 C108.508531,123.290155 98.3405064,135.09489 85.4738752,135.09489 Z M170.525237,135.09489 C157.88039,135.09489 147.510584,123.290155 147.510584,108.914901 C147.510584,94.5396472 157.658606,82.7145587 170.525237,82.7145587 C183.391518,82.7145587 193.761324,94.5189427 193.539891,108.914901 C193.539891,123.290155 183.391518,135.09489 170.525237,135.09489 Z"
      fill="currentColor"
      fillRule="nonzero"
    />
  </svg>
)

export const UserBar = () => {
  const [chatOpen, setChatOpen] = useState(false)
  const { user, isAuthenticated, saveToken, signOut } = useUserStore()

  useEffect(() => {
    const handleAuthToken = async (event: WailsEvent) => {
      const tokenValue = Array.isArray(event.data) ? (event.data as unknown[])[0] : event.data

      if (typeof tokenValue === 'string' && tokenValue.length > 0) {
        try {
          await saveToken(tokenValue)
          toast({
            title: 'Success',
            description: 'You have successfully signed in.',
            icon: CheckIcon,
          })
        } catch {
          toast({
            title: 'Error',
            description: 'Failed to save authentication session.',
            icon: AlertTriangleIcon,
          })
        }
      } else {
        console.error('Unexpected auth token data:', event.data)
        toast({
          title: 'Error',
          description: 'Failed to retrieve authentication token.',
          icon: AlertTriangleIcon,
        })
      }
    }

    const unsubscribe = Events.On('authTokenReceived', handleAuthToken)
    return () => {
      unsubscribe()
    }
  }, [saveToken])

  const handleSignOut = async () => {
    try {
      await signOut()
    } catch {
      toast({
        title: 'Error',
        description: 'Failed to sign out. Session persists.',
        icon: AlertTriangleIcon,
      })
      return
    }
    setTimeout(() => {
      window.location.href = '/'
    }, 100)
  }

  if (!isAuthenticated()) {
    return (
      <div className="w-full px-3 pb-1">
        <button
          type="button"
          onClick={() => Browser.OpenURL(DISCORD_AUTH_URL)}
          className={cn(
            'group flex w-full items-center justify-center gap-2.5 rounded-lg px-3 py-2.5',
            'bg-[#5865F2]/90 text-white',
            'transition-colors duration-200 hover:bg-[#5865F2]'
          )}
        >
          <DiscordIcon className="h-4 w-4 shrink-0" />
          <span className="text-sm font-medium tracking-tight">Sign in with Discord</span>
        </button>
      </div>
    )
  }

  return (
    <div className="w-full px-3 pb-1">
      <div className="rounded-xl border border-border/50 bg-secondary/20 p-1.5">
        <div className="flex items-center gap-1">
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <button
                type="button"
                className={cn(
                  'group flex min-w-0 flex-1 items-center gap-2.5 rounded-lg px-2 py-1.5',
                  'text-left outline-none transition-colors duration-200',
                  'hover:bg-secondary/60 focus-visible:ring-1 focus-visible:ring-ring'
                )}
              >
                <Avatar className="h-8 w-8 shrink-0 ring-1 ring-border/60">
                  <AvatarImage
                    src={`https://cdn.discordapp.com/avatars/${user.discord_id}/${user.avatar}.png`}
                    alt={user.username}
                    className="h-full w-full object-cover"
                  />
                  <AvatarFallback className="text-[10px] font-medium">
                    {user.username.substring(0, 2).toUpperCase()}
                  </AvatarFallback>
                </Avatar>
                <div className="min-w-0 flex-1">
                  <p className="truncate text-sm font-medium leading-tight text-foreground">
                    {user.username}
                  </p>
                  <p className="truncate text-[11px] leading-tight text-muted-foreground">
                    Account
                  </p>
                </div>
                <ChevronUpIcon className="h-3.5 w-3.5 shrink-0 text-muted-foreground/70 transition-transform duration-200 group-data-[state=open]:rotate-180" />
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent className="w-52" side="top" align="start" sideOffset={8}>
              <DropdownMenuLabel className="font-normal">
                <div className="flex flex-col gap-0.5">
                  <span className="text-sm font-medium">{user.username}</span>
                  <span className="text-xs text-muted-foreground">Signed in with Discord</span>
                </div>
              </DropdownMenuLabel>
              <DropdownMenuSeparator />
              <DropdownMenuItem
                variant="destructive"
                onSelect={e => {
                  e.preventDefault()
                  void handleSignOut()
                }}
              >
                <LogOutIcon />
                Sign out
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>

          <Tooltip>
            <TooltipTrigger asChild>
              <button
                type="button"
                onClick={() => setChatOpen(true)}
                aria-label="Ask a friendly Daru"
                className={cn(
                  'flex h-9 w-9 shrink-0 items-center justify-center rounded-lg',
                  'text-muted-foreground outline-none transition-colors duration-200',
                  'hover:bg-secondary/60 hover:text-primary',
                  'focus-visible:ring-1 focus-visible:ring-ring'
                )}
              >
                <MessageCircleQuestionIcon className="h-4 w-4" />
              </button>
            </TooltipTrigger>
            <TooltipContent side="top" sideOffset={6}>
              Ask a friendly Daru
            </TooltipContent>
          </Tooltip>
        </div>
      </div>

      <AIChatDialog open={chatOpen} onOpenChange={setChatOpen} />
    </div>
  )
}
