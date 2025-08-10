import type { ReactNode } from 'react'

import { cn } from '@/lib/utils'

interface SidebarItemProps {
  name: string
  icon: ReactNode
  isActive?: boolean
  badgeCount?: number
  onClick: () => void
}

export const SidebarItem = ({
  name,
  icon,
  isActive = false,
  badgeCount = -1,
  onClick,
}: SidebarItemProps) => {
  return (
    <a
      href="#"
      className="relative flex items-center gap-3 rounded-lg px-3 py-2 group"
      onClick={e => {
        e.preventDefault()
        onClick()
      }}
    >
      <div
        className={cn(
          'absolute inset-0 rounded-lg bg-muted transition-opacity duration-500 ease-in-out',
          isActive ? 'opacity-100' : 'opacity-0 group-hover:opacity-10'
        )}
      />
      <div className="relative z-10 flex w-full items-center gap-3">
        <span
          className={cn(
            'transition-colors duration-300 ease-in-out',
            isActive ? 'text-primary' : 'text-muted-foreground group-hover:text-primary'
          )}
        >
          {icon}
        </span>
        <span
          className={cn(
            'transition-colors duration-300 ease-in-out',
            isActive ? 'text-primary' : 'text-muted-foreground group-hover:text-primary'
          )}
        >
          {name}
        </span>
        {badgeCount > 0 && (
          <span className="ml-auto flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-primary text-xs text-primary-foreground">
            {badgeCount}
          </span>
        )}
      </div>
    </a>
  )
}
