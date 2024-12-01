<script lang="ts">
    import "./app.css";
    import {Toaster} from "$lib/components/ui/sonner/index.js";
    import {onMount} from "svelte";
    import addons from "./addons";
    import UI from "./screens/UI.svelte";

    onMount(async () => {
        await addons.populateAddonStore();
    });

    // TODO: Add a way to reset addon settings file because it can contain parsing errors
</script>

<Toaster/>
<UI/>
