<script lang="ts">
    import DashboardScreen from "$screens/DashboardScreen.svelte";
    import NavbarItem from "./NavbarItem.svelte";
    import House from "lucide-svelte/icons/house";
</script>

<NavbarItem name="Dashboard" Icon={House} Screen={DashboardScreen} />
