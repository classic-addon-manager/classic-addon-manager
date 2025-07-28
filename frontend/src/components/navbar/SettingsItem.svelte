<script lang="ts">
  import SettingsIcon from 'lucide-svelte/icons/settings-2'

  import SettingsScreen from '$screens/SettingsScreen.svelte'

  import NavbarItem from './NavbarItem.svelte'
</script>

<NavbarItem
  name="Settings"
  Icon={SettingsIcon}
  Screen={SettingsScreen}
/>
