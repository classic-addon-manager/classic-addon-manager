<script lang="ts">
  import AlertCircle from 'lucide-svelte/icons/alert-circle'
  import CalendarDays from 'lucide-svelte/icons/calendar-days'
  import CheckCircle2 from 'lucide-svelte/icons/check-circle-2'
  import LoaderCircle from 'lucide-svelte/icons/loader-circle'
  import Tag from 'lucide-svelte/icons/tag'

  import addons from '@/addons'
  import {safeCall, toast} from '@/utils'
  import {Button} from '$lib/components/ui/button/index.js'
  import * as Dialog from '$lib/components/ui/dialog/index.js'
  import * as Select from '$lib/components/ui/select'
  import {cn} from '$lib/utils'
  import {type Addon, LocalAddonService, RemoteAddonService} from '$lib/wails'

  interface Release {
    zipball_url: string;
    body: string;
    tag_name: string;
    published_at: string;
  }

  interface ApiResponse {
    status: boolean;
    message: string;
    data: Release[];
  }

  let {
    addon,
    open = $bindable(),
    onOpenChange
  }: {
    addon: Addon;
    open: boolean;
    onOpenChange: (open: boolean) => void;
  } = $props()

  let releases = $state<Release[]>([])
  let selectedVersion = $state<string | undefined>(undefined)
  let isLoading = $state(false)
  let error = $state<string | null>(null)

  // Derived state for trigger content
  const triggerContent = $derived(
    releases.find((r) => r.tag_name === selectedVersion)?.tag_name ?? 'Select a version'
  )

  // Helper to determine if a version is current
  function isCurrentVersion(tagName: string) {
    return tagName === addon.version
  }

  async function fetchReleases() {
    if (!addon) return
    isLoading = true
    error = null
    releases = []
    selectedVersion = undefined
    console.debug(`Fetching releases for ${addon.name}`)

    try {
      const response = await fetch(`https://aac.gaijin.dev/addon/${addon.name}/releases`)
      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`)
      }
      const result: ApiResponse = await response.json()

      if (result.status && result.data) {
        // Filter out duplicates based on tag_name
        const uniqueReleasesMap = new Map<string, Release>()
        for (const release of result.data) {
          if (!uniqueReleasesMap.has(release.tag_name)) {
            uniqueReleasesMap.set(release.tag_name, release)
          }
        }
        releases = Array.from(uniqueReleasesMap.values())
        console.debug(`Fetched ${result.data.length} releases, ${releases.length} unique versions for ${addon.name}`)
      } else {
        throw new Error(result.message || 'Failed to fetch releases: Invalid API response')
      }
    } catch (e: any) {
      console.error(`Failed to fetch releases for ${addon.name}:`, e)
      error = e.message || 'An unknown error occurred.'
      toast.error(`Failed to load versions: ${error}`)
    } finally {
      isLoading = false
    }
  }

  async function handleInstall() {
    if (!selectedVersion) {
      toast.error('Please select a version to install.')
      return
    }
    const selectedRelease = releases.find(r => r.tag_name === selectedVersion)
    if (!selectedRelease) {
      toast.error('Selected version not found in the release list.')
      return
    }
    let manifest = await addons.getManifest(addon.name)
    let installed = await addons.install(manifest, selectedRelease.tag_name)
    if (installed) {
      toast.success('Addon installed successfully.')
      onOpenChange(false)
    } else {
      toast.error('Failed to install addon.')
    }
  }

  // Fetch releases when the dialog becomes visible
  $effect(() => {
    if (open) {
      fetchReleases()
    }
  })
</script>

<Dialog.Root bind:open onOpenChange={onOpenChange}>
  <Dialog.Content class="sm:max-w-[500px]">
    <Dialog.Header class="pb-4 border-b">
      <Dialog.Title class="flex items-center gap-2 text-xl">
        <Tag size={18} class="text-primary"/>
        Install Version
      </Dialog.Title>
      <Dialog.Description class="text-sm text-muted-foreground">
        Select a version of <span class="font-medium text-foreground">{addon.alias}</span> to install.
        <div class="mt-1 flex items-center gap-1.5 text-xs">
          <span class="inline-flex items-center bg-muted px-2 py-0.5 rounded text-muted-foreground">
            Current: <span class="font-medium ml-1">{addon.version}</span>
          </span>
        </div>
      </Dialog.Description>
    </Dialog.Header>

    {#if isLoading}
      <div class="flex items-center justify-center p-8">
        <LoaderCircle class="h-8 w-8 animate-spin text-muted-foreground"/>
        <span class="ml-3 text-muted-foreground">Loading versions...</span>
      </div>
    {:else if error}
      <div class="flex items-center gap-3 bg-destructive/10 text-destructive p-4 rounded-md mt-4">
        <AlertCircle size={20}/>
        <div>
          <p class="font-medium">Error loading versions</p>
          <p class="text-sm opacity-90">{error}</p>
        </div>
      </div>
    {:else if releases.length > 0}
      <div class="py-2">
        <label for="version-select" class="text-sm font-medium mb-2 block">Version</label>
        <Select.Root type="single" bind:value={selectedVersion}>
          <Select.Trigger class="w-full bg-card">
            <div class="flex items-center gap-2">
              {#if selectedVersion}
                <Tag size={14} class="text-primary"/>
              {/if}
              {triggerContent}
            </div>
          </Select.Trigger>
          <Select.Content
            class="max-h-[300px] overflow-y-auto"
            align="start"
            sideOffset={4}
          >
            <Select.Group>
              <Select.GroupHeading class="font-medium px-2 py-1.5 text-xs uppercase">
                Available Versions
              </Select.GroupHeading>
              {#each releases as release (release.tag_name)}
                <Select.Item
                  value={release.tag_name}
                  label={release.tag_name}
                  class={cn(
                    'py-2.5 pl-8 pr-2', 
                    isCurrentVersion(release.tag_name) && 'bg-muted/50'
                  )}
                >
                  <div class="flex flex-col">
                    <div class="flex items-center gap-2">
                      <span class="font-medium">{release.tag_name}</span>
                      {#if isCurrentVersion(release.tag_name)}
                        <span class="bg-primary/15 text-primary text-[10px] px-1.5 py-0.5 rounded-full uppercase font-medium">
                          Current
                        </span>
                      {/if}
                    </div>
                    <div class="flex items-center gap-2 mt-0.5 text-xs text-muted-foreground">
                      <CalendarDays size={12}/>
                      <span>
                        {new Date(release.published_at).toLocaleDateString(undefined, {
                          day: 'numeric',
                          month: 'short',
                          year: 'numeric'
                        })}
                      </span>
                    </div>
                  </div>
                </Select.Item>
              {/each}
            </Select.Group>
          </Select.Content>
        </Select.Root>

        {#if selectedVersion && releases.find(r => r.tag_name === selectedVersion)?.body}
          <div class="mt-4 text-sm">
            <p class="font-medium text-sm mb-1">Release Notes:</p>
            <div class="bg-muted p-3 rounded-md text-xs max-h-[100px] overflow-y-auto whitespace-pre-line">
              {releases.find(r => r.tag_name === selectedVersion)?.body}
            </div>
          </div>
        {/if}
      </div>
    {:else}
      <div class="flex items-center justify-center gap-3 p-8 text-center text-muted-foreground">
        <AlertCircle size={18}/>
        <span>No other versions found for this addon.</span>
      </div>
    {/if}

    <Dialog.Footer class="border-t pt-4 gap-2">
      <Button variant="outline" onclick={() => onOpenChange(false)}>
        Cancel
      </Button>
      <Button
        onclick={handleInstall}
        disabled={isLoading || !!error || !selectedVersion || releases.length === 0}
        class="gap-2"
      >
        {#if selectedVersion}
          <CheckCircle2 size={16}/>
          Install {selectedVersion}
        {:else}
          Install Selected Version
        {/if}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root> 