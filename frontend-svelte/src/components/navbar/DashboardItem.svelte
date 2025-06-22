<script lang="ts">
    import DashboardScreen from "$screens/DashboardScreen.svelte";
    import NavbarItem from "./NavbarItem.svelte";
    import House from "lucide-svelte/icons/house";

    import { getUpdatesAvailableCount } from "$stores/AddonStore.svelte";

    let updatesAvailableCount = $derived.by(() => getUpdatesAvailableCount());
</script>

<NavbarItem
    name="Dashboard"
    Icon={House}
    Screen={DashboardScreen}
    badgeCount={updatesAvailableCount > 0 ? updatesAvailableCount : -1}
/>
