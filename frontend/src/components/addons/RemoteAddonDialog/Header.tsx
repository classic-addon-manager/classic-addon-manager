import { Browser } from '@wailsio/runtime'
import { BugIcon, CalendarDaysIcon, GithubIcon, TagIcon, UserIcon } from 'lucide-react'
import { useState } from 'react'

import { Badge } from '@/components/ui/badge.tsx'
import { Button } from '@/components/ui/button.tsx'
import { DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { formatToLocalTime } from '@/lib/utils.ts'
import type { AddonManifest } from '@/lib/wails'
import type { Release } from '@/lib/wails'

interface HeaderProps {
  manifest: AddonManifest
  release: Release | null
}

const BannerImage = ({
  bannerUrl,
  hasBanner,
  setHasBanner,
  alias,
}: {
  bannerUrl: string | null
  hasBanner: boolean
  setHasBanner: (hasBanner: boolean) => void
  alias: string
}) => {
  if (bannerUrl && hasBanner) {
    return (
      <img
        className="absolute top-0 left-0 w-full h-full object-cover"
        src={bannerUrl}
        alt={`${alias} Banner`}
        onError={() => setHasBanner(false)}
      />
    )
  }
  return null
}

const ReleaseInformation = ({ release }: { release: Release | null }) => {
  if (!release) {
    return <Badge variant="outline">No Release</Badge>
  }

  return (
    <span className="inline-flex items-center gap-x-1.5">
      <Badge variant="secondary" className="inline-flex items-center gap-1">
        <TagIcon className="w-3 h-3" />
        {release.tag_name}
      </Badge>
      <span className="inline-flex items-center gap-1 text-xs text-muted-foreground">
        <CalendarDaysIcon className="w-3 h-3" />
        {formatToLocalTime(release.published_at)}
      </span>
    </span>
  )
}

export const Header = ({ manifest, release }: HeaderProps) => {
  const [hasBanner, setHasBanner] = useState(true)
  const bannerUrl =
    'repo' in manifest && 'branch' in manifest
      ? `https://raw.githubusercontent.com/${manifest.repo}/${manifest.branch}/banner.png`
      : null

  if (!hasBanner) {
    return (
      <DialogHeader className="p-6 pb-4 shrink-0 border-b">
        <DialogTitle className="text-2xl font-semibold mb-1">{manifest.alias}</DialogTitle>
        <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm mb-3 text-muted-foreground">
          <span className="inline-flex items-center gap-1">
            <UserIcon className="w-3.5 h-3.5" />
            <span className="font-normal text-foreground/90">{manifest.author}</span>
          </span>
          <ReleaseInformation release={release} />
        </div>
        <div className="flex flex-wrap gap-2 text-sm">
          <Button
            variant="outline"
            size="sm"
            className="flex items-center gap-1"
            onClick={() => Browser.OpenURL(`https://github.com/${manifest.repo}`)}
          >
            <GithubIcon className="w-4 h-4" />
            View Code
          </Button>
          <Button
            variant="outline"
            size="sm"
            className="flex items-center gap-1"
            onClick={() => Browser.OpenURL(`https://github.com/${manifest.repo}/issues/new`)}
          >
            <BugIcon className="w-4 h-4" />
            Report Issue
          </Button>
        </div>
      </DialogHeader>
    )
  }

  return (
    <div className="relative w-full h-48 overflow-hidden shrink-0 rounded-t-lg border-b dark:border-white/10">
      <BannerImage
        bannerUrl={bannerUrl}
        hasBanner={hasBanner}
        setHasBanner={setHasBanner}
        alias={manifest.alias}
      />
      <div className="absolute inset-0 bg-gradient-to-t from-black/75 via-black/50 to-black/5"></div>
      <div className="absolute bottom-0 left-0 right-0 p-6 pb-4 pt-16 text-white bg-gradient-to-t from-black/60 to-transparent">
        <DialogTitle className="text-2xl font-semibold mb-1">{manifest.alias}</DialogTitle>
        <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm mb-3 text-white/80">
          <span className="inline-flex items-center gap-1">
            <UserIcon className="w-3.5 h-3.5" />
            <span className="font-normal text-white/90">{manifest.author}</span>
          </span>
          <ReleaseInformation release={release} />
        </div>
        <div className="flex flex-wrap gap-x-2 gap-y-2 text-sm">
          <Button
            variant="outline"
            size="sm"
            className="flex items-center gap-1 border-white/50 text-white hover:bg-white/10"
            onClick={() => Browser.OpenURL(`https://github.com/${manifest.repo}`)}
          >
            <GithubIcon className="w-4 h-4" />
            View Code
          </Button>
          <Button
            variant="outline"
            size="sm"
            className="flex items-center gap-1 border-white/50 text-white hover:bg-white/10"
            onClick={() => Browser.OpenURL(`https://github.com/${manifest.repo}/issues/new`)}
          >
            <BugIcon className="w-4 h-4" />
            Report Issue
          </Button>
        </div>
      </div>
    </div>
  )
}
