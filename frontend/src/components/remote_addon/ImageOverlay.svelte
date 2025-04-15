<script lang="ts">
    import { Dialog as DialogPrimitive } from "bits-ui";
    import Cross2 from "svelte-radix/Cross2.svelte";
    import * as Dialog from "$lib/components/ui/dialog/index";

    let { 
        src, 
        alt, 
        open = $bindable(false),
        onClose = () => {}
    }: { 
        src: string, 
        alt: string, 
        open: boolean,
        onClose: () => void 
    } = $props();

    function onOpenChange(isOpen: boolean) {
        open = isOpen;
        if (!isOpen) {
            onClose();
        }
    }
</script>

<style>
    .content-wrapper {
        position: fixed;
        left: 50%;
        top: 50%;
        transform: translate(-50%, -50%);
        width: 95vw;
        height: 95vh;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .zoom-animation {
        transition: all 200ms cubic-bezier(0.4, 0, 0.2, 1);
        width: 100%;
        height: 100%;
        display: flex;
        align-items: center;
        justify-content: center;
    }
    
    .zoom-animation[data-state="closed"] {
        opacity: 0;
        transform: scale(0.5);
    }
    
    .zoom-animation[data-state="open"] {
        opacity: 1;
        transform: scale(1);
    }

    .fade-animation {
        transition: opacity 200ms cubic-bezier(0.4, 0, 0.2, 1);
    }
    
    .fade-animation[data-state="closed"] {
        opacity: 0;
    }
    
    .fade-animation[data-state="open"] {
        opacity: 1;
    }

    .image-container {
        position: relative;
        width: 100%;
        height: 100%;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .image-container img {
        max-width: 95vw;
        max-height: 95vh;
        width: auto;
        height: auto;
        object-fit: contain;
        flex: 1;
        min-width: 60vw;
        min-height: 60vh;
    }
</style>

<Dialog.Root {open} onOpenChange={onOpenChange}>
    <Dialog.Portal>
        <Dialog.Overlay class="fixed inset-0 z-50 bg-black/80 fade-animation" />
        <div class="content-wrapper z-50">
            <DialogPrimitive.Content
                class="!p-0 !gap-0 !bg-transparent !border-0 !shadow-none !w-auto !h-auto zoom-animation"
            >
                <div class="image-container">
                    <img {src} {alt} class="rounded-lg" />
                    <Dialog.Close class="absolute -top-4 -right-4 rounded-full bg-background p-2 text-foreground hover:bg-muted transition-colors shadow-lg">
                        <Cross2 class="size-4" />
                        <span class="sr-only">Close</span>
                    </Dialog.Close>
                </div>
            </DialogPrimitive.Content>
        </div>
    </Dialog.Portal>
</Dialog.Root> 