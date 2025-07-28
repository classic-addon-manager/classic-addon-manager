<script lang="ts">
    import * as Dialog from "$lib/components/ui/dialog/index";
    import * as Tabs from "$lib/components/ui/tabs/index";
    import * as Tooltip from "$lib/components/ui/tooltip/index";
    import type {AddonManifest, Release} from "$lib/wails";
    import addons from "../../addons";
    import {LocalAddonService, RemoteAddonService} from "$lib/wails";
    import {Browser} from "@wailsio/runtime";
    import {Button} from "$lib/components/ui/button";
    import {GithubIcon, BugIcon, DownloadIcon, Tag, User, CalendarDays, Trash2Icon} from "lucide-svelte";
    import Like from "lucide-svelte/icons/thumbs-up";
    import Dislike from "lucide-svelte/icons/thumbs-down";
    import Heart from "lucide-svelte/icons/heart";
    import Package from "lucide-svelte/icons/package";
    import {apiClient} from "../../api";
    import {isAuthenticated} from "$stores/UserStore.svelte";
    import {toast} from "../../utils";
    import DOMPurify from "dompurify";
    import {marked} from "marked";
    import {Badge} from "$lib/components/ui/badge/index.js";
    import RemoteAddonReadme from "./RemoteAddonReadme.svelte";
    import {
        resolveTransitiveDependencies,
        installDependenciesInOrder,
        type DependencyInfo
    } from "$lib/dependency-resolver";

    let {
        open = $bindable(),
        onOpenChange,
        onInstall,
        onViewDependency,
        addon,
        onUninstall,
    }: {
        open: boolean;
        onOpenChange: (open: boolean) => void;
        onInstall: (installedManifest: AddonManifest) => void;
        onViewDependency: (manifest: AddonManifest) => void;
        addon: AddonManifest;
        onUninstall: (uninstalledManifest: AddonManifest) => void;
    } = $props();

  type DependencyInfo = {
    manifest: AddonManifest;
    isInstalled: boolean;
  };

  let {
    open = $bindable(),
    onOpenChange,
    onInstall,
    onViewDependency,
    addon,
    onUninstall,
  }: {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    onInstall: (installedManifest: AddonManifest) => void;
    onViewDependency: (manifest: AddonManifest) => void;
    addon: AddonManifest;
    onUninstall: (uninstalledManifest: AddonManifest) => void;
  } = $props()

  let release: Release | undefined = $state()
  let isInstalled = $state(false)
  let hasBanner = $state(false)
  let banner: string = $state('')
  let readme = $state('Loading description...')
  let changelog = $state('Loading changelog...')
  let rating = $state(0)
  let tabs = $derived(getTabs())
  let dependencies: DependencyInfo[] = $state([])
  let currentTab = $state('description')

  marked.setOptions({
    gfm: true,
    breaks: true,
  })

  async function initializeData(openState: boolean) {
    if (!openState) return
    currentTab = 'description'
    readme = 'Loading description...'
    changelog = 'Loading changelog...'
    dependencies = []
    release = undefined
    rating = 0
    hasBanner = false

    isInstalled = await LocalAddonService.IsInstalled(addon.name)

    hasBanner = await checkForBanner()

    await Promise.allSettled([
      getRelease(),
      getMyRating(),
      getReadme(),
      getDependencies()
    ])
  }

  $effect(() => {
    initializeData(open)
  })

  async function checkForBanner() {
    try {
      const response = await fetch(
        `https://raw.githubusercontent.com/${addon.repo}/${addon.branch}/banner.png`,
      )
      if (!response.ok) return false

      const blob = await response.blob()
      if (banner && banner.startsWith('blob:')) {
        URL.revokeObjectURL(banner)
      }
      banner = URL.createObjectURL(blob)
      return true
    } catch (e) {
      console.error('Error fetching banner:', e)
      return false
    }
  }

  async function getReadme() {
    try {
      const response = await fetch(
        `https://raw.githubusercontent.com/${addon.repo}/refs/heads/${addon.branch}/README.md`,
      )
      if (!response.ok) {
        const fallbackReadme = addon.description || 'No description provided.'
        readme = DOMPurify.sanitize(marked.parse(fallbackReadme, {async: false}))
        return
      }
      const rawReadme = await response.text()
      readme = DOMPurify.sanitize(await marked.parse(rawReadme))
    } catch (e) {
      console.error('Error fetching README:', e)
      const errorReadme = addon.description || 'Error loading description.'
      readme = DOMPurify.sanitize(marked.parse(errorReadme, {async: false}))
    }
  }

  async function getRelease() {
    try {
      const latestRelease = await RemoteAddonService.GetLatestRelease(addon.name)
      release = latestRelease
      if (latestRelease?.body) {
        changelog = DOMPurify.sanitize(await marked.parse(latestRelease.body))
      } else {
        changelog = 'No changelog provided.'
      }
    } catch (e) {
      toast.error(
        `Failed to fetch release information for ${addon.name}`,
      )
      console.error('Fetch release error:', e)
      changelog = 'Error loading changelog.'
    }
  }

    async function getDependencies() {
        if (addon.dependencies && addon.dependencies.length > 0) {
            try {
                const result = await resolveTransitiveDependencies(addon);

                if (result.errors.length > 0) {
                    console.warn("Dependency resolution errors:", result.errors);
                    // Show first error to user, log all errors
                    toast.error(`Dependency resolution warning: ${result.errors[0]}`);
                }

                dependencies = result.dependencies;
                console.log(`Found ${dependencies.length} total dependencies (including transitive) for ${addon.alias}`);

                // Log dependency tree for debugging
                if (dependencies.length > 0) {
                    console.log("Dependency tree:");
                    dependencies.forEach(dep => {
                        console.log(`  ${"  ".repeat(dep.depth)}${dep.manifest.alias} (${dep.isInstalled ? 'installed' : 'not installed'})`);
                    });
                }

            } catch (e) {
                toast.error("Failed to fetch dependencies");
                console.error("Failed to fetch dependencies", e);
                dependencies = [];
            }
        } else {
            dependencies = [];
        }
    }
  }

  function formatToLocalTime(dateString: string | undefined, includeTime: boolean = true): string {
    if (!dateString) return 'N/A'
    try {
      const date = new Date(dateString)
      let options: Intl.DateTimeFormatOptions = {
        day: '2-digit',
        month: 'long',
        year: 'numeric',
      }
      if (includeTime) {
        options = {
          ...options,
          hour: '2-digit',
          minute: '2-digit',
        }
      }
      return isNaN(date.getTime()) ? 'Invalid Date' : date.toLocaleString(undefined, options).replace(',', '')
    } catch (e) {
      console.error('Error formatting date:', dateString, e)
      return 'Invalid Date'
    }
  }

  async function handleInstall(): Promise<void> {
    if (isInstalled) {
      toast.info('Addon is already installed')
      return
    }
    if (!release) {
      toast.error('Cannot install addon without a release.')
      return
    }

    let didInstall: boolean = false
    try {
      toast.info(`Installing ${addon.alias}...`)
      if (dependencies.length > 0) {
        toast.info(`Installing ${dependencies.length} dependencies...`)
        let failedDeps = 0
        for (const depInfo of dependencies) {
          if (depInfo.isInstalled) {
            console.log(`Dependency ${depInfo.manifest.alias} already installed.`)
            continue
          }
          try {
            await addons.install(depInfo.manifest, 'latest')
            toast.success('Dependency installed', {
              description: `${depInfo.manifest.alias} was installed.`,
            })
            depInfo.isInstalled = true
          } catch (e: any) {
            failedDeps++
            toast.error('Failed to install dependency', {
              description: `Failed to install ${depInfo.manifest.alias}: ${e?.message || e}`,
            })
            console.error(`Dependency install error for ${depInfo.manifest.alias}:`, e)
          }
        }
        if (failedDeps > 0) {
          toast.error(`${failedDeps} dependencies failed to install. Main addon installation aborted.`)
          return
        }
      }

        let didInstall: boolean = false;
        try {
            toast.info(`Installing ${addon.alias}...`);
            if (dependencies.length > 0) {
                const uninstalledDeps = dependencies.filter(d => !d.isInstalled);
                if (uninstalledDeps.length > 0) {
                    toast.info(`Installing ${uninstalledDeps.length} dependencies (including transitive)...`);

                    const failedDepNames = await installDependenciesInOrder(dependencies);

                    if (failedDepNames.length > 0) {
                        const failedDepAliases = failedDepNames.map(name => {
                            const dep = dependencies.find(d => d.manifest.name === name);
                            return dep ? dep.manifest.alias : name;
                        }).join(", ");

                        toast.error(`${failedDepNames.length} dependencies failed to install. Main addon installation aborted.`, {
                            description: `Failed: ${failedDepAliases}`,
                        });
                        return;
                    }

                    toast.success(`All ${uninstalledDeps.length} dependencies installed successfully`);
                } else {
                    console.log("All dependencies already installed.");
                }
            }

    if (didInstall) {
      toast.success('Addon installed', {
        description: `${addon.alias} was installed successfully.`,
      })
      onInstall(addon)
      isInstalled = true
      onOpenChange(false)
    }
  }

  async function handleUninstall(): Promise<void> {
    if (!isInstalled) {
      toast.info('Addon is not installed.')
      return
    }

    let didUninstall = false
    try {
      toast.info(`Uninstalling ${addon.alias}...`)
      didUninstall = await LocalAddonService.UninstallAddon(addon.name)
    } catch (e: any) {
      toast.error(`Failed to uninstall ${addon.alias}: ${e?.message || e}`)
      console.error('Uninstall error:', e)
    }

    if (didUninstall) {
      toast.success('Addon uninstalled', {
        description: `${addon.alias} was uninstalled successfully.`,
      })
      onUninstall(addon)
      isInstalled = false
      onOpenChange(false)
    }
  }

  async function getMyRating() {
    if (!isAuthenticated()) return
    try {
      const rateResponse = await apiClient.get(`/addon/${addon.name}/my-rating`)
      if (rateResponse.status === 200) {
        const r = await rateResponse.json()
        rating = r.data.rating
      } else if (rateResponse.status !== 404) {
        console.warn(`Failed to fetch rating: Status ${rateResponse.status}`)
      }
    } catch (e) {
      console.error('Failed to fetch rating', e)
    }
  }

  async function handleRating(r: number) {
    if (!isAuthenticated()) {
      toast.info('Please log in to rate addons.')
      return
    }
    if (rating === r) {
      try {
        const response = await fetch(`/addon/${addon.name}/rate`, {
          method: 'DELETE',
        })

        if (!response.ok) {
          const errorText = await response.text()
          toast.error(`Could not remove rating: ${response.status} ${errorText || ''}`.trim())
          console.error('Error removing rating:', response.status, errorText)
          return
        }
        rating = 0
        toast.success('Rating removed')
      } catch (e) {
        toast.error('Could not remove rating, request failed')
        console.error('Error removing rating:', e)
      }
    } else {
      try {
        const response = await apiClient.post(`/addon/${addon.name}/rate`, {
          is_like: r === 1,
        })

        if (!response.ok) {
          const errorText = await response.text()
          toast.error(`Could not rate addon: ${response.status} ${errorText || ''}`.trim())
          console.error('Error submitting rating:', response.status, errorText)
          return
        }

        rating = r

        toast.success('Addon rated', {
          description: r === 1 ? `Liked ${addon.alias}` : `Disliked ${addon.alias}`,
        })
      } catch (e) {
        toast.error('Could not rate addon, request failed')
        console.error('Error submitting rating:', e)
      }
    }
  }

  function getTabs() {
    const baseTabs = [
      {value: 'description', label: 'Description'},
      {value: 'changelog', label: 'Changelog'},
    ]
    if (dependencies.length > 0) {
      baseTabs.push({value: 'dependencies', label: `Dependencies (${dependencies.length})`})
    }
    if (addon.kofi) {
      baseTabs.push({value: 'kofi', label: 'Support Author'})
    }
    return baseTabs
  }

  // Click handler for dependency items
  function handleDependencyClick(dependencyManifest: AddonManifest) {
    console.log('Clicked dependency:', dependencyManifest.alias)
    // Call the callback prop passed from the parent
    onViewDependency(dependencyManifest)
  }
</script>

<Dialog.Root bind:open={open} onOpenChange={onOpenChange}>
  <Dialog.Content class="sm:max-w-[70%] lg:max-w-[60%] max-h-[90svh] flex flex-col p-0">
    {#if hasBanner}
      <div class="relative w-full h-48 overflow-hidden shrink-0 rounded-t-lg border-b dark:border-white/10">
        <img
          class="absolute top-0 left-0 w-full h-full object-cover"
          src={banner}
          alt="{addon.alias} Banner"
        />
        <div class="absolute inset-0 bg-gradient-to-t from-black/75 via-black/50 to-black/5"></div>
        <div class="absolute bottom-0 left-0 right-0 p-6 pb-4 pt-16 text-white bg-gradient-to-t from-black/60 to-transparent">
          <Dialog.Title class="text-2xl font-semibold mb-1">{addon.alias}</Dialog.Title>
          <div class="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm mb-3 text-white/80">
            <span class="inline-flex items-center gap-1">
              <User class="w-3.5 h-3.5"/>
              <span class="font-normal text-white/90">{addon.author}</span>
            </span>
            {#if release}
              <span class="inline-flex items-center gap-x-1.5">
                <Badge variant="secondary"
                  class="inline-flex items-center gap-1 bg-white/20 text-white border-white/30">
                  <Tag class="w-3 h-3"/>
                  {release.tag_name}
                </Badge>
                <span class="inline-flex items-center gap-1 text-xs text-white/70">
                  <CalendarDays class="w-3 h-3"/>
                  {formatToLocalTime(release.published_at, false)}
                </span>
              </span>
            {:else}
              <Badge variant="outline" class="bg-white/20 text-white border-white/30">No Release</Badge>
            {/if}
          </div>
          <div class="flex flex-wrap gap-x-2 gap-y-2 text-sm">
            <Button
              variant="outline"
              size="sm"
              class="flex items-center gap-1 border-white/50 text-white hover:bg-white/10"
              onclick={() => Browser.OpenURL(`https://github.com/${addon.repo}`)}
            >
              <GithubIcon class="w-4 h-4"/>
              View Code
            </Button>
            <Button
              variant="outline"
              size="sm"
              class="flex items-center gap-1 border-white/50 text-white hover:bg-white/10"
              onclick={() => Browser.OpenURL(`https://github.com/${addon.repo}/issues/new`)}
            >
              <BugIcon class="w-4 h-4"/>
              Report Issue
            </Button>
          </div>
        </div>
      </div>
    {:else}
      <Dialog.Header class="p-6 pb-4 shrink-0 border-b">
        <Dialog.Title class="text-2xl font-semibold mb-1">{addon.alias}</Dialog.Title>
        <div class="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm mb-3 text-muted-foreground">
          <span class="inline-flex items-center gap-1">
            <User class="w-3.5 h-3.5"/>
            <span class="font-normal text-foreground/90">{addon.author}</span>
          </span>
          {#if release}
            <span class="inline-flex items-center gap-x-1.5">
              <Badge variant="secondary" class="inline-flex items-center gap-1">
                <Tag class="w-3 h-3"/>
                {release.tag_name}
              </Badge>
              <span class="inline-flex items-center gap-1 text-xs text-muted-foreground">
                <CalendarDays class="w-3 h-3"/>
                {formatToLocalTime(release.published_at, false)}
              </span>
            </span>
          {:else}
            <Badge variant="outline">No Release</Badge>
          {/if}
        </div>
        <div class="flex flex-wrap gap-x-2 gap-y-2 text-sm">
          <Button
            variant="outline"
            size="sm"
            class="flex items-center gap-1"
            onclick={() => Browser.OpenURL(`https://github.com/${addon.repo}`)}
          >
            <GithubIcon class="w-4 h-4"/>
            View Code
          </Button>
          <Button
            variant="outline"
            size="sm"
            class="flex items-center gap-1"
            onclick={() => Browser.OpenURL(`https://github.com/${addon.repo}/issues/new`)}
          >
            <BugIcon class="w-4 h-4"/>
            Report Issue
          </Button>
        </div>
      </Dialog.Header>
    {/if}

    {#if addon.warning}
      <div class="px-6 py-3 bg-yellow-100 dark:bg-yellow-900/30 border-y border-yellow-200 dark:border-yellow-800">
        <p class="text-sm text-yellow-800 dark:text-yellow-200 flex items-start gap-2">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 shrink-0 mt-0.5" viewBox="0 0 24 24"
            fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"
            stroke-linejoin="round">
            <path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"/>
            <line x1="12" y1="9" x2="12" y2="13"/>
            <line x1="12" y1="17" x2="12.01" y2="17"/>
          </svg>
          {addon.warning}
        </p>
      </div>
    {/if}

    <div class="flex-1 flex flex-col min-h-0 overflow-y-hidden px-6 pb-0">
      <Tabs.Root bind:value={currentTab} class="w-full flex flex-col min-h-0">
        <Tabs.List class="inline-flex items-center bg-transparent justify-start gap-x-4 w-full mb-4">
          {#each tabs as tab (tab.value)}
            <Tabs.Trigger
              value={tab.value}
              class="inline-flex items-center rounded-none justify-center whitespace-nowrap px-3 py-2 text-sm font-medium transition-all focus-visible:outline-none disabled:pointer-events-none disabled:opacity-50 border-b-2 border-transparent data-[state=active]:border-primary data-[state=active]:text-primary text-muted-foreground hover:text-primary"
            >
              {tab.label}
            </Tabs.Trigger>
          {/each}
        </Tabs.List>

        <div class="flex-1 min-h-0 overflow-y-auto pb-2 pr-2 -mr-2">
          <Tabs.Content value="description" class="max-w-none" style="transform: translateZ(0);">
            <RemoteAddonReadme {readme}/>
          </Tabs.Content>

                    <Tabs.Content
                            value="changelog"
                            class="text-sm"
                    >
                        {#if release}
                            <div class="inline-flex items-center gap-1.5 text-xs text-muted-foreground mb-4">
                                <CalendarDays class="w-3.5 h-3.5"/>
                                <span>Released on {formatToLocalTime(release.published_at, true)}</span>
                            </div>
                            <div class="border rounded-lg p-4 bg-card">
                                <div class="prose max-w-none text-sm text-foreground dark:text-foreground/90">
                                    {@html changelog}
                                </div>
                            </div>
                        {:else}
                            <div class="flex flex-col items-center justify-center h-full text-center text-muted-foreground pt-10">
                                <Package class="w-12 h-12 mb-4 opacity-50"/>
                                <p class="font-medium">{changelog}</p>
                                {#if changelog === 'No changelog provided.'}
                                    <p class="text-xs mt-1">The author hasn't provided release notes.</p>
                                {:else if changelog === 'Error loading changelog.'}
                                    <p class="text-xs mt-1 text-destructive">Could not fetch release details.</p>
                                {/if}
                            </div>
                        {/if}
                    </Tabs.Content>

                    {#if dependencies.length > 0}
                        <Tabs.Content value="dependencies">
                            <p class="mb-4 text-sm text-muted-foreground">This addon requires the following
                                dependencies (including transitive dependencies):</p>
                            <div class="space-y-3">
                                {#each dependencies as d (d.manifest.name)}
                                    <button
                                            class="flex flex-col w-full p-4 border rounded-lg bg-background hover:bg-muted/50 transition-colors text-left focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
                                            onclick={() => handleDependencyClick(d.manifest)}
                                            aria-label={`View details for ${d.manifest.alias}`}
                                    >
                                        <div class="flex justify-between items-start mb-1 gap-2">
                                            <span class="font-medium text-base flex-1 break-words">{d.manifest.alias}</span>
                                            {#if d.isInstalled}
                                                <Badge variant="secondary" class="text-xs whitespace-nowrap">Installed
                                                </Badge>
                                            {:else}
                                                <Badge variant="outline" class="text-xs whitespace-nowrap">Not
                                                    Installed
                                                </Badge>
                                            {/if}
                                        </div>
                                        <p class="text-sm text-muted-foreground mb-2">by {d.manifest.author}</p>
                                        <p class="text-sm text-foreground/80 line-clamp-2">
                                            {d.manifest.description || 'No description available.'}
                                        </p>
                                    </button>
                                {/each}
                            </div>
                        </Tabs.Content>
                    {/if}

                    {#if addon.kofi}
                        <Tabs.Content value="kofi">
                            <div class="flex flex-col items-center justify-center text-center space-y-4 p-4 border rounded-lg bg-secondary/30">
                                <Heart class="w-10 h-10 text-red-500"/>
                                <p class="text-sm text-muted-foreground">
                                    Enjoying {addon.alias}? Show your appreciation by buying {addon.author} a coffee!
                                </p>
                                <Button
                                        variant="default"
                                        onclick={() => Browser.OpenURL(`https://ko-fi.com/${addon.kofi}`)}
                                >
                                    <Heart class="w-4 h-4 mr-2"/>
                                    Support {addon.author} on Ko-fi
                                </Button>
                            </div>
                        </Tabs.Content>
                    {/if}
                </div>
              </div>
            {:else}
              <div class="flex flex-col items-center justify-center h-full text-center text-muted-foreground pt-10">
                <Package class="w-12 h-12 mb-4 opacity-50"/>
                <p class="font-medium">{changelog}</p>
                {#if changelog === 'No changelog provided.'}
                  <p class="text-xs mt-1">The author hasn't provided release notes.</p>
                {:else if changelog === 'Error loading changelog.'}
                  <p class="text-xs mt-1 text-destructive">Could not fetch release details.</p>
                {/if}
              </div>
            {/if}
          </Tabs.Content>

          {#if dependencies.length > 0}
            <Tabs.Content value="dependencies">
              <p class="mb-4 text-sm text-muted-foreground">This addon requires the following
                dependencies:</p>
              <div class="space-y-3">
                {#each dependencies as d (d.manifest.name)}
                  <button
                    class="flex flex-col w-full p-4 border rounded-lg bg-background hover:bg-muted/50 transition-colors text-left focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
                    onclick={() => handleDependencyClick(d.manifest)}
                    aria-label={`View details for ${d.manifest.alias}`}
                  >
                    <div class="flex justify-between items-start mb-1 gap-2">
                      <span class="font-medium text-base flex-1 break-words">{d.manifest.alias}</span>
                      {#if d.isInstalled}
                        <Badge variant="secondary" class="text-xs whitespace-nowrap">Installed
                        </Badge>
                      {:else}
                        <Badge variant="outline" class="text-xs whitespace-nowrap">Not
                          Installed
                        </Badge>
                      {/if}
                    </div>
                    <p class="text-sm text-muted-foreground mb-2">by {d.manifest.author}</p>
                    <p class="text-sm text-foreground/80 line-clamp-2">
                      {d.manifest.description || 'No description available.'}
                    </p>
                  </button>
                {/each}
              </div>
            </Tabs.Content>
          {/if}

          {#if addon.kofi}
            <Tabs.Content value="kofi">
              <div class="flex flex-col items-center justify-center text-center space-y-4 p-4 border rounded-lg bg-secondary/30">
                <Heart class="w-10 h-10 text-red-500"/>
                <p class="text-sm text-muted-foreground">
                  Enjoying {addon.alias}? Show your appreciation by buying {addon.author} a coffee!
                </p>
                <Button
                  variant="default"
                  onclick={() => Browser.OpenURL(`https://ko-fi.com/${addon.kofi}`)}
                >
                  <Heart class="w-4 h-4 mr-2"/>
                  Support {addon.author} on Ko-fi
                </Button>
              </div>
            </Tabs.Content>
          {/if}
        </div>
      </Tabs.Root>
    </div>

    <Dialog.Footer class="p-4 border-t shrink-0">
      <div class="flex justify-between items-center w-full gap-4">
        <div class="flex gap-1 items-center">
          {#if isAuthenticated()}
            <Tooltip.Provider>
              <Tooltip.Root delayDuration={100}>
                <Tooltip.Trigger>
                  <Button
                    variant="ghost"
                    size="icon"
                    class="h-8 w-9 transition-all duration-200 hover:scale-105 hover:bg-blue-100 dark:hover:bg-blue-900/30 {rating === 1 ? 'bg-blue-100 dark:bg-blue-900/30 border border-blue-500' : ''}"
                    onclick={() => handleRating(1)}
                    aria-label="Like addon"
                  >
                    <Like class="w-5 h-5 {rating === 1 ? 'text-blue-500' : 'text-muted-foreground group-hover:text-blue-500'}"/>
                  </Button>
                </Tooltip.Trigger>
                <Tooltip.Content>
                  <p>{rating === 1 ? 'Unlike' : 'Like'}</p>
                </Tooltip.Content>
              </Tooltip.Root>
            </Tooltip.Provider>
            <Tooltip.Provider>
              <Tooltip.Root delayDuration={100}>
                <Tooltip.Trigger>
                  <Button
                    variant="ghost"
                    size="icon"
                    class="h-8 w-9 transition-all duration-200 hover:scale-105 hover:bg-red-100 dark:hover:bg-red-900/30 {rating === -1 ? 'bg-red-100 dark:bg-red-900/30 border border-red-500' : ''}"
                    onclick={() => handleRating(-1)}
                    aria-label="Dislike addon"
                  >
                    <Dislike
                      class="w-5 h-5 {rating === -1 ? 'text-red-500' : 'text-muted-foreground group-hover:text-red-500'}"/>
                  </Button>
                </Tooltip.Trigger>
                <Tooltip.Content>
                  <p>{rating === -1 ? 'Remove Dislike' : 'Dislike'}</p>
                </Tooltip.Content>
              </Tooltip.Root>
            </Tooltip.Provider>
          {:else}
            <Tooltip.Provider>
              <Tooltip.Root delayDuration={100}>
                <Tooltip.Trigger class="cursor-not-allowed opacity-50">
                  <span class="flex items-center gap-1 p-2">
                    <Like class="w-5 h-5 text-muted-foreground"/>
                    <Dislike class="w-5 h-5 text-muted-foreground"/>
                  </span>
                </Tooltip.Trigger>
                <Tooltip.Content>
                  <p>Log in to rate addons</p>
                </Tooltip.Content>
              </Tooltip.Root>
            </Tooltip.Provider>
          {/if}
        </div>

        <Button
          variant={isInstalled ? 'destructive' : 'default'}
          onclick={isInstalled ? handleUninstall : handleInstall}
          disabled={!isInstalled && !release}
          class="min-w-[100px]"
          aria-label={isInstalled ? `Uninstall ${addon.alias}` : !release ? 'Addon not available for installation' : `Install ${addon.alias}`}
        >
          {#if isInstalled}
            <Trash2Icon class="w-4 h-4 mr-2"/>
            Uninstall
          {:else if !release}
            Not Available
          {:else}
            <DownloadIcon class="w-4 h-4 mr-2"/>
            Install
          {/if}
        </Button>
      </div>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>