import { Window } from '@wailsio/runtime'
import { useAtomValue } from 'jotai'
import { type MouseEvent, type ReactNode, useCallback, useEffect, useMemo, useState } from 'react'

import aacLogo from '@/assets/images/aac-logo-wide.webp'
import { activePageAtom } from '@/atoms/sidebarAtoms'
import { PAGE_DEFINITIONS } from '@/components/sidebar/pageDefinitions'
import { cn } from '@/lib/utils'

const WINDOW_CONTROL_BUTTON =
  'no-drag inline-flex h-8 w-8 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/70'

const WindowControlIcon = ({ children }: { children: ReactNode }) => (
  <span className="pointer-events-none flex h-4 w-4 items-center justify-center">{children}</span>
)

export function TitleBar() {
  const activePage = useAtomValue(activePageAtom)
  const [isMaximised, setIsMaximised] = useState(false)

  const activeDefinition = useMemo(
    () => PAGE_DEFINITIONS.find(page => page.id === activePage) ?? PAGE_DEFINITIONS[0],
    [activePage]
  )

  const syncMaximisedState = useCallback(async () => {
    try {
      setIsMaximised(await Window.IsMaximised())
    } catch (error) {
      console.error('Failed to query maximise state:', error)
    }
  }, [])

  useEffect(() => {
    void syncMaximisedState()

    const onResize = () => {
      void syncMaximisedState()
    }
    const onFocus = () => {
      void syncMaximisedState()
    }

    window.addEventListener('resize', onResize)
    window.addEventListener('focus', onFocus)

    return () => {
      window.removeEventListener('resize', onResize)
      window.removeEventListener('focus', onFocus)
    }
  }, [syncMaximisedState])

  const handleToggleMaximise = useCallback(async () => {
    try {
      if (await Window.IsMaximised()) {
        await Window.Restore()
      } else {
        await Window.Maximise()
      }
      await syncMaximisedState()
    } catch (error) {
      console.error('Failed to toggle maximise state:', error)
    }
  }, [syncMaximisedState])

  const handleTitlebarDoubleClick = useCallback(
    async (event: MouseEvent<HTMLElement>) => {
      if (event.target instanceof Element && event.target.closest('.no-drag')) {
        return
      }
      await handleToggleMaximise()
    },
    [handleToggleMaximise]
  )

  const ActivePageIcon = activeDefinition.icon

  return (
    <header
      className="drag-region grid h-14 shrink-0 grid-cols-[220px_1fr_auto] items-center border-b bg-background/95 px-3 backdrop-blur supports-[backdrop-filter]:bg-background/70"
      onDoubleClick={event => void handleTitlebarDoubleClick(event)}
    >
      <div className="flex items-center">
        <div className="flex items-center gap-3 rounded-md px-2 py-1">
          <img src={aacLogo} alt="AAC Logo" className="h-8 w-auto" />
        </div>
      </div>

      <div className="flex items-center justify-center px-4">
        <div className="flex items-center gap-2 rounded-full  px-3 py-1.5 shadow-sm">
          <ActivePageIcon className="h-4 w-4 text-primary" />
          <span className="text-sm font-medium">{activeDefinition.name}</span>
        </div>
      </div>

      <div className="no-drag flex items-center gap-1 justify-self-end">
        <button
          type="button"
          className={WINDOW_CONTROL_BUTTON}
          onClick={() => void Window.Minimise()}
          title="Minimise"
          aria-label="Minimise window"
        >
          <WindowControlIcon>
            <svg
              viewBox="0 0 16 16"
              className="h-3.5 w-3.5 fill-none stroke-current"
              aria-hidden="true"
            >
              <path d="M3 8.5H13" strokeWidth="1.5" strokeLinecap="round" />
            </svg>
          </WindowControlIcon>
        </button>
        <button
          type="button"
          className={WINDOW_CONTROL_BUTTON}
          onClick={() => void handleToggleMaximise()}
          title={isMaximised ? 'Restore' : 'Maximise'}
          aria-label={isMaximised ? 'Restore window' : 'Maximise window'}
        >
          <WindowControlIcon>
            {isMaximised ? (
              <svg
                viewBox="0 0 16 16"
                className="h-3.5 w-3.5 fill-none stroke-current"
                aria-hidden="true"
              >
                <path
                  d="M5 3.75H12.25V11"
                  strokeWidth="1.3"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                />
                <path d="M10.75 6H3.75V13H10.75V6Z" strokeWidth="1.3" strokeLinejoin="round" />
              </svg>
            ) : (
              <svg
                viewBox="0 0 16 16"
                className="h-3.5 w-3.5 fill-none stroke-current"
                aria-hidden="true"
              >
                <rect x="3.75" y="3.75" width="8.5" height="8.5" rx="0.5" strokeWidth="1.3" />
              </svg>
            )}
          </WindowControlIcon>
        </button>
        <button
          type="button"
          className={cn(
            WINDOW_CONTROL_BUTTON,
            'hover:bg-destructive hover:text-destructive-foreground'
          )}
          onClick={() => void Window.Close()}
          title="Close"
          aria-label="Close window"
        >
          <WindowControlIcon>
            <svg
              viewBox="0 0 16 16"
              className="h-3.5 w-3.5 fill-none stroke-current"
              aria-hidden="true"
            >
              <path d="M4 4L12 12" strokeWidth="1.5" strokeLinecap="round" />
              <path d="M12 4L4 12" strokeWidth="1.5" strokeLinecap="round" />
            </svg>
          </WindowControlIcon>
        </button>
      </div>
    </header>
  )
}
