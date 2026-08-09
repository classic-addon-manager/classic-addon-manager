import { Browser } from '@wailsio/runtime'
import { HeartIcon } from 'lucide-react'

import { Button } from '@/components/ui/button.tsx'
import type { AddonManifest } from '@/lib/wails'

import { Section } from './Section.tsx'

interface SupportSectionProps {
  manifest: AddonManifest
}

export const SupportSection = ({ manifest }: SupportSectionProps) => {
  if (!manifest.kofi) return null

  return (
    <Section label="Support the Author">
      <Button
        variant="outline"
        size="sm"
        onClick={() => Browser.OpenURL(`https://ko-fi.com/${manifest.kofi}`)}
      >
        <HeartIcon className="h-4 w-4 text-primary" />
        Buy {manifest.author} a coffee
      </Button>
    </Section>
  )
}
