<script lang="ts">
  import {Browser} from '@wailsio/runtime'
  import LoaderCircle from 'lucide-svelte/icons/loader-circle'
  import semver from 'semver'
  import {onMount} from 'svelte'

  import aacLogo from '@/assets/images/aac-logo-wide.png'
  import {AACWebsiteItem, AddonsItem, DashboardItem} from '@/components/navbar'
  import {TroubleshootingItem} from '@/components/navbar'
  import UserBar from '@/components/UserBar.svelte'
  import {safeCall} from '@/utils'
  import {getVersion} from '$atoms/application.svelte'
  import {getActiveScreen} from '$atoms/screen.svelte'
  import {Button} from '$lib/components/ui/button/index'
  import * as Card from '$lib/components/ui/card/index'
  import {
    ApplicationRelease,
    ApplicationService
  } from '$lib/wails'

  let updateAvailable = $state(false)
  let updateInformation = $state({
    version: '',
    url: '',
  })
  let isUpdating = $state(false)

  let ActiveScreen = $derived.by(() => getActiveScreen())

  onMount(async () => {
    const [releaseInfo, error] = await safeCall<ApplicationRelease>(ApplicationService.GetLatestRelease)
    if (error) {
      console.error('Failed to get latest release: ', error)
      return
    }
    if (releaseInfo) {
      updateInformation = releaseInfo
      if (isNewerVersion(updateInformation.version)) {
        updateAvailable = true
      }
    }
  })

  function isNewerVersion(version: string): boolean {
    return semver.gt(version, getVersion())
  }
</script>

<div class="grid min-h-screen w-full md:grid-cols-[220px_1fr]">
  <div class="bg-muted/40 border-r">
    <div class="flex h-full max-h-screen flex-col gap-2">
      <div class="flex h-14 items-center border-b px-4">
        <div class="flex items-center gap-2 font-semibold">
          <img
            src={aacLogo}
            alt="ArcheAge Classic Logo"
            class="h-12 w-auto"
          />
        </div>
      </div>
      <div class="flex-1">
        <nav class="grid items-start px-2 text-sm font-medium">
          <DashboardItem/>
          <AddonsItem/>
          <TroubleshootingItem/>
          <AACWebsiteItem/>
        </nav>
      </div>

      {#if updateAvailable}
        <div class="mt-auto p-4">
          <Card.Root
            data-x-chunk-name="dashboard-02-chunk-0"
            data-x-chunk-description="A card with a call to action"
          >
            <Card.Header class="p-2 pt-0 md:p-4">
              <Card.Title>Update available!</Card.Title>
              <Card.Description>
                Version {updateInformation.version} is available.
                <br/>
                <br/>
                Click the button below to update.
              </Card.Description>
            </Card.Header>
            <Card.Content class="p-2 pt-0 md:p-4 md:pt-0">
              <Button
                size="sm"
                class="w-full"
                onclick={() => {
                  isUpdating = true
                  ApplicationService.SelfUpdate(updateInformation.url)
                }}
              >
                {#if isUpdating}
                  <LoaderCircle class="mr-2 size-4 animate-spin"/>
                  Updating...
                {:else}
                  Update
                {/if}
              </Button>
            </Card.Content>
          </Card.Root>
        </div>
      {/if}

      <div class="w-full">
        <UserBar/>
      </div>

      <div class="mx-auto mb-2 text-gray-300 text-opacity-40">
        <span class="hover:text-blue-400 cursor-pointer transition-all"
          onclick={() => Browser.OpenURL(`https://github.com/classic-addon-manager/classic-addon-manager/releases/tag/v${getVersion()}`)}>v{getVersion()}
          by Sami</span>
      </div>
    </div>
  </div>
  <div class="flex flex-col">
    <ActiveScreen/>
  </div>
</div>
