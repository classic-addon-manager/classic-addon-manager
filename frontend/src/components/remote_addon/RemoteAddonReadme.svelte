<script lang="ts">
    let {readme}: { readme: string } = $props();
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

    /* Each paragraph needs some margin */
    #markdown-container :global(p) {
        @apply my-2;
    }
</style>

<div id="markdown-container" class="max-w-[95%] mx-auto">
    {@html readme}
</div>
