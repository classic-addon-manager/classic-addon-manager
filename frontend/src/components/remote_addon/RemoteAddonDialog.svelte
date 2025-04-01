<script lang="ts">
    import * as Dialog from "$lib/components/ui/dialog/index";
    import * as Tabs from "$lib/components/ui/tabs/index";
    import * as Tooltip from "$lib/components/ui/tooltip/index";
    import {buttonVariants} from "$lib/components/ui/button/index";
    import type {AddonManifest, Release} from "$lib/wails";
    import {onMount} from "svelte";
    import addons from "../../addons";
    import {LocalAddonService, RemoteAddonService} from "$lib/wails";
    import {Browser} from "@wailsio/runtime";
    import {Button} from "$lib/components/ui/button";
    import {GithubIcon, BugIcon} from "lucide-svelte";
    import Like from "lucide-svelte/icons/thumbs-up";
    import Dislike from "lucide-svelte/icons/thumbs-down";
    import Heart from "lucide-svelte/icons/heart";
    import {apiClient} from "../../api";
    import {isAuthenticated} from "$stores/UserStore.svelte";
    import {toast} from "../../utils";
    import RemoteAddonReadme from "./RemoteAddonReadme.svelte";
    import DOMPurify from "dompurify";
    import {marked} from "marked";

    let {
        open = $bindable(),
        onOpenChange,
        onInstall,
        addon,
    }: {
        open: boolean;
        onOpenChange: (open: boolean) => void;
        onInstall: (installed: boolean) => void;
        addon: AddonManifest;
    } = $props();

    let release: Release | undefined = $state();
    let isInstalled = $state(false);
    let hasBanner = $state(false);
    let banner: string = $state("");
    let readme = $state("");
    let rating = $state(0);
    let tabs = $state(getTabs());
    let dependencies: AddonManifest[] = $state([]);

    onMount(async () => {
        isInstalled = await LocalAddonService.IsInstalled(addon.name);
        hasBanner = await checkForBanner();
        tabs = getTabs();
    });

    async function checkForBanner() {
        const response = await fetch(
            `https://raw.githubusercontent.com/${addon.repo}/${addon.branch}/banner.png`,
        );
        if (!response.ok) {
            return false;
        }

        const reader = response.body?.getReader();
        if (reader) {
            const chunks = [];
            let done = false;
            while (!done) {
                let result;
                try {
                    result = await reader.read();
                } catch (e) {
                    console.error("Error reading image:", e);
                    return false;
                }
                done = result.done;
                if (result.value) {
                    chunks.push(result.value);
                }
            }
            const blob = new Blob(chunks);
            banner = URL.createObjectURL(blob);
        }
        return true;
    }

    async function getReadme(open: boolean) {
        if (!open) return;
        const response = await fetch(
            `https://raw.githubusercontent.com/${addon.repo}/refs/heads/${addon.branch}/README.md`,
        );
        if (!response.ok) {
            return;
        }
        marked.setOptions({
            gfm: true,
            // breaks: true,
        });
        readme = DOMPurify.sanitize(await marked.parse(await response.text()));
    }

    async function getRelease(open: boolean) {
        if (!open) return;
        try {
            release = await RemoteAddonService.GetLatestRelease(addon.name);
        } catch (e) {
            toast.error(
                `Failed to fetch release information for ${addon.name}`,
            );
            console.error(e);
        }
    }

    // Whenever the dialog is opened, run these
    $effect(() => {
        getRelease(open);
        getMyRating(open);
        getReadme(open);
        getDependencies(open);
    });

    async function getDependencies(open: boolean) {
        if (!open) return;
        if (addon.dependencies && addon.dependencies.length > 0) {
            dependencies = await Promise.all(addon.dependencies.map(async (a) => {
                return await addons.getManifest(a);
            })).catch((e) => {
                toast.error("Failed to fetch dependencies");
                console.error("Failed to fetch dependencies", e);
                return [];
            });
        }
    }

    function formatToLocalTime(dateString: string): string {
        const date = new Date(dateString);
        const options: Intl.DateTimeFormatOptions = {
            hour: "2-digit",
            minute: "2-digit",
            day: "2-digit",
            month: "long",
            year: "numeric",
        };
        return date.toLocaleString(undefined, options).replace(",", "");
    }

    async function handleInstall(): Promise<void> {
        if (isInstalled) {
            toast.error("Addon is already installed");
            return;
        }
        let didInstall: boolean = false;
        try {
            didInstall = await addons.install(addon);
            if (!didInstall) {
                toast.error("Failed to install addon");
            }
        } catch (e) {
            if (e == "no release found") {
                toast.error(`No release found for ${addon.name}`);
            } else {
                toast.error("Failed to install addon");
            }
        }

        if (didInstall) {
            toast.success("Addon installed", {
                description: `${addon.alias} was installed`,
            });

            if (addon.dependencies && addon.dependencies.length > 0) {
                for (const d of addon.dependencies) {
                    try {
                        await addons.install(await addons.getManifest(d));
                    } catch (e) {
                        toast.error("Failed to install dependency", {
                            description: `Failed to install ${d}`,
                        });
                    }
                    toast.success("Dependency installed", {
                        description: `${d} was installed`,
                    });
                }
            }

            onInstall(didInstall);
            isInstalled = true;
            open = false;
        }
    }

    function getMyRating(open: boolean) {
        if (!open || !isAuthenticated()) return;
        apiClient
            .get(`/addon/${addon.name}/my-rating`)
            .then(async (rateResponse) => {
                if (rateResponse.status === 200) {
                    const r = await rateResponse.json();
                    rating = r.data.rating;
                }
            })
            .catch((e) => {
                console.error("Failed to fetch rating", e);
            });
    }

    async function handleRating(r: number) {
        if (rating === r || r === 0) {
            return;
        }
        const response = await apiClient.post(`/addon/${addon.name}/rate`, {
            is_like: r === 1,
        });

        if (response.status !== 200) {
            toast.error("Could not rate addon, try again later");
            return;
        }

        rating = r;

        if (r === 1) {
            toast.success("Addon rated", {
                description: `Liked ${addon.alias}`,
            });
        } else {
            toast.success("Addon rated", {
                description: `Disliked ${addon.alias}`,
            });
        }
    }

    function getTabs() {
        if (addon.kofi) {
            return [
                {value: "description", label: "Description"},
                {value: "changelog", label: "Changelog"},
                {value: "kofi", label: "Support Author"},
            ];
        }
        return [
            {value: "description", label: "Description"},
            {value: "changelog", label: "Changelog"},
        ];
    }
