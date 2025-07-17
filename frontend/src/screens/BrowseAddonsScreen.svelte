<script lang="ts">
  import PackageSearch from 'lucide-svelte/icons/package-search'
  import RefreshCw from 'lucide-svelte/icons/refresh-cw'
  import Search from 'lucide-svelte/icons/search'
  import {onMount} from 'svelte'
  import {fade, fly} from 'svelte/transition'

  import {timeAgo, toast} from '@/utils'
  import {safeCall} from '@/utils'
  import {Button} from '$lib/components/ui/button'
  import {Input} from '$lib/components/ui/input/index.js'
  import * as Select from '$lib/components/ui/select/index.js'
  import type {AddonManifest} from '$lib/wails'
  import {LocalAddonService, RemoteAddonService} from '$lib/wails'

  import RemoteAddonDialog from '../components/remote_addon/RemoteAddonDialog.svelte'
  import RemoteAddonSkeleton from '../components/remote_addon/RemoteAddonSkeleton.svelte'
  import RemoteAddon from '../components/RemoteAddon.svelte'

  type AddonListItem = {
    manifest: AddonManifest;
    isInstalled: boolean;
  };

  let isReady: boolean = $state(false)
  let isRefreshing: boolean = $state(false)
  let searchPhrase: string = $state('')
  let addons: AddonListItem[] = $state([])
  let tags = [
    {label: 'All', value: 'all'},
  ]

  let selectedTag: string = $state('all')
  let isDialogOpen: boolean = $state(false)
  let selectedAddonForDialog: AddonManifest | null = $state(null)

  const triggerContent = $derived(
    tags.find((t) => t.value === selectedTag)?.label ?? 'Select a tag'
  )

  const filteredAddons = $derived.by(() => {
    return addons.filter(item => {
      if (selectedTag != 'all' && !item.manifest.tags.includes(selectedTag)) {
        return false
      }
      return item.manifest.alias.toLowerCase().includes(searchPhrase.toLowerCase()) || item.manifest.description.toLowerCase().includes(searchPhrase.toLowerCase())
    })
  })

  onMount(async () => {
    await loadAddons()
    isReady = true
  })

  async function loadAddons() {
    const manifests = await RemoteAddonService.GetAddonManifest()
    const installStatusPromises = manifests.map(m => LocalAddonService.IsInstalled(m.name))
    const installStatuses = await Promise.all(installStatusPromises)

    let tmp: AddonListItem[] = manifests.map((manifest, index) => ({
      manifest,
      isInstalled: installStatuses[index]
    }))

    const uniqueTags = new Set<string>()
    manifests.forEach(manifest => {
      manifest.tags.forEach(tag => {
        if (tag !== 'Example') {
          uniqueTags.add(tag)
        }
      })
    })

    const sortedTags = Array.from(uniqueTags).sort((a, b) => a.localeCompare(b))
    tags = [
      {label: 'All', value: 'all'},
      ...sortedTags.map(tag => ({label: tag, value: tag}))
    ]

    tmp.sort((a, b) => {
      const aIsNew = timeAgo(a.manifest.added_at) < 32
      const bIsNew = timeAgo(b.manifest.added_at) < 32

      if (aIsNew && !bIsNew) return -1
      if (!aIsNew && bIsNew) return 1

      if (aIsNew && bIsNew) {
        const aTime = new Date(a.manifest.added_at).getTime()
        const bTime = new Date(b.manifest.added_at).getTime()
        if (aTime !== bTime) {
          return bTime - aTime
        }
      }

      return a.manifest.name.localeCompare(b.manifest.name)
    })

    addons = tmp
    searchPhrase = ''
  }

  async function getAddonManifest() {
    if (isRefreshing) return

    isRefreshing = true
    const startTime = Date.now()

    const [, error] = await safeCall(loadAddons)
    if (error) {
      console.error('Failed to refresh addons', error)
      toast.error(`Failed to refresh addons: ${error}`)
      return
    }

    // Only add artificial delay when manually refreshing
    const elapsedTime = Date.now() - startTime
    if (elapsedTime < 500) {
      await new Promise(resolve => setTimeout(resolve, 500 - elapsedTime))
    }

    toast.success('Addons refreshed')
    isRefreshing = false
  }

  function viewAddonDetails(addonToView: AddonManifest) {
    selectedAddonForDialog = addonToView
    isDialogOpen = true
  }

  function closeDialog() {
    isDialogOpen = false
    setTimeout(() => {
      selectedAddonForDialog = null
    }, 300)
  }

  function handleInstallSuccess(installedManifest: AddonManifest) {
    addons = addons.map(item =>
      item.manifest.name === installedManifest.name
        ? {...item, isInstalled: true}
        : item
    )
    closeDialog()
  }

  function handleUninstallSuccess(uninstalledManifest: AddonManifest) {
    addons = addons.map(item =>
      item.manifest.name === uninstalledManifest.name
        ? {...item, isInstalled: false}
        : item
    )
    closeDialog()
  }
