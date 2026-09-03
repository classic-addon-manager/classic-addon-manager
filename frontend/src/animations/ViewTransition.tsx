import { AnimatePresence, motion } from 'framer-motion'
import type { ReactNode } from 'react'

import { fade, VIEW_FADE_DURATION } from '@/animations/transitions'

export const ViewTransition = ({
  view,
  className,
  duration = VIEW_FADE_DURATION,
  children,
}: {
  view: string
  className?: string
  duration?: number
  children: ReactNode
}) => (
  <AnimatePresence mode="wait">
    <motion.div key={view} className={className} {...fade(duration)}>
      {children}
    </motion.div>
  </AnimatePresence>
)
