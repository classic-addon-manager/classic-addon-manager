<script lang="ts">
  import DOMPurify from 'dompurify'
  import {ArrowUpCircle, CalendarDays, LoaderCircle, Package, Tag, User} from 'lucide-svelte'
  import {marked} from 'marked'

  import {formatToLocalTime} from '@/utils'
  import {Badge} from '$lib/components/ui/badge'
  import {Button} from '$lib/components/ui/button'
  import {ScrollArea} from '$lib/components/ui/scroll-area'
  import * as Sheet from '$lib/components/ui/sheet'
  import {type Addon, Release} from '$lib/wails'

  let {
    open = $bindable(),
    onOpenChange,
    addon,
    release
  }: {
    open: boolean
    onOpenChange: (open: boolean) => void
    addon: Addon | null
    release: Release | null
  } = $props()

  let changelog = $state('')
  let isUpdating = $state(false)

  $effect(() => {
    if (open && release?.body) {
      changelog = DOMPurify.sanitize(
        marked.parse(release.body, {async: false})
      )
    } else {
      changelog = ''
    }
  })
</script>

{#if addon && release}
  {#key addon.name}
    <Sheet.Root {open} {onOpenChange}>
      <Sheet.Content class="flex flex-col h-full">
        <div class="flex-none">
          <Sheet.Header class="mb-3">
            <span class="text-2xl font-semibold">{addon.alias}</span>

            {#if addon.description}
              <Sheet.Description class="text-sm text-muted-foreground"
              >{addon.description}</Sheet.Description>
            {/if}
          </Sheet.Header>
          <div
            class="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm mb-3 text-muted-foreground"
          >
            <span class="inline-flex items-center gap-1">
              <User class="w-3.5 h-3.5"/>
              <span class="font-normal text-foreground/90"
              >{addon.author}</span
              >
            </span>
            <span class="inline-flex items-center gap-x-1.5">
              <Badge
                variant="secondary"
                class="inline-flex items-center gap-1"
              >
                <Tag class="w-3 h-3"/>
                {release.tag_name}
              </Badge>
            </span>
          </div>
        </div>

        <div class="flex-1 overflow-hidden flex flex-col min-h-0">
          {#if changelog.length > 0}
            <div
              class="inline-flex items-center gap-1.5 text-xs text-muted-foreground mb-4 flex-none"
            >
              <CalendarDays class="w-3.5 h-3.5"/>
              <span
              >Released on {formatToLocalTime(
                release.published_at,
              )}</span
              >
            </div>
            <ScrollArea class="flex-1">
              <div class="border rounded-lg p-4 bg-card">
                <div id="markdown-container"
                  class="prose max-w-none text-sm text-foreground dark:text-foreground/90"
                >
                  {@html changelog}
                </div>
              </div>
            </ScrollArea>
          {:else}
            <div
              class="flex flex-col items-center justify-center h-full text-center text-muted-foreground pt-10"
            >
              <Package class="w-12 h-12 mb-4 opacity-50"/>
              <p class="font-medium">No changelog available</p>
              <p class="text-xs mt-1">
                The author hasn't provided release notes.
              </p>
            </div>
          {/if}
        </div>

        <Sheet.Footer class="w-full flex-none mt-4">
          {#if isUpdating}
            <Button
              type="button"
              variant="default"
              disabled
              class="w-full"
            >
              <LoaderCircle class="w-4 h-4 mr-2 animate-spin"/>
              Updating...
            </Button>
          {:else}
            <Button
              type="button"
              variant="default"
              onclick={() => {isUpdating = true}}
              class="w-full"
            >
              <ArrowUpCircle class="w-4 h-4 mr-2"/>
              Update to {release.tag_name}
            </Button>
          {/if}
        </Sheet.Footer>
      </Sheet.Content>
    </Sheet.Root>
  {/key}
{/if}

<style>
    /* Make links more noticable */
    #markdown-container :global(a) {
        @apply
        text-blue-400 hover:text-blue-500
        hover:underline
        transition-all
        ;
    }

    #markdown-container :global(strong) {
        @apply text-foreground;
    }
</style>