</script>

<div class="flex flex-col h-screen">
  <header class="bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 border-b">
    <div class="container flex h-16 items-center gap-4 px-4">
      <div class="relative flex-1">
        <Search
          class="text-muted-foreground absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 transform"
        />
        <Input
          disabled={!isReady}
          type="search"
          placeholder="Search addons..."
          class="bg-background w-full pl-10 shadow-none transition-colors focus-visible:bg-background/80"
          bind:value={searchPhrase}
        />
      </div>
      <div class="flex items-center gap-3 shrink-0">
        <Select.Root type="single" bind:value={selectedTag} allowDeselect={false} disabled={!isReady}>
          <Select.Trigger class="w-[140px] bg-background/80">
            {triggerContent}
          </Select.Trigger>
          <Select.Content>
            {#each tags as tag (tag.value)}
              <Select.Item value={tag.value} label={tag.label}/>
            {/each}
          </Select.Content>
        </Select.Root>
        <Button
          variant="outline"
          disabled={!isReady || isRefreshing}
          class="min-w-[120px] w-[120px] flex items-center justify-center gap-2 transition-all duration-200 hover:shadow-md"
          onclick={async() => {
            await getAddonManifest()
          }}
        >
          <RefreshCw class="h-4 w-4 {isRefreshing ? 'animate-spin' : ''}"/>
          {isRefreshing ? 'Refreshing' : 'Refresh'}
        </Button>
      </div>
    </div>
  </header>

  <main class="flex-1 overflow-auto">
    <div class="container px-4">
      {#if !isReady}
        <div class="flex flex-1 flex-col gap-4 py-4 relative" in:fade>
          <RemoteAddonSkeleton/>
          <RemoteAddonSkeleton/>
          <RemoteAddonSkeleton/>
          <RemoteAddonSkeleton/>
          <RemoteAddonSkeleton/>
        </div>
      {:else if filteredAddons.length === 0}
        <div class="flex flex-col items-center justify-center py-16 text-center text-muted-foreground" in:fade>
          <PackageSearch class="h-12 w-12 mb-4"/>
          <h3 class="text-lg font-semibold mb-2">No addons found</h3>
          <p class="max-w-sm">
            {#if searchPhrase}
              No addons match your search criteria. Try adjusting your search or filters.
            {:else}
              No addons are currently available. Check back later or try refreshing.
            {/if}
          </p>
        </div>
      {:else}
        <div class="flex flex-1 flex-col gap-4 py-4">
          {#each filteredAddons as item (item.manifest.name)}
            <div in:fly={{y: 20, duration: 200}}>
              <RemoteAddon
                addon={item.manifest}
                isInstalled={item.isInstalled}
                onViewDetails={() => viewAddonDetails(item.manifest)}
              />
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </main>
</div>

{#if selectedAddonForDialog}
  <RemoteAddonDialog
    bind:open={isDialogOpen}
    addon={selectedAddonForDialog}
    onOpenChange={(o) => { if (!o) closeDialog() }}
    onInstall={handleInstallSuccess}
    onViewDependency={viewAddonDetails}
    onUninstall={handleUninstallSuccess}
  />
{/if}

<style>
    /* Add smooth transitions */
    :global(.transition-colors) {
        transition-property: background-color, border-color, color, fill, stroke;
        transition-timing-function: cubic-bezier(0.4, 0, 0.2, 1);
        transition-duration: 150ms;
    }
</style>
