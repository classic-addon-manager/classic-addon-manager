<script lang="ts">
  import Blocks from 'lucide-svelte/icons/blocks'

  import BrowseAddonsScreen from '$screens/BrowseAddonsScreen.svelte'

  import NavbarItem from './NavbarItem.svelte'
</script>

<NavbarItem name="Addons" Icon={Blocks} Screen={BrowseAddonsScreen}/>
