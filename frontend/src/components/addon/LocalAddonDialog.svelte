<script lang="ts">
    import * as Dialog from "$lib/components/ui/dialog/index.js";
    import { addon as ad } from "../../../wailsjs/go/models";

    let {
        open = $bindable(),
        onOpenChange,
        addon,
    }: {
        open: boolean;
        onOpenChange: (open: boolean) => void;
        addon: ad.Addon;
    } = $props();
</script>

<Dialog.Root {open} {onOpenChange}>
    <Dialog.Content>
        <Dialog.Header>
            <Dialog.Title>{addon.displayName}</Dialog.Title>
            <Dialog.Description>
                This action cannot be undone. This will permanently delete your
                account and remove your data from our servers.
            </Dialog.Description>
        </Dialog.Header>
    </Dialog.Content>
</Dialog.Root>