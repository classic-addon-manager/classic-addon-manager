<script lang="ts">
    import { Badge } from "$lib/components/ui/badge";
    import { setActiveScreen } from "../../stores/ScreenStore.svelte";
    import {
        getActiveNavbar,
        setActiveNavbar,
    } from "../../stores/NavbarStore.svelte";

    let {
        name,
        Icon,
        badgeCount = -1,
        Screen,
        isActivatable = true,
    }: {
        name: string;
        Icon?: any;
        badgeCount?: number;
        Screen?: any;
        isActivatable?: boolean;
    } = $props();
    let isActive = $derived.by(() => getActiveNavbar() === name.toLowerCase());
</script>

<a
    href="##"
    class:inactive={!isActive}
    class:active={isActive}
    onclick={() => {
        if (!isActivatable) {
            return;
        }
        setActiveNavbar(name.toLowerCase());
        if (Screen != null) {
            setActiveScreen(Screen);
        }
    }}
>
    {#if Icon != null}
        <Icon class="h-4 w-4" />
    {/if}
    {name}
    {#if badgeCount >= 0}
        <Badge
            class="ml-auto flex h-6 w-6 shrink-0 items-center justify-center rounded-full"
        >
            {badgeCount}
        </Badge>
    {/if}
</a>

<style lang="postcss">
    .inactive {
        @apply text-muted-foreground hover:text-primary flex items-center gap-3 rounded-lg px-3 py-2 transition-all;
    }
    .active {
        @apply bg-muted text-primary hover:text-primary flex items-center gap-3 rounded-lg px-3 py-2 transition-all;
    }
</style>
