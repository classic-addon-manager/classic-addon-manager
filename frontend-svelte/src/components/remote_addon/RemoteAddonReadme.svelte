<script lang="ts">
    import ImageOverlay from "./ImageOverlay.svelte";
    let {readme}: { readme: string } = $props();
    
    let selectedImage: { src: string, alt: string } | null = $state(null);
    let isImageOverlayOpen: boolean = $state(false);

    function handleImageClick(event: MouseEvent) {
        const target = event.target as HTMLElement;
        if (target.tagName === 'IMG') {
            const img = target as HTMLImageElement;
            selectedImage = {
                src: img.src,
                alt: img.alt || 'Image'
            };
            isImageOverlayOpen = true;
        }
    }

    function handleOverlayClose() {
        isImageOverlayOpen = false;
        // Clear the selected image after the overlay is closed
        setTimeout(() => {
            selectedImage = null;
        }, 200); // Wait for the animation to finish
    }
</script>

<style>
    /* Make links more noticable */
    #markdown-container :global(a) {
        @apply
        text-blue-400 hover:text-blue-500
        hover:underline
        transition-all
        ;
    }

    /* All headers need a border and some padding */
    #markdown-container :global(h1),
    #markdown-container :global(h2),
    #markdown-container :global(h3),
    #markdown-container :global(h4),
    #markdown-container :global(h5),
    #markdown-container :global(h6) {
        @apply my-2 border-b border-gray-600 pb-2;
    }

    /* Headers need to be bigger and bold */
    #markdown-container :global(h1) {
        @apply text-3xl font-semibold;
    }

    #markdown-container :global(h2) {
        @apply text-2xl font-semibold;
    }

    #markdown-container :global(h3) {
        @apply text-xl font-semibold;
    }

    #markdown-container :global(h4) {
        @apply text-lg font-semibold;
    }

    #markdown-container :global(h5) {
        @apply text-base font-semibold;
    }

    #markdown-container :global(h6) {
        @apply text-sm font-bold;
    }

    /* Links containing images need margins */
    #markdown-container :global(a img) {
        @apply my-3;
    }

    /* Make images clickable */
    #markdown-container :global(img) {
        @apply 
        cursor-pointer 
        hover:opacity-90 
        transition-all
        hover:scale-105
        ;
        transition-duration: 400ms;
    }

    /* Code blocks need to be more distinct */
    #markdown-container :global(code) {
        @apply
        bg-gray-700
        py-0.5
        px-1
        overflow-x-auto
        rounded-sm
        font-mono
        ;
    }

    /* Styling for pre blocks */
    #markdown-container :global(pre) {
        @apply
        bg-neutral-900 /* Slightly darker background */
        p-4           /* More padding */
        my-4          /* Vertical margin */
        overflow-x-auto
        rounded-md    /* More rounded corners */
        font-mono
        text-muted-foreground
        ;
    }

    /* Reset code styling when inside pre */
    #markdown-container :global(pre code) {
      @apply
        bg-transparent /* Remove background */
        p-0           /* Remove padding */
        rounded-none  /* Remove rounded corners */
        ;
    }

    /* Each paragraph needs some margin */
    #markdown-container :global(p) {
        @apply my-2;
    }

    /* UL and OL need some margin */
    #markdown-container :global(ul),
    #markdown-container :global(ol) {
        @apply my-5;
    }

    /* UL needs decorations */
    #markdown-container :global(ul) {
        @apply list-disc list-inside;
    }

    /* OL needs decorations */
    #markdown-container :global(ol) {
        @apply list-decimal list-inside;
    }
</style>

<div id="markdown-container" class="max-w-[95%] mx-auto" on:click={handleImageClick}>
    {@html readme}
</div>

{#if selectedImage}
    <ImageOverlay 
        src={selectedImage.src}
        alt={selectedImage.alt}
        bind:open={isImageOverlayOpen}
        onClose={handleOverlayClose}
    />
{/if}

