<script lang="ts">
  import './app.css'
  import '@wailsio/runtime'

  import {onMount} from 'svelte'

  import {setVersion} from '$atoms/application.svelte'
  import {Toaster} from '$lib/components/ui/sonner/index.js'
  import {ApplicationService} from '$lib/wails'

  import addons from './addons'
  import UI from './screens/UI.svelte'

  onMount(async () => {
    setVersion(await ApplicationService.GetVersion())
    await addons.populateAddonStore()
  })
</script>

<Toaster closeButton/>
<UI/>
