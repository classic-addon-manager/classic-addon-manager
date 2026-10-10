import { useAtomValue } from 'jotai'

import { maxDownloadsAtom } from '@/components/addons/atoms'

const BAR_HEIGHTS = ['h-1', 'h-1.5', 'h-2', 'h-2.5', 'h-3']
const RING_CIRCUMFERENCE = 2 * Math.PI * 7.5

const compactNumber = new Intl.NumberFormat('en', {
  notation: 'compact',
  maximumFractionDigits: 1,
})

/** Maps a download count to 1-5 bars on a log scale relative to the most downloaded addon. */
const downloadLevel = (downloads: number, maxDownloads: number): number => {
  if (downloads <= 1 || maxDownloads <= 1) return 1
  const level = Math.round(1 + (4 * Math.log(downloads)) / Math.log(maxDownloads))
  return Math.min(BAR_HEIGHTS.length, Math.max(1, level))
}

export const DownloadMeter = ({ downloads }: { downloads: number }) => {
  const maxDownloads = useAtomValue(maxDownloadsAtom)
  const level = downloadLevel(downloads, maxDownloads)

  return (
    <div className="flex items-center justify-end gap-2 text-xs" title={`${downloads} downloads`}>
      <span className="font-mono tabular-nums text-foreground">
        {compactNumber.format(downloads)}
      </span>
      <span className="flex h-3 items-end gap-0.5" aria-hidden="true">
        {BAR_HEIGHTS.map((height, i) => (
          <span
            key={height}
            className={`w-[3px] rounded-[1px] ${height} ${i < level ? 'bg-muted-foreground' : 'bg-border'}`}
          />
        ))}
      </span>
    </div>
  )
}

const ringColor = (percentage: number): string => {
  if (percentage >= 90) return 'text-green-500'
  if (percentage >= 70) return 'text-primary'
  return 'text-amber-500'
}

interface LikeRingProps {
  percentage?: number | null
  /** Reserve three digits so the rings line up down a column. */
  fixedWidth?: boolean
}

export const LikeRing = ({ percentage, fixedWidth = true }: LikeRingProps) => {
  if (percentage === null || percentage === undefined) {
    return (
      <div className="text-right font-mono text-xs text-muted-foreground/60" title="No ratings yet">
        —
      </div>
    )
  }

  return (
    <div className="flex items-center justify-end gap-1.5 text-xs" title={`${percentage}% likes`}>
      <svg viewBox="0 0 20 20" className="h-[18px] w-[18px] -rotate-90" aria-hidden="true">
        <circle cx="10" cy="10" r="7.5" fill="none" strokeWidth="2.5" className="stroke-border" />
        <circle
          cx="10"
          cy="10"
          r="7.5"
          fill="none"
          stroke="currentColor"
          strokeWidth="2.5"
          strokeLinecap="round"
          strokeDasharray={`${(RING_CIRCUMFERENCE * percentage) / 100} ${RING_CIRCUMFERENCE}`}
          className={ringColor(percentage)}
        />
      </svg>
      <span
        className={`font-mono tabular-nums text-foreground ${fixedWidth ? 'w-[3ch] text-left' : ''}`}
      >
        {percentage}
      </span>
    </div>
  )
}
