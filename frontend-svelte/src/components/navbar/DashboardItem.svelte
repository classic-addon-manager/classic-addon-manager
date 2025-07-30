<script lang="ts">
  import House from 'lucide-svelte/icons/house'

  import {getUpdateCount} from '$atoms/addon.svelte'
  import DashboardScreen from '$screens/DashboardScreen.svelte'

  import NavbarItem from './NavbarItem.svelte'

  let updatesAvailableCount = $derived.by(() => getUpdateCount())
</script>

<NavbarItem
  name="Dashboard"
  Icon={House}
  Screen={DashboardScreen}
  badgeCount={updatesAvailableCount > 0 ? updatesAvailableCount : -1}
/>
