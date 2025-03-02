<script lang="ts">
    import NavbarItem from "./NavbarItem.svelte";

    import Link from "lucide-svelte/icons/link";
    import {Browser} from "@wailsio/runtime"
</script>

<a
        href="##"
        onclick={() => Browser.OpenURL("https://aa-classic.com")}
>
    <NavbarItem name="AA Classic Website" Icon={Link} isActivatable={false}/>
</a>
