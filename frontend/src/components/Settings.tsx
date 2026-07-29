import { AlertCircleIcon, FolderOpen, Settings2 } from 'lucide-react'
import { type ReactNode, useEffect, useState } from 'react'

import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Switch } from '@/components/ui/switch'
import { toast } from '@/components/ui/toast'
import { getErrorMessage } from '@/lib/utils'
import { ApplicationService } from '@/lib/wails'
import { useSettingsStore } from '@/stores/settingsStore'

import { AccentColorPicker } from './settings/AccentColorPicker'
import { QuickActions } from './settings/QuickActions'

const DIALOG_TITLE = 'Select ArcheAge Classic Documents directory'
const SUCCESS_TITLE = 'Success'
const ERROR_TITLE = 'Error during directory selection.'

const Section = ({
  title,
  description,
  children,
}: {
  title: string
  description?: string
  children: ReactNode
}) => (
  <section className="space-y-3">
    <div className="px-1">
      <h2 className="text-sm font-medium tracking-tight">{title}</h2>
      {description && <p className="mt-0.5 text-xs text-muted-foreground">{description}</p>}
    </div>
    <div className="overflow-hidden rounded-xl border border-border/60 bg-card/40">{children}</div>
  </section>
)

export const Settings = () => {
  const {
    autoPathDetection,
    aacPath,
    isInitialized,
    setAutoPathDetection,
    setAACPath,
    loadConfig,
  } = useSettingsStore()
  const [errDocsPath, setErrDocsPath] = useState('')

  useEffect(() => {
    if (!isInitialized) {
      loadConfig()
    }
  }, [isInitialized, loadConfig])

  const openSelect = async () => {
    setErrDocsPath('')
    try {
      const selectedPath = await ApplicationService.SelectAndValidateDocsPath(DIALOG_TITLE)

      if (selectedPath) {
        setAACPath(selectedPath)
        toast({
          title: SUCCESS_TITLE,
          description: `Path set to: ${selectedPath}`,
        })
      }
    } catch (err) {
      console.error('Directory picker error:', err)
      setErrDocsPath(getErrorMessage(err, 'Failed to select a documents directory'))
    }
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      <header className="border-b bg-background/95 backdrop-blur supports-backdrop-filter:bg-background/60">
        <div className="container flex h-16 items-center gap-4 px-4">
          <div className="flex items-center gap-3">
            <div className="rounded-lg bg-primary/10 p-2">
              <Settings2 className="h-6 w-6 text-primary" />
            </div>
            <div>
              <h1 className="text-xl font-semibold tracking-tight">Settings</h1>
              <p className="text-sm text-muted-foreground">
                Manage Classic Addon Manager preferences
              </p>
            </div>
          </div>
        </div>
      </header>

      <main className="min-h-0 flex-1 overflow-auto">
        <div className="container mx-auto max-w-2xl space-y-8 px-4 py-8">
          <Section
            title="Documents directory"
            description="Where the addon manager looks for your ArcheAge Classic documents"
          >
            <div className="flex items-center justify-between gap-6 px-4 py-3.5">
              <div className="min-w-0 space-y-1">
                <div className="text-sm font-medium leading-none">Override automatic detection</div>
                <p className="text-xs text-muted-foreground">
                  Specify the documents directory yourself
                </p>
              </div>
              <Switch
                checked={!autoPathDetection}
                onCheckedChange={checked => {
                  setAutoPathDetection(!checked)
                  setErrDocsPath('')
                }}
              />
            </div>

            {!autoPathDetection ? (
              <div className="space-y-3 border-t border-border/60 bg-muted/15 px-4 py-4">
                <div className="flex gap-2">
                  <Input
                    id="install-path"
                    type="text"
                    placeholder="C:\AAClassic\Documents\..."
                    value={aacPath}
                    disabled
                    className="flex-1 font-mono text-xs"
                    aria-label="ArcheAge Classic Documents Directory"
                  />
                  <Button
                    variant="outline"
                    size="icon"
                    onClick={openSelect}
                    title="Browse for directory"
                    className="shrink-0"
                  >
                    <FolderOpen className="h-4 w-4" />
                  </Button>
                </div>

                {errDocsPath && (
                  <div className="flex gap-2 rounded-lg border border-destructive/20 bg-destructive/10 p-3">
                    <AlertCircleIcon className="mt-0.5 h-4 w-4 shrink-0 text-destructive" />
                    <div className="space-y-0.5">
                      <div className="text-sm font-medium text-destructive">{ERROR_TITLE}</div>
                      <p className="text-xs text-destructive/80">{errDocsPath}</p>
                    </div>
                  </div>
                )}

                <p className="text-xs leading-relaxed text-muted-foreground">
                  Directory should contain an <span className="text-foreground/80">Addon</span>{' '}
                  folder and <span className="text-foreground/80">system.cfg</span>.
                </p>
              </div>
            ) : (
              <div className="flex items-center gap-2.5 border-t border-border/60 px-4 py-3">
                <span className="relative flex h-2 w-2 shrink-0">
                  <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-emerald-400/40 opacity-75" />
                  <span className="relative inline-flex h-2 w-2 rounded-full bg-emerald-500" />
                </span>
                <p className="text-xs text-muted-foreground">
                  Detecting documents directory automatically
                  {aacPath ? (
                    <>
                      {' · '}
                      <span className="font-mono text-foreground/70">{aacPath}</span>
                    </>
                  ) : null}
                </p>
              </div>
            )}
          </Section>

          <Section
            title="Accent color"
            description="Choose the highlight color used across the app"
          >
            <AccentColorPicker />
          </Section>

          <Section title="Locations" description="Open folders used by Classic Addon Manager">
            <QuickActions />
          </Section>
        </div>
      </main>
    </div>
  )
}
