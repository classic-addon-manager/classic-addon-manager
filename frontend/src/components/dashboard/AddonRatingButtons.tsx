import { clsx } from 'clsx'
import { AnimatePresence, motion } from 'framer-motion'
import { ThumbsDownIcon, ThumbsUpIcon } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'

import { Button } from '@/components/ui/button'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { getMyRating, rateAddon } from '@/lib/addon'
import { useUserStore } from '@/stores/userStore'

const HAPPY_EMOJIS = ['🎉', '✨', '🥳', '😊', '💖']
const SAD_EMOJIS = ['😢', '💔', '😞', '🥺', '😭']

interface RatingBurst {
  key: number
  type: 'happy' | 'sad'
  particles: Array<{ id: number; emoji: string; x: number; y: number; rotate: number }>
}

const RatingEmojiBurst = ({ particles }: { particles: RatingBurst['particles'] }) => {
  const [visible, setVisible] = useState(true)

  useEffect(() => {
    const timer = setTimeout(() => setVisible(false), 2000)
    return () => clearTimeout(timer)
  }, [])

  if (!visible) return null

  return (
    <div className="absolute inset-0 pointer-events-none overflow-visible z-50" aria-hidden>
      <AnimatePresence>
        {particles.map(p => (
          <motion.span
            key={p.id}
            className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 text-lg select-none"
            initial={{ opacity: 1, scale: 0, rotate: 0, x: 0, y: 0 }}
            animate={{
              opacity: [1, 1, 1, 0],
              x: p.x,
              y: p.y,
              scale: 1.3,
              rotate: p.rotate,
            }}
            transition={{ duration: 1.6, ease: 'easeOut' }}
          >
            {p.emoji}
          </motion.span>
        ))}
      </AnimatePresence>
    </div>
  )
}

interface AddonRatingButtonsProps {
  addonName: string
  addonAlias: string
  isManaged: boolean
}

export const AddonRatingButtons = (props: AddonRatingButtonsProps) => (
  <AddonRatingButtonsContent key={props.addonName} {...props} />
)

const AddonRatingButtonsContent = ({
  addonName,
  addonAlias,
  isManaged,
}: AddonRatingButtonsProps) => {
  const { isAuthenticated } = useUserStore()
  const authenticated = isAuthenticated()
  const [rating, setRating] = useState(0)
  const [burst, setBurst] = useState<RatingBurst | null>(null)
  const nextBurstKey = useRef(0)

  useEffect(() => {
    let active = true
    getMyRating(addonName, authenticated, value => {
      if (active) setRating(value)
    }).catch(e => console.error('Failed to fetch rating: ', e))
    return () => {
      active = false
    }
  }, [addonName, authenticated])

  if (!isManaged) return null

  const handleRateAddon = async (newRating: number) => {
    const key = ++nextBurstKey.current
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
    setBurst({ key, type, particles })
    await rateAddon(addonName, addonAlias, newRating, rating, setRating)
  }

  if (!authenticated) {
    return (
      <Tooltip>
        <TooltipTrigger asChild>
          <span
            className="flex items-center gap-3 p-2 cursor-not-allowed opacity-50"
            aria-label="Log in to rate addons"
          >
            <ThumbsUpIcon className="w-4 h-4 text-muted-foreground" />
            <ThumbsDownIcon className="w-4 h-4 text-muted-foreground" />
          </span>
        </TooltipTrigger>
        <TooltipContent className="font-bold">Sign in using Discord to rate addons</TooltipContent>
      </Tooltip>
    )
  }

  return (
    <div className="flex gap-1 items-center relative">
      <motion.div
        className="relative"
        whileTap={{ scale: 1.4, rotate: -20 }}
        transition={{ type: 'spring', stiffness: 400, damping: 10 }}
      >
        <Button
          variant="ghost"
          size="icon"
          className={clsx(
            'h-7 w-7 transition-colors duration-200 hover:bg-blue-100 dark:hover:bg-blue-900/30',
            rating === 1 && 'bg-blue-100 dark:bg-blue-900/30 border border-blue-500 text-blue-500'
          )}
          onClick={() => handleRateAddon(1)}
          aria-label="Like addon"
        >
          <ThumbsUpIcon
            className={clsx('w-4 h-4', {
              'text-blue-500': rating === 1,
              'text-muted-foreground': rating !== 1,
            })}
          />
        </Button>
        {burst?.type === 'happy' && (
          <RatingEmojiBurst key={burst.key} particles={burst.particles} />
        )}
      </motion.div>
      <motion.div
        className="relative"
        whileTap={{ scale: 0.8, rotate: 15 }}
        transition={{ type: 'spring', stiffness: 400, damping: 10 }}
      >
        <Button
          variant="ghost"
          size="icon"
          className={clsx(
            'h-7 w-7 transition-colors duration-200 hover:bg-red-100 dark:hover:bg-red-900/30',
            rating === -1 && 'bg-red-100 dark:bg-red-900/30 border border-red-500 text-red-500'
          )}
          onClick={() => handleRateAddon(-1)}
          aria-label="Dislike addon"
        >
          <ThumbsDownIcon
            className={clsx('w-4 h-4', {
              'text-red-500': rating === -1,
              'text-muted-foreground': rating !== -1,
            })}
          />
        </Button>
        {burst?.type === 'sad' && <RatingEmojiBurst key={burst.key} particles={burst.particles} />}
      </motion.div>
    </div>
  )
}
