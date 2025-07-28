<script lang="ts">
    import SettingsScreen from "$screens/SettingsScreen.svelte";
    import NavbarItem from "./NavbarItem.svelte";
    import SettingsIcon from "lucide-svelte/icons/settings-2";
</script>

<NavbarItem
    name="Settings"
    Icon={SettingsIcon}
    Screen={SettingsScreen}
/>
