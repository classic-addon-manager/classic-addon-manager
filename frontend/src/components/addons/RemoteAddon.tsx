import { useSetAtom } from 'jotai'
import { BlocksIcon, CheckIcon, DownloadIcon, HeartIcon } from 'lucide-react'
import { useState } from 'react'

import { isManifestDialogOpenAtom, selectedManifestAtom } from '@/components/addons/atoms.ts'
import { daysAgo } from '@/lib/utils.ts'
import type { AddonManifest } from '@/lib/wails'

import { Button } from '../ui/button'

interface RemoteAddonProps {
  manifest: AddonManifest
  installed: boolean
}

export const RemoteAddon = ({ manifest, installed }: RemoteAddonProps) => {
  const [hasIcon, setIcon] = useState(true)
  const setSelectedManifest = useSetAtom(selectedManifestAtom)
  const setDialogOpen = useSetAtom(isManifestDialogOpenAtom)

  const isNew = daysAgo(manifest.added_at) < 32
  const iconUrl = `https://raw.githubusercontent.com/${manifest.repo}/${manifest.branch}/icon.png`

  const Icon = () => {
    if (hasIcon) {
      return (
        <img
          className="h-10 w-10 rounded-lg object-cover border border-border/50 shadow-xs"
          src={iconUrl}
          alt={`${manifest.alias} icon`}
          loading="lazy"
          onError={() => setIcon(false)}
        />
      )
    }

    return (
      <div className="flex items-center justify-center h-10 w-10 rounded-lg bg-background border border-border/50 shadow-xs">
        <BlocksIcon className="h-5 w-5 opacity-40 stroke-[1.5]" />
      </div>
    )
  }

  const NewBadge = () => {
    if (!isNew) return null
    return (
      <div className="absolute -top-1.5 -right-1.5 bg-red-500/15 text-red-600 border border-red-600/40 text-[8px] font-black px-1.5 py-0.5 rounded-full shadow-xs backdrop-blur-[2px] z-10">
        NEW
      </div>
    )
  }

  const ActionButton = () => {
    if (installed) {
      return (
        <Button
          variant="ghost"
          size="icon"
          className="cursor-default text-green-500 focus-visible:ring-0 focus-visible:ring-offset-0"
          disabled={true}
          aria-label="Installed"
          tabIndex={-1}
        >
          <CheckIcon className="h-5 w-5" />
        </Button>
      )
    }

    return (
      <Button
        variant="ghost"
        size="icon"
        aria-label="View details and install"
        className="cursor-pointer text-muted-foreground hover:text-primary focus-visible:ring-primary/40"
      >
        <DownloadIcon className="h-5 w-5" />
      </Button>
    )
  }

  const Tags = () => {
    if (manifest.tags.length === 0) {
      return <div className="col-span-3"></div>
    }
    return (
      <div className="flex items-center justify-end gap-1 flex-wrap col-span-3">
        {manifest.tags.slice(0, 4).map((tag, index) => (
          <span
            key={`${tag}-${index}`}
            className="text-[10px] font-medium px-1.5 py-0.5 rounded-md bg-secondary/50 border border-secondary/50 text-secondary-foreground shadow-xs whitespace-nowrap transition-all"
          >
            {tag}
          </span>
        ))}
        {manifest.tags.length > 4 && (
          <span className="text-[10px] text-muted-foreground">
            +{manifest.tags.length - 4} more
          </span>
        )}
      </div>
    )
  }

  return (
    <div
      className="grid grid-cols-12 items-center gap-2 bg-muted/30 hover:bg-muted/50 h-16 w-full rounded-xl cursor-pointer transition-all px-4 py-2 border border-border/50 hover:border-primary/30 hover:shadow-sm hover:ring-1 hover:ring-primary/10 group"
      onClick={() => {
        setSelectedManifest(manifest)
        setDialogOpen(true)
      }}
    >
      {/* Left Section: Icon, Title, Stats */}
      <div className="flex items-center gap-3 col-span-5">
        <div className="relative shrink-0">
          <Icon />
          <NewBadge />
        </div>
        <div className="flex flex-col  overflow-hidden">
          {/* First Row: Alias */}
          <div className="text-foreground font-medium truncate group-hover:text-primary transition-colors">
            {manifest.alias}
          </div>
          {/* Second Row: Stats (Downloads/Likes) */}
          <div className="flex items-center gap-3 text-xs text-muted-foreground mt-0.5">
            <div className="flex items-center gap-0.5" title={`${manifest.downloads} downloads`}>
              <DownloadIcon className="w-3 h-3" /> {manifest.downloads}
            </div>
            {manifest.like_percentage && (
              <div
                className="flex items-center gap-0.5"
                title={`${manifest.like_percentage}% likes`}
              >
                <HeartIcon className="w-3 h-3" /> {manifest.like_percentage}%
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Middle Section 1: Author */}
      <div
        className="text-sm text-muted-foreground truncate text-center col-span-3"
        title={`Author: ${manifest.author}`}
      >
        {manifest.author}
      </div>

      {/* Middle Section 2: Tags */}
      <Tags />

      {/* Right Section: Button */}
      <div className="flex justify-end col-span-1">
        <ActionButton />
      </div>
    </div>
  )
}
