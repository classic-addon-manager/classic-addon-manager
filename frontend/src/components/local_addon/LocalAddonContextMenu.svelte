<script lang="ts">
  import {Browser} from '@wailsio/runtime'
  import AlertTriangle from 'lucide-svelte/icons/alert-triangle'
  import Bug from 'lucide-svelte/icons/bug'
  import FolderOpen from 'lucide-svelte/icons/folder-open'
  import GitBranch from 'lucide-svelte/icons/git-branch'
  import Github from 'lucide-svelte/icons/github'
  import Trash from 'lucide-svelte/icons/trash'
  import type {Snippet} from 'svelte'

  import addons from '@/addons'
  import {toast} from '@/utils'
  import * as ContextMenu from '$lib/components/ui/context-menu/index.js'
  import {type Addon, LocalAddonService} from '$lib/wails'

  import LocalAddonVersionSelectDialog from './LocalAddonVersionSelectDialog.svelte'

  let {
    contextTriggerArea,
    addon,
    onOpenChange,
  }: {
    contextTriggerArea: Snippet;
    addon: Addon;
    onOpenChange?: (open: boolean) => void;
  } = $props()

  let isVersionSelectOpen = $state(false)

  async function handleAddonAction(action: 'uninstall' | 'unmanage') {
    const actionVerb = action === 'uninstall' ? 'Uninstalling' : 'Unmanaging'
    const actionPast = action === 'uninstall' ? 'Uninstalled' : 'Unmanaged'

    console.debug(`${actionVerb} addon: ${addon.name}`)
    const success = await addons[action](addon.name)

    if (success) {
      console.debug(`${actionPast} addon: ${addon.name}`)
      toast.success(`${addon.alias} was ${action}ed`)
    } else {
      console.error(`Failed to ${action} addon: ${addon.name}`)
      toast.error(`Failed to ${action} ${addon.alias}`)
    }
  }

  async function handleOpenDirectory() {
    try {
      await LocalAddonService.OpenDirectory(addon.name)
    } catch (e) {
      console.error('Failed to open directory: ', e)
      toast.error(`Failed to open directory: ${e}`)
    }
  }
</script>

<ContextMenu.Root {onOpenChange}>
  <ContextMenu.Trigger>
    {@render contextTriggerArea()}
  </ContextMenu.Trigger>
  <ContextMenu.Content>
    <ContextMenu.Item class="gap-2" onclick={handleOpenDirectory}>
      <FolderOpen size={16}/>
      Open directory
    </ContextMenu.Item>

    {#if addon.isManaged}
      <ContextMenu.Item class="gap-3" onclick={() => Browser.OpenURL(`https://github.com/${addon.repo}`)}>
        <Github size={16}/>
        View code
      </ContextMenu.Item>
      <ContextMenu.Item class="gap-2"
        onclick={() => Browser.OpenURL(`https://github.com/${addon.repo}/issues/new`)}>
        <Bug size={16}/>
        Report issue
      </ContextMenu.Item>
      <ContextMenu.Item class="gap-2" onclick={() => {isVersionSelectOpen = true}}>
        <GitBranch size={16}/>
        Install other version
      </ContextMenu.Item>
    {/if}

    <ContextMenu.Item class="gap-2" onclick={() => handleAddonAction('uninstall')}>
      <Trash size={16}/>
      Uninstall
    </ContextMenu.Item>

    {#if addon.isManaged}
      <ContextMenu.Item class="gap-2" onclick={() => handleAddonAction('unmanage')}>
        <AlertTriangle size={16}/>
        Unmanage
      </ContextMenu.Item>
    {/if}
  </ContextMenu.Content>
</ContextMenu.Root>

{#if addon}
  <LocalAddonVersionSelectDialog
    {addon}
    bind:open={isVersionSelectOpen}
    onOpenChange={(o) => isVersionSelectOpen = o}
  />
{/if}
