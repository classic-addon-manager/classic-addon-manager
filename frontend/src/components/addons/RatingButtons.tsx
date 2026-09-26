import { AnimatePresence, motion } from 'framer-motion'
import { ThumbsDownIcon, ThumbsUpIcon } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'

import { Button } from '@/components/ui/button'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { cn } from '@/lib/utils'
import { useUserStore } from '@/stores/userStore'

const HAPPY_EMOJIS = ['🎉', '✨', '🥳', '😊', '💖']
const SAD_EMOJIS = ['😢', '💔', '😞', '🥺', '😭']

type BurstParticle = { id: number; emoji: string; x: number; y: number; rotate: number }

interface RatingBurst {
  key: number
  type: 'happy' | 'sad'
  particles: BurstParticle[]
  origin: { x: number; y: number }
}

const SIZES = {
  sm: { button: 'h-7 w-7', icon: 'h-4 w-4', gap: 'gap-3' },
  md: { button: 'h-8 w-9', icon: 'h-5 w-5', gap: 'gap-1' },
}

/* Drawn on the page itself rather than inside the buttons, so dialogs and scroll areas cannot cut it off. */
const RatingEmojiBurst = ({ particles, origin }: Pick<RatingBurst, 'particles' | 'origin'>) => {
  const [visible, setVisible] = useState(true)

  useEffect(() => {
    const timer = setTimeout(() => setVisible(false), 2000)
    return () => clearTimeout(timer)
  }, [])

  if (!visible) return null

  return createPortal(
    <div
      className="fixed pointer-events-none z-100"
      style={{ left: origin.x, top: origin.y }}
      aria-hidden
    >
      <AnimatePresence>
        {particles.map(p => (
          <motion.span
            key={p.id}
            className="absolute left-0 top-0 -translate-x-1/2 -translate-y-1/2 text-lg select-none"
            initial={{ opacity: 1, scale: 0, rotate: 0, x: 0, y: 0 }}
            animate={{ opacity: [1, 1, 1, 0], x: p.x, y: p.y, scale: 1.3, rotate: p.rotate }}
            transition={{ duration: 1.6, ease: 'easeOut' }}
          >
            {p.emoji}
          </motion.span>
        ))}
      </AnimatePresence>
    </div>,
    document.body
  )
}

const createBurst = (key: number, newRating: number, button: HTMLElement): RatingBurst => {
  const rect = button.getBoundingClientRect()
  const origin = { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 }
  const type = newRating === 1 ? 'happy' : 'sad'
  const emojis = type === 'happy' ? HAPPY_EMOJIS : SAD_EMOJIS
  const particles = emojis.map((emoji, i) => {
    const angle = (i / (emojis.length - 1) - 0.5) * Math.PI * 0.8
    const dist = 50 + Math.random() * 40
    return {
      id: key * 100 + i,
      emoji,
      x: Math.sin(angle) * dist,
      y: -Math.cos(angle) * dist,
      rotate: (Math.random() - 0.5) * 100,
    }
  })
  return { key, type, particles, origin }
}

export interface RatingButtonsProps {
  rating: number
  /* Resolve to true when the vote was saved; the emoji burst only plays after a successful save. */
  onRate: (rating: number) => Promise<boolean> | boolean | void
  disabled?: boolean
  size?: keyof typeof SIZES
}

export const RatingButtons = ({
  rating,
  onRate,
  disabled = false,
  size = 'md',
}: RatingButtonsProps) => {
  const { isAuthenticated } = useUserStore()
  const [burst, setBurst] = useState<RatingBurst | null>(null)
  const nextBurstKey = useRef(0)
  const sizes = SIZES[size]

  if (!isAuthenticated()) {
    return (
      <Tooltip delayDuration={100}>
        <TooltipTrigger asChild>
          <span
            className={cn('flex items-center p-2 cursor-not-allowed opacity-50', sizes.gap)}
            aria-label="Log in to rate addons"
          >
            <ThumbsUpIcon className={cn(sizes.icon, 'text-muted-foreground')} />
            <ThumbsDownIcon className={cn(sizes.icon, 'text-muted-foreground')} />
          </span>
        </TooltipTrigger>
        <TooltipContent>
          <p>Sign in using Discord to rate addons</p>
        </TooltipContent>
      </Tooltip>
    )
  }

  const handleRate = async (newRating: number, button: HTMLElement) => {
    const saved = await onRate(newRating)
    if (saved) setBurst(createBurst(++nextBurstKey.current, newRating, button))
  }

  const options = [
    {
      value: 1,
      type: 'happy',
      Icon: ThumbsUpIcon,
      label: 'Like addon',
      tooltip: rating === 1 ? 'Liked' : 'Like',
      tap: { scale: 1.4, rotate: -20 },
      hover: 'hover:bg-primary/10',
      active: 'bg-primary/10 ring-1 ring-inset ring-primary/40',
      activeIcon: 'text-primary',
    },
    {
      value: -1,
      type: 'sad',
      Icon: ThumbsDownIcon,
      label: 'Dislike addon',
      tooltip: rating === -1 ? 'Disliked' : 'Dislike',
      tap: { scale: 0.8, rotate: 15 },
      hover: 'hover:bg-destructive/10',
      active: 'bg-destructive/10 ring-1 ring-inset ring-destructive/40',
      activeIcon: 'text-destructive',
    },
  ] as const

  return (
    <>
      <div className="flex gap-1 items-center relative">
        {options.map(option => {
          const selected = rating === option.value
          return (
            <motion.div
              key={option.value}
              className="relative"
              whileTap={option.tap}
              transition={{ type: 'spring', stiffness: 400, damping: 10 }}
            >
              <Tooltip delayDuration={100}>
                <TooltipTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon"
                    className={cn(
                      sizes.button,
                      'transition-all duration-200 hover:scale-105',
                      option.hover,
                      selected && option.active
                    )}
                    onClick={event => handleRate(option.value, event.currentTarget)}
                    aria-label={option.label}
                    disabled={disabled}
                  >
                    <option.Icon
                      className={cn(
                        sizes.icon,
                        selected ? option.activeIcon : 'text-muted-foreground'
                      )}
                    />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>
                  <p>{option.tooltip}</p>
                </TooltipContent>
              </Tooltip>
            </motion.div>
          )
        })}
      </div>
      {burst && (
        <RatingEmojiBurst key={burst.key} particles={burst.particles} origin={burst.origin} />
      )}
    </>
  )
}
