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

const RatingEmojiBurst = ({ burstKey, type }: { burstKey: number; type: 'happy' | 'sad' }) => {
  const lastProcessedKey = useRef(0)
  const [particles, setParticles] = useState<
    Array<{ id: number; emoji: string; x: number; y: number; rotate: number }>
  >([])

  useEffect(() => {
    if (burstKey === 0 || burstKey === lastProcessedKey.current) return
    lastProcessedKey.current = burstKey

    const isHappy = type === 'happy'
    const emojis = isHappy ? HAPPY_EMOJIS : SAD_EMOJIS
    const count = emojis.length

    const newParticles = Array.from({ length: count }, (_, i) => {
      const angle = (i / (count - 1) - 0.5) * Math.PI * 0.8
      const dist = 50 + Math.random() * 40
      return {
        id: burstKey * 100 + i,
        emoji: emojis[i % emojis.length],
        x: Math.sin(angle) * dist,
        y: -Math.cos(angle) * dist,
        rotate: (Math.random() - 0.5) * 100,
      }
    })

    setParticles(newParticles)
    const timer = setTimeout(() => setParticles([]), 2000)
    return () => clearTimeout(timer)
  }, [burstKey, type])

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

export const AddonRatingButtons = ({
  addonName,
  addonAlias,
  isManaged,
}: AddonRatingButtonsProps) => {
  const { isAuthenticated } = useUserStore()
  const [rating, setRating] = useState(0)
  const [burstType, setBurstType] = useState<'happy' | 'sad' | null>(null)
  const [burstKey, setBurstKey] = useState(0)

  useEffect(() => {
    setBurstType(null)
    setBurstKey(0)
  }, [addonName])

  useEffect(() => {
    getMyRating(addonName, isAuthenticated(), setRating).catch(e =>
      console.error('Failed to fetch rating: ', e)
    )
  }, [addonName, isAuthenticated])

  if (!isManaged) return null

  const handleRateAddon = async (newRating: number) => {
    setBurstType(newRating === 1 ? 'happy' : 'sad')
    setBurstKey(k => k + 1)
    await rateAddon(addonName, addonAlias, newRating, rating, setRating)
  }

  if (!isAuthenticated()) {
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
        {burstType === 'happy' && <RatingEmojiBurst burstKey={burstKey} type="happy" />}
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
        {burstType === 'sad' && <RatingEmojiBurst burstKey={burstKey} type="sad" />}
      </motion.div>
    </div>
  )
}
