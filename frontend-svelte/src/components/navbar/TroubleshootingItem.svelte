<script lang="ts">
    import Wrench from "lucide-svelte/icons/wrench";
    import NavbarItem from "./NavbarItem.svelte";
    import TroubleshootingScreen from "$screens/TroubleshootingScreen.svelte";
</script>

<NavbarItem name="Troubleshooting" Icon={Wrench} Screen={TroubleshootingScreen}/>
