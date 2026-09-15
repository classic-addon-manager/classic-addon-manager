import { BarChart3, LoaderCircle } from 'lucide-react'
import { useState } from 'react'
import {
  Area,
  AreaChart,
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  ReferenceLine,
  XAxis,
  YAxis,
} from 'recharts'

import type { OwnedAddon } from '@/components/developer/ownedParse'
import type {
  AddonRatingStats,
  AddonVersionStats,
  SnapshotPoint,
  VersionLagBucket,
} from '@/components/developer/types.ts'
import { useDevAddonStats } from '@/components/developer/useDevAddonStats.ts'
import { Button } from '@/components/ui/button'
import {
  type ChartConfig,
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
} from '@/components/ui/chart'

const LAG_LABELS: Record<VersionLagBucket, string> = {
  latest: 'Latest',
  one_behind: '1 behind',
  two_behind: '2+ behind',
  unavailable: 'Unavailable',
}

const LAG_COLOR: Record<VersionLagBucket, string> = {
  latest: 'var(--primary)',
  one_behind: 'color-mix(in oklch, var(--primary) 60%, transparent)',
  two_behind: 'color-mix(in oklch, var(--primary) 35%, transparent)',
  unavailable: 'color-mix(in oklch, var(--destructive) 70%, transparent)',
}

const LAG_ORDER: VersionLagBucket[] = ['latest', 'one_behind', 'two_behind', 'unavailable']

const downloadConfig = {
  dailyDownloads: {
    label: 'Downloads',
    color: 'var(--primary)',
  },
} satisfies ChartConfig

const subscriberConfig = {
  subscribers: {
    label: 'Subscribers',
    color: 'var(--primary)',
  },
} satisfies ChartConfig

const changeConfig = {
  dailySubscriberChange: {
    label: 'Change',
    color: 'var(--primary)',
  },
} satisfies ChartConfig

const versionConfig = {
  subscribers: {
    label: 'Subscribers',
  },
  latest: {
    label: LAG_LABELS.latest,
    color: LAG_COLOR.latest,
  },
  one_behind: {
    label: LAG_LABELS.one_behind,
    color: LAG_COLOR.one_behind,
  },
  two_behind: {
    label: LAG_LABELS.two_behind,
    color: LAG_COLOR.two_behind,
  },
  unavailable: {
    label: LAG_LABELS.unavailable,
    color: LAG_COLOR.unavailable,
  },
} satisfies ChartConfig

type ChartPoint = Pick<SnapshotPoint, 'takenOn' | 'dailyDownloads' | 'dailySubscriberChange'> & {
  subscribers: number | null
}

export function AddonStatistics({ addon }: { addon: OwnedAddon }) {
  const [period, setPeriod] = useState<7 | 30>(7)
  const [reloadKey, setReloadKey] = useState(0)
  const loaded = useDevAddonStats(addon.uuid, reloadKey)

  if (loaded.loading) {
    return (
      <div className="flex flex-col items-center justify-center py-16 text-muted-foreground">
        <LoaderCircle className="size-8 animate-spin opacity-50" strokeWidth={1.5} />
        <p className="mt-3 text-sm">Loading statistics...</p>
      </div>
    )
  }

  if (!loaded.stats) {
    return (
      <div className="flex flex-col items-center justify-center gap-3 py-16">
        <p className="text-center text-sm text-destructive">
          {loaded.error ?? 'These statistics could not be loaded.'}
        </p>
        <Button variant="outline" onClick={() => setReloadKey(key => key + 1)}>
          Retry
        </Button>
      </div>
    )
  }

  const { downloads, ratings, subscribers, versions } = loaded.stats
  const downloadSeries = seriesInWindow(downloads.series, period)
  const subscriberSeries = seriesInWindow(subscribers.series, period)
  const likeLabel =
    ratings.likePercentage === null ? 'No votes' : formatRatio(ratings.likePercentage)

  return (
    <section className="space-y-8">
      <div className="flex flex-wrap gap-8 border-b pb-5">
        <Metric label="Total downloads" value={downloads.total.toLocaleString()} />
        <Metric label="Subscribers" value={subscribers.current.toLocaleString()} />
        <Metric label="Liked" value={likeLabel} />
      </div>

      <div className="space-y-3">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <h3 className="flex items-center gap-2 text-sm font-medium">
            <BarChart3 className="size-4" />
            Download trend
          </h3>
          <div className="flex gap-1">
            {([7, 30] as const).map(days => (
              <Button
                key={days}
                size="sm"
                variant={period === days ? 'secondary' : 'ghost'}
                aria-pressed={period === days}
                onClick={() => setPeriod(days)}
              >
                {days} days
              </Button>
            ))}
          </div>
        </div>
        <DailyDownloadChart points={downloadSeries} name={addon.alias} />
        <p className="text-xs text-muted-foreground">
          Daily downloads. A missing adjacent snapshot is a gap, not a zero. The current UTC day
          updates hourly and is partial.
        </p>
      </div>

      <div className="space-y-3">
        <h3 className="text-sm font-medium">Subscribers</h3>
        <SubscriberChart points={subscriberSeries} name={addon.alias} />
        <SignedChangeChart points={subscriberSeries} name={addon.alias} />
        <p className="text-xs text-muted-foreground">
          Active subscribers and signed net change between adjacent days. Negative bars are churn.
        </p>
      </div>

      <RatingsPanel ratings={ratings} />
      <VersionPanel versions={versions} />
    </section>
  )
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-xs text-muted-foreground">{label}</p>
      <p className="mt-1 text-2xl tabular-nums">{value}</p>
    </div>
  )
}