</script>

<Dialog.Root {open} {onOpenChange}>
    <Dialog.Content class="max-h-[90%] max-w-[60%]">
        <Dialog.Header>
            <Dialog.Title>
                <div class="flex">
                    <div class="flex-row">
                        {addon.alias}
                        <span class="text-muted-foreground"
                        >{release ? release.tag_name : ""}</span
                        >
                    </div>
                </div>

                <div class="flex-row mt-2">
                    <div class="flex gap-4">
                        <a
                                href="javascript: void(0);"
                                class="text-muted-foreground text-sm hover:text-blue-500 transition duration-300 ease-in-out"
                                onclick={() =>
                                Browser.OpenURL(
                                    `https://github.com/${addon.repo}`,
                                )}
                        >
                            <div class="flex items-center">
                                <GithubIcon class="w-4 h-4 mr-1"/>
                                View code
                            </div>
                        </a>
                        <a
                                href="javascript: void(0);"
                                class="text-muted-foreground text-sm hover:text-blue-500 transition duration-300 ease-in-out"
                                onclick={() =>
                                Browser.OpenURL(
                                    `https://github.com/${addon.repo}/issues/new`,
                                )}
                        >
                            <div class="flex items-center">
                                <BugIcon class="w-4 h-4 mr-1"/>
                                Report issue
                            </div>
                        </a>
                    </div>
                </div>
            </Dialog.Title>
        </Dialog.Header>
        <Tabs.Root value="description" class="w-full">
            <!-- So there was a weird issue causing grid-cols-2 to not work dynamically. This was the fix. -->
            {#if tabs.length === 2}
                <Tabs.List class="grid w-full grid-cols-2">
                    {#each getTabs() as tab}
                        <Tabs.Trigger value={tab.value}
                        >{tab.label}</Tabs.Trigger
                        >
                    {/each}
                </Tabs.List>
            {/if}

            {#if tabs.length === 3}
                <Tabs.List class="grid w-full grid-cols-3">
                    {#each getTabs() as tab}
                        <Tabs.Trigger value={tab.value}
                        >{tab.label}</Tabs.Trigger
                        >
                    {/each}
                </Tabs.List>
            {/if}
            <Tabs.Content value="description">
                <div
                        class="flex flex-1 flex-col max-h-[calc(100vh-47vh)] overflow-auto"
                >
                    {#if hasBanner}
                        <img
                                class="max-h-[40svh] aspect-video object-fill"
                                src={banner}
                                alt=""
                        />
                    {/if}
                    <div>
                        {#if readme === ""}
                            <p class="mt-4">{addon.description}</p>
                        {:else}
                            <RemoteAddonReadme {readme}/>
                        {/if}
                    </div>
                </div>
            </Tabs.Content>
            <Tabs.Content value="changelog" class="max-h-[50vh] overflow-auto">
                <p class="text-muted-foreground">
                    Released {formatToLocalTime(release?.published_at)}
                </p>
                <p class="whitespace-pre-wrap">
                    {release?.body || "No change log was provided by the addon"}
                </p>
            </Tabs.Content>

            <Tabs.Content value="kofi">
                <div class="flex flex-col items-center justify-center h-full p-6 text-center space-y-4">
                    <Heart class="w-12 h-12 text-red-500 mb-2" />
                    <p class="text-sm text-muted-foreground">
                        Enjoying {addon.alias}? Show your appreciation and support future development by buying {addon.author} a coffee!
                    </p>
                    <Button
                            variant="default"
                            onclick={() =>
                            Browser.OpenURL(`https://ko-fi.com/${addon.kofi}`)}
                    >
                        <Heart class="w-4 h-4 mr-2" />
                        Support {addon.author} on Ko-fi
                    </Button>
                </div>
            </Tabs.Content>
        </Tabs.Root>
        <div class="flex justify-end gap-5">
            {#if isAuthenticated()}
                <div class="flex gap-2 items-center">
                    <Button
                            class="mt-2 transition-all duration-200 hover:scale-110 {rating === 1 ? 'bg-blue-100 dark:bg-blue-900/30 border-blue-500' : ''}"
                            variant="outline"
                            onclick={() => handleRating(1)}
                    >
                        {#if rating === 1}
                            <Like class="w-6 text-blue-500 transition-colors"/>
                        {:else}
                            <Like class="w-6 transition-colors hover:text-blue-500"/>
                        {/if}
                    </Button>
                    <Button
                            class="mt-2 transition-all duration-200 hover:scale-110 {rating === -1 ? 'bg-red-100 dark:bg-red-900/30 border-red-500' : ''}"
                            variant="outline"
                            onclick={() => handleRating(-1)}
                    >
                        {#if rating === -1}
                            <Dislike class="w-6 text-red-500 transition-colors"/>
                        {:else}
                            <Dislike class="w-6 transition-colors hover:text-red-500"/>
                        {/if}
                    </Button>
                </div>
            {/if}
            {#if !isInstalled}
                {#if addon.dependencies && addon.dependencies.length > 0}
                    <Tooltip.Provider>
                        <Tooltip.Root delayDuration={0}>
                            <Tooltip.Trigger
                                    class={buttonVariants({ variant: "default" }) + " mt-2"}
                                    onclick={handleInstall}
                            >
                                Install
                                {#if addon.dependencies && addon.dependencies.length > 0}
                                    <span class="text-xs text-gray-600"
                                    >({addon.dependencies.length} dependencies)</span
                                    >
                                {/if}
                            </Tooltip.Trigger>
                            <Tooltip.Content class="bg-background text-white border">
                                <p class="mb-2">Installing this addon will also install the following addons</p>
                                {#each dependencies as d}
                                    <p><span class="text-foreground font-bold">{d.alias}</span> <span
                                            class="text-muted-foreground font-semibold">by {d.author}</span></p>
                                {/each}
                            </Tooltip.Content>
                        </Tooltip.Root>
                    </Tooltip.Provider>
                {:else}
                    <Button
                            class="mt-2"
                            variant="default"
                            onclick={handleInstall}
                    >
                        Install
                    </Button>
                {/if}
            {/if}
        </div>
    </Dialog.Content>
</Dialog.Root>
