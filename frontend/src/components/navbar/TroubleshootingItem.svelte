<script lang="ts">
  import Wrench from 'lucide-svelte/icons/wrench'

  import TroubleshootingScreen from '$screens/TroubleshootingScreen.svelte'

  import NavbarItem from './NavbarItem.svelte'
</script>

<NavbarItem name="Troubleshooting" Icon={Wrench} Screen={TroubleshootingScreen}/>
