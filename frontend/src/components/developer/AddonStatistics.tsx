import { BarChart3 } from 'lucide-react'
import { useState } from 'react'

import { backendUnavailable } from '@/components/developer/catalogEditing'
import { mockDownloadTrends } from '@/components/developer/developerMocks'
import type { OwnedAddon } from '@/components/developer/ownedParse'
import { Button } from '@/components/ui/button'

export function AddonStatistics({ addon }: { addon: OwnedAddon }) {
  const [period, setPeriod] = useState<'7' | '30'>('7')
  const points = mockDownloadTrends[period]
  const maximum = Math.max(...points)
  return (
    <section className="space-y-6">
      <div className="flex flex-wrap gap-8 border-b pb-5">
        <div>
          <p className="text-xs text-muted-foreground">Total downloads</p>
          <p className="mt-1 text-2xl tabular-nums">{addon.downloads.toLocaleString()}</p>
        </div>
        <div>
          <p className="text-xs text-muted-foreground">Likes</p>
          <p className="mt-1 text-2xl tabular-nums">
            {addon.likePercentage === null ? 'Not available' : `${addon.likePercentage}%`}
          </p>
        </div>
      </div>
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h3 className="flex items-center gap-2 text-sm font-medium">
          <BarChart3 className="size-4" />
          Download trend
        </h3>
        <div className="flex gap-1">
          {(['7', '30'] as const).map(value => (
            <Button
              key={value}
              size="sm"
              variant={period === value ? 'secondary' : 'ghost'}
              aria-pressed={period === value}
              onClick={() => {
                setPeriod(value)
                backendUnavailable('Download history')
              }}
            >
              {value} days
            </Button>
          ))}
        </div>
      </div>
      <p className="text-xs text-muted-foreground">
        Sample trend data. Totals above are from your published addon.
      </p>
      <div
        className="flex h-40 items-end gap-2 border-b"
        role="img"
        aria-label={`Sample download counts: ${points.join(', ')}`}
      >
        {points.map((point, index) => (
          <div
            key={index}
            className="min-w-0 flex-1 rounded-t bg-primary/80"
            style={{ height: `${(point / maximum) * 100}%` }}
            title={`${point} sample downloads`}
          />
        ))}
      </div>
      <div className="flex justify-between text-xs text-muted-foreground">
        <span>{period} days ago</span>
        <span>Today</span>
      </div>
    </section>
  )
}
