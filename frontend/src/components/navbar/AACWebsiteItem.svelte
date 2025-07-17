<script lang="ts">
  import {Browser} from '@wailsio/runtime'
  import Link from 'lucide-svelte/icons/link'

  import NavbarItem from './NavbarItem.svelte'
</script>

<a
  href="##"
  onclick={() => Browser.OpenURL('https://aa-classic.com')}
>
  <NavbarItem name="AA Classic Website" Icon={Link} isActivatable={false}/>
</a>
