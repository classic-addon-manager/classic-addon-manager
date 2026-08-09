import { ArrowUpIcon } from 'lucide-react'
import { type RefObject, useEffect, useState } from 'react'

import { Button } from '@/components/ui/button.tsx'
import { cn } from '@/lib/utils.ts'

const SHOW_AFTER_SCROLL = 240

interface ScrollToTopProps {
  scrollRef: RefObject<HTMLDivElement | null>
}

export const ScrollToTop = ({ scrollRef }: ScrollToTopProps) => {
  const [isVisible, setIsVisible] = useState(false)

  useEffect(() => {
    const container = scrollRef.current
    if (!container) return

    const handleScroll = () => setIsVisible(container.scrollTop > SHOW_AFTER_SCROLL)
    container.addEventListener('scroll', handleScroll, { passive: true })
    return () => container.removeEventListener('scroll', handleScroll)
  }, [scrollRef])

  return (
    <Button
      variant="outline"
      size="icon"
      aria-label="Scroll to top"
      tabIndex={isVisible ? 0 : -1}
      onClick={() => scrollRef.current?.scrollTo({ top: 0, behavior: 'smooth' })}
      className={cn(
        'absolute right-4 bottom-20 z-30 h-9 w-9 rounded-full bg-background/70 shadow-md backdrop-blur-md transition-all duration-200',
        isVisible ? 'opacity-100' : 'pointer-events-none translate-y-2 opacity-0'
      )}
    >
      <ArrowUpIcon className="h-4 w-4" />
    </Button>
  )
}
