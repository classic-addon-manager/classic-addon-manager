<script lang="ts">
  import {AlertCircleIcon, FolderOpen, HardDrive, Settings2} from 'lucide-svelte'
  import {toast} from 'svelte-sonner'

  import * as Alert from '$lib/components/ui/alert'
  import {Button} from '$lib/components/ui/button'
  import {Card, CardContent, CardDescription, CardHeader, CardTitle} from '$lib/components/ui/card'
  import {Input} from '$lib/components/ui/input'
  import {Separator} from '$lib/components/ui/separator'
  import {Switch} from '$lib/components/ui/switch'
  import {ApplicationService} from '$lib/wails'
  import {
    autoPathDetection, getAACPath,
    setAutoPathDetection
  } from '$stores/SettingsStore.svelte'

  let pathInput = $state(getAACPath())
  let errDocsPath = $state('')

  async function openSelect() {
    try {
      const selectedPath = await ApplicationService.SelectAndValidateDocsPath('Select ArcheAge Classic Documents directory')

      if (selectedPath) {
        pathInput = selectedPath
        toast.success(`Path set to: ${selectedPath}`)
        errDocsPath = ''
      }
    } catch (err: any) {
      console.error('Directory picker error:', err)
      pathInput = ''
      errDocsPath = err.message
    }
  }
</script>

<div class="flex flex-col h-screen">
  <header class="bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 border-b">
    <div class="container flex h-16 items-center gap-4 px-4">
      <div class="flex items-center gap-3">
        <div class="p-2 bg-primary/10 rounded-lg">
          <Settings2 class="h-6 w-6 text-primary"/>
        </div>
        <div>
          <h1 class="text-xl font-semibold tracking-tight">Settings</h1>
          <p class="text-sm text-muted-foreground">Manage Classic Addon Manager preferences</p>
        </div>
      </div>
    </div>
  </header>

  <main class="flex-1 overflow-auto">
    <div class="container px-4 py-6 max-w-4xl">
      <div class="grid gap-6">
        <!-- Directory Settings Card -->
        <Card class="shadow-sm">
          <CardHeader class="pb-3">
            <div class="flex items-center gap-2">
              <HardDrive class="h-5 w-5 text-primary"/>
              <CardTitle class="text-xl">AAC Documents Directory</CardTitle>
            </div>
            <CardDescription>
              Configure how the addon manager detects your ArcheAge Classic documents directory
            </CardDescription>
          </CardHeader>
          <CardContent class="space-y-6">
            <!-- Override Toggle -->
            <div class="flex items-start justify-between p-4 bg-muted/30 rounded-lg border">
              <div class="space-y-1">
                <div class="font-medium">Override Automatic Detection</div>
                <div class="text-sm text-muted-foreground">
                  Manually specify your AAC documents directory instead of using automatic detection
                </div>
              </div>
              <Switch
                checked={!autoPathDetection()}
                onCheckedChange={() => {
                  setAutoPathDetection(!autoPathDetection())
                  ApplicationService.SettingsSetAutoDetectPath(autoPathDetection())
                }}
              />
            </div>

            <!-- Directory Path Configuration -->
            {#if !autoPathDetection()}
              <div class="space-y-4 p-4 bg-background border rounded-lg">
                <div class="space-y-2">
                  <label for="install-path" class="text-sm font-medium">
                    ArcheAge Classic Documents Directory
                  </label>
                  <div class="flex gap-2">
                    <Input
                      id="install-path"
                      type="text"
                      placeholder="C:\AAClassic\Documents\..."
                      bind:value={pathInput}
                      disabled={true}
                      class="flex-1"
                    />
                    <Button
                      variant="outline"
                      size="icon"
                      onclick={openSelect}
                      class="shrink-0"
                      title="Browse for directory"
                    >
                      <FolderOpen class="h-4 w-4"/>
                    </Button>
                  </div>
                </div>

                {#if errDocsPath}
                  <Alert.Root variant="destructive">
                    <AlertCircleIcon/>
                    <Alert.Title class="ml-1">Error during directory selection.</Alert.Title>
                    <Alert.Description class="ml-1">
                      <p>{errDocsPath}</p>
                    </Alert.Description>
                  </Alert.Root>
                {/if}

                <div class="text-xs text-muted-foreground bg-muted/50 p-3 rounded border-l-4 border-l-primary/50">
                  <strong>Note:</strong> The documents directory should contain the "Addon" directory
                  and "system.cfg" file.
                  This path will be used to locate addon directories and game resources.
                </div>
              </div>
            {:else}
              <div class="p-4 bg-muted/20 rounded-lg border border-dashed">
                <div class="text-center text-muted-foreground">
                  <HardDrive class="h-8 w-8 mx-auto mb-2 opacity-50"/>
                  <p class="text-sm">Automatic directory detection is enabled</p>
                  <p class="text-xs mt-1">The application will automatically find your ArcheAge
                    Classic documents directory</p>
                </div>
              </div>
            {/if}
          </CardContent>
        </Card>

      </div>
    </div>
  </main>
</div>
