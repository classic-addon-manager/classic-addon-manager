import { AlertCircleIcon, FolderOpen, HardDrive, Settings2 } from 'lucide-react'
import { useEffect, useState } from 'react'

import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Switch } from '@/components/ui/switch'
import { toast } from '@/components/ui/toast'
import { ApplicationService } from '@/lib/wails'
import { useSettingsStore } from '@/stores/settingsStore'

import { QuickActions } from './settings/QuickActions'

const DIALOG_TITLE = 'Select ArcheAge Classic Documents directory'
const SUCCESS_TITLE = 'Success'
const ERROR_TITLE = 'Error during directory selection.'

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
    try {
      const selectedPath = await ApplicationService.SelectAndValidateDocsPath(DIALOG_TITLE)

      if (selectedPath) {
        setAACPath(selectedPath)
        toast({
          title: SUCCESS_TITLE,
          description: `Path set to: ${selectedPath}`,
        })
        setErrDocsPath('')
      }
    } catch (err) {
      console.error('Directory picker error:', err)
      setErrDocsPath(err instanceof Error ? err.message : 'An unknown error occurred')
    }
  }

  const toggleAutoDetection = () => setAutoPathDetection(!autoPathDetection)

  return (
    <div className="flex flex-col h-screen">
      <header className="bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 border-b">
        <div className="container flex h-16 items-center gap-4 px-4">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-primary/10 rounded-lg">
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

      <main className="flex-1 overflow-auto">
        <div className="container mx-auto px-4 py-6 max-w-4xl">
          <Card className="shadow-sm">
            <CardHeader className="pb-3">
              <div className="flex items-center gap-2">
                <HardDrive className="h-5 w-5 text-primary" />
                <CardTitle className="text-xl">AAC Documents Directory</CardTitle>
              </div>
              <CardDescription>
                Configure how the addon manager detects your ArcheAge Classic documents directory
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              {/* Override Toggle */}
              <div className="flex items-start justify-between p-4 bg-muted/30 rounded-lg border">
                <div className="space-y-1">
                  <div className="font-medium">Override Automatic Detection</div>
                  <div className="text-sm text-muted-foreground">
                    Manually specify your AAC documents directory instead of using automatic
                    detection
                  </div>
                </div>
                <Switch checked={!autoPathDetection} onCheckedChange={toggleAutoDetection} />
              </div>

              {/* Directory Path Configuration */}
              {!autoPathDetection ? (
                <div className="space-y-4 p-4 bg-background border rounded-lg">
                  <div className="space-y-2">
                    <label htmlFor="install-path" className="text-sm font-medium">
                      ArcheAge Classic Documents Directory
                    </label>
                    <div className="flex gap-2">
                      <Input
                        id="install-path"
                        type="text"
                        placeholder="C:\AAClassic\Documents\..."
                        value={aacPath}
                        disabled
                        className="flex-1"
                      />
                      <Button
                        variant="outline"
                        size="icon"
                        onClick={openSelect}
                        title="Browse for directory"
                      >
                        <FolderOpen className="h-4 w-4" />
                      </Button>
                    </div>
                  </div>

                  {errDocsPath && (
                    <div className="flex gap-2 p-3 bg-destructive/10 border border-destructive/20 rounded-lg">
                      <AlertCircleIcon className="h-4 w-4 text-destructive mt-0.5 shrink-0" />
                      <div className="space-y-1">
                        <div className="text-sm font-medium text-destructive">{ERROR_TITLE}</div>
                        <p className="text-sm text-destructive/80">{errDocsPath}</p>
                      </div>
                    </div>
                  )}

                  <div className="text-xs text-muted-foreground bg-muted/50 p-3 rounded border-l-4 border-l-primary/50">
                    <strong>Note:</strong> The documents directory should contain the "Addon"
                    directory and "system.cfg" file. This path will be used to locate addon
                    directories and game resources.
                  </div>
                </div>
              ) : (
                <div className="p-4 bg-muted/20 rounded-lg border border-dashed">
                  <div className="text-center text-muted-foreground">
                    <HardDrive className="h-8 w-8 mx-auto mb-2 opacity-50" />
                    <p className="text-sm">Automatic directory detection is enabled</p>
                    <p className="text-xs mt-1">
                      The application will automatically find your ArcheAge Classic documents
                      directory
                    </p>
                  </div>
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      </main>

      <footer className="border-t bg-muted/30">
        <div className="container px-4 py-4 max-w-4xl">
          <div className="flex items-center justify-between">
            <div className="text-sm text-muted-foreground">Quick Actions</div>
            <QuickActions />
          </div>
        </div>
      </footer>
    </div>
  )
}