function DailyDownloadChart({ points, name }: { points: ChartPoint[]; name: string }) {
  const values = points.flatMap(point =>
    point.dailyDownloads === null ? [] : [point.dailyDownloads]
  )
  if (points.length === 0) {
    return <p className="text-sm text-muted-foreground">No snapshots in this period.</p>
  }
  if (values.length === 0) {
    return <p className="text-sm text-muted-foreground">No daily download deltas in this period.</p>
  }
  return (
    <ChartContainer
      config={downloadConfig}
      className="aspect-auto h-40 w-full"
      role="img"
      aria-label={`Daily downloads for ${name}: ${values.join(', ')}`}
    >
      <BarChart accessibilityLayer data={points} margin={{ left: 8, right: 8 }}>
        <CartesianGrid vertical={false} />
        <XAxis
          dataKey="takenOn"
          tickLine={false}
          axisLine={false}
          tickMargin={8}
          minTickGap={24}
          tickFormatter={formatSnapshotDay}
        />
        <ChartTooltip
          content={<ChartTooltipContent labelFormatter={formatTooltipDay} indicator="line" />}
        />
        <Bar dataKey="dailyDownloads" fill="var(--color-dailyDownloads)" radius={2} />
      </BarChart>
    </ChartContainer>
  )
}

function SubscriberChart({ points, name }: { points: ChartPoint[]; name: string }) {
  if (points.length === 0) {
    return <p className="text-sm text-muted-foreground">No snapshots in this period.</p>
  }
  const values = points.flatMap(point => (point.subscribers === null ? [] : [point.subscribers]))
  return (
    <ChartContainer
      config={subscriberConfig}
      className="aspect-auto h-32 w-full"
      role="img"
      aria-label={`Subscriber counts for ${name}: ${values.join(', ')}`}
    >
      <AreaChart accessibilityLayer data={points} margin={{ left: 8, right: 8, top: 8 }}>
        <CartesianGrid vertical={false} />
        <XAxis
          dataKey="takenOn"
          tickLine={false}
          axisLine={false}
          tickMargin={8}
          minTickGap={24}
          tickFormatter={formatSnapshotDay}
        />
        <ChartTooltip
          cursor={false}
          content={<ChartTooltipContent indicator="dot" labelFormatter={formatTooltipDay} />}
        />
        <Area
          dataKey="subscribers"
          type="linear"
          fill="var(--color-subscribers)"
          fillOpacity={0.35}
          stroke="var(--color-subscribers)"
          strokeWidth={2}
          dot={{ r: 2, fill: 'var(--color-subscribers)', strokeWidth: 0 }}
          activeDot={{ r: 5, strokeWidth: 2, stroke: 'var(--background)' }}
        />
      </AreaChart>
    </ChartContainer>
  )
}

function SignedChangeChart({ points, name }: { points: ChartPoint[]; name: string }) {
  const values = points.flatMap(point =>
    point.dailySubscriberChange === null ? [] : [point.dailySubscriberChange]
  )
  if (values.length === 0) return null
  return (
    <ChartContainer
      config={changeConfig}
      className="aspect-auto h-24 w-full"
      role="img"
      aria-label={`Daily subscriber change for ${name}: ${values.join(', ')}`}
    >
      <BarChart accessibilityLayer data={points} margin={{ left: 8, right: 8 }}>
        <CartesianGrid vertical={false} />
        <XAxis
          dataKey="takenOn"
          tickLine={false}
          axisLine={false}
          tickMargin={8}
          minTickGap={24}
          tickFormatter={formatSnapshotDay}
        />
        <ReferenceLine y={0} stroke="var(--border)" />
        <ChartTooltip
          cursor={false}
          content={<ChartTooltipContent hideIndicator labelFormatter={formatTooltipDay} />}
        />
        <Bar dataKey="dailySubscriberChange" radius={2}>
          {points.map(point => (
            <Cell
              key={point.takenOn}
              fill={
                point.dailySubscriberChange !== null && point.dailySubscriberChange < 0
                  ? 'var(--destructive)'
                  : 'var(--primary)'
              }
            />
          ))}
        </Bar>
      </BarChart>
    </ChartContainer>
  )
}

