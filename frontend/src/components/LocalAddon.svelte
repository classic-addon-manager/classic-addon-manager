<script lang="ts">
  import {
    Check as CheckMark,
    Download,
    LoaderCircle,
    ShieldQuestion,
  } from 'lucide-svelte'

  import LocalAddonUpdateDrawer from '@/components/local_addon/LocalAddonUpdateDrawer.svelte'
  import {
    getLatestReleaseMap,
    isCheckingUpdates,
  } from '$atoms/addon.svelte'
  import {localAddonDrawer, updateDrawer} from '$atoms/addon-drawer.svelte'
  import {Badge} from '$lib/components/ui/badge'
  import {cn} from '$lib/utils'
  import type {Addon, Release} from '$lib/wails'

  // import {openAddonDrawer} from '$stores/LocalAddonDrawerStore.svelte.js'
  import LocalAddonContextMenu from './local_addon/LocalAddonContextMenu.svelte'
  // import LocalAddonDialog from './local_addon/LocalAddonDialog.svelte'
  import LocalAddonUpdateDialog from './local_addon/LocalAddonUpdateDialog.svelte'

  interface Props {
    addon: Addon;
  }

  let {addon}: Props = $props()

  let latestRelease = $derived(getLatestReleaseMap().get(addon.name))
  let isChecking = $derived(isCheckingUpdates())

  let openUpdateDialog = $state(false)
  let isContextMenuOpen = $state(false)

  function handleOpenUpdateDialogChange(o: boolean) {
    openUpdateDialog = o
  }

  function handleContextMenuOpenChange(open: boolean) {
    isContextMenuOpen = open
  }
</script>

<!--<LocalAddonDialog-->
<!--        {addon}-->
<!--        bind:open={openDrawer}-->
<!--        onOpenChange={handleOpenDrawerChange}-->
<!--/>-->

{#if latestRelease}
  <LocalAddonUpdateDialog
    {addon}
    release={latestRelease}
    bind:open={openUpdateDialog}
    onOpenChange={handleOpenUpdateDialogChange}
  />
{/if}

{#snippet contextTriggerArea()}
  <div
    class={cn(
      'cursor-pointer grid grid-cols-4 p-2 hover:bg-muted/50 border-t transition-colors items-center text-sm',
      isContextMenuOpen && 'bg-muted',
    )}
    onclick={() => localAddonDrawer.open(addon)}
  >
    <div class="font-medium">{addon.alias}</div>
    <div class="text-center">{addon.author}</div>
    <div class="text-center">{addon.version}</div>
    <div class="text-center">
      {#if addon.isManaged}
        {#if isChecking}
          <div class="flex justify-center">
            <LoaderCircle size={20} class="mr-1 animate-spin"/>
          </div>
        {:else if latestRelease}
          {#if latestRelease.published_at > addon.updatedAt}
            <Badge
              class="py-1 cursor-pointer flex-shrink-0 flex-grow-0"
              variant="default"
              onclick={(e) => {
                e.preventDefault()
                e.stopPropagation()
                /*openUpdateDialog = true*/
                updateDrawer.open(addon, latestRelease)
              }}
            >
              <Download size={14} class="mr-1"/>
              Update ({latestRelease.tag_name})
            </Badge>
          {:else}
            <div class="flex justify-center">
              <CheckMark size={20} class="text-green-600 mr-2"/>
              Up to date
            </div>
          {/if}
        {:else}
          <div class="flex justify-center">
            <CheckMark size={20} class="text-green-600 mr-2"/>
            Up to date
          </div>
        {/if}
      {:else}
        <Badge class="py-1 cursor-pointer" variant="warning">
          <ShieldQuestion size={14} class="mr-2"/>
          Not managed
        </Badge>
      {/if}
    </div>
  </div>
{/snippet}

<LocalAddonContextMenu
  {addon}
  {contextTriggerArea}
  onOpenChange={handleContextMenuOpenChange}
/>
