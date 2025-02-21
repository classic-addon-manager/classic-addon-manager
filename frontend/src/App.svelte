<script lang="ts">
    import "./app.css";
    import {Toaster} from "$lib/components/ui/sonner/index.js";
    import {onMount} from "svelte";
    import addons from "./addons";
    import UI from "./screens/UI.svelte";
    import {setVersion} from "$stores/ApplicationStore.svelte";
    import {
        GetVersion as GoGetVersion
    } from "../wailsjs/go//app/App";

    onMount(async () => {
        setVersion(await GoGetVersion())
        await addons.populateAddonStore();
    });
</script>

<Toaster closeButton/>
<UI/>