function RatingsPanel({ ratings }: { ratings: AddonRatingStats }) {
  const total = ratings.totalVotes
  return (
    <div className="space-y-3">
      <h3 className="text-sm font-medium">Ratings</h3>
      {total === 0 ? (
        <p className="text-sm text-muted-foreground">No ratings yet.</p>
      ) : (
        <>
          <div
            className="flex h-2 overflow-hidden rounded-full bg-muted"
            role="img"
            aria-label={`${ratings.likes} likes, ${ratings.dislikes} dislikes`}
          >
            <div className="bg-primary" style={{ width: `${(ratings.likes / total) * 100}%` }} />
            <div
              className="bg-muted-foreground/35"
              style={{ width: `${(ratings.dislikes / total) * 100}%` }}
            />
          </div>
          <p className="text-sm text-muted-foreground">
            {ratings.likes.toLocaleString()} like{ratings.likes === 1 ? '' : 's'} ·{' '}
            {ratings.dislikes.toLocaleString()} dislike{ratings.dislikes === 1 ? '' : 's'} ·{' '}
            {formatRatio(ratings.likePercentage ?? 0)}
          </p>
          {ratings.deletedAccounts > 0 && (
            <p className="text-xs text-muted-foreground">
              Includes {ratings.deletedAccounts.toLocaleString()} vote
              {ratings.deletedAccounts === 1 ? '' : 's'} from deleted accounts.
            </p>
          )}
        </>
      )}
    </div>
  )
}

function VersionPanel({ versions }: { versions: AddonVersionStats }) {
  const bucketTotal = LAG_ORDER.reduce((sum, key) => sum + versions.buckets[key], 0)
  const data = versions.share.map(row => ({
    ...row,
    fill: `var(--color-${row.lagBucket})`,
  }))

  return (
    <div className="space-y-3">
      <h3 className="text-sm font-medium">Version adoption</h3>
      {bucketTotal === 0 ? (
        <p className="text-sm text-muted-foreground">No subscribers yet.</p>
      ) : (
        <>
          <div className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
            {LAG_ORDER.map(key => (
              <span key={key} className="inline-flex items-center gap-1.5">
                <span className="size-2 rounded-[2px]" style={{ background: LAG_COLOR[key] }} />
                {LAG_LABELS[key]}{' '}
                <span className="tabular-nums text-foreground">
                  {versions.buckets[key].toLocaleString()}
                </span>
              </span>
            ))}
          </div>
          <ChartContainer
            config={versionConfig}
            className="aspect-auto w-full"
            style={{ height: Math.max(80, data.length * 36) }}
            role="img"
            aria-label={data.map(row => `${row.tagName}: ${row.subscribers}`).join(', ')}
          >
            <BarChart
              accessibilityLayer
              data={data}
              layout="vertical"
              margin={{ left: 0, right: 8, top: 4, bottom: 4 }}
            >
              <XAxis type="number" dataKey="subscribers" hide />
              <YAxis
                dataKey="tagName"
                type="category"
                tickLine={false}
                axisLine={false}
                tickMargin={8}
                width={88}
              />
              <ChartTooltip
                cursor={false}
                content={
                  <ChartTooltipContent
                    nameKey="lagBucket"
                    labelFormatter={(_value, payload) => {
                      const row = payload?.[0]?.payload as (typeof data)[number] | undefined
                      if (!row) return ''
                      return `${row.tagName} · ${formatRatio(row.share)}`
                    }}
                  />
                }
              />
              <Bar dataKey="subscribers" radius={4} />
            </BarChart>
          </ChartContainer>
        </>
      )}
    </div>
  )
}

function seriesInWindow(series: SnapshotPoint[], days: number): ChartPoint[] {
  const date = new Date()
  date.setUTCDate(date.getUTCDate() + 1 - days)
  const byDay = new Map(series.map(point => [point.takenOn, point]))
  // Keep missing dates on the axis without inventing counts or connecting the area across gaps.
  const points = Array.from({ length: days }, () => {
    const takenOn = date.toISOString().slice(0, 10)
    date.setUTCDate(date.getUTCDate() + 1)
    return (
      byDay.get(takenOn) ?? {
        takenOn,
        dailyDownloads: null,
        subscribers: null,
        dailySubscriberChange: null,
      }
    )
  })
  return points.some(point => point.subscribers !== null) ? points : []
}

function formatSnapshotDay(takenOn: string): string {
  return new Date(`${takenOn}T00:00:00Z`).toLocaleDateString(undefined, {
    month: 'short',
    day: 'numeric',
    timeZone: 'UTC',
  })
}

function formatTooltipDay(value: unknown): string {
  return typeof value === 'string' ? formatSnapshotDay(value) : String(value ?? '')
}

function formatRatio(value: number): string {
  return new Intl.NumberFormat(undefined, {
    style: 'percent',
    maximumFractionDigits: 1,
  }).format(value)
}
