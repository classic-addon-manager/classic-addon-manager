<script lang="ts">
  import {X} from 'lucide-svelte'
  import {Drawer as DrawerPrimitive} from 'vaul-svelte'

  import {Button} from '$lib/components/ui/button/index.js'
  import {cn} from '$lib/utils.js'

  import DrawerOverlay from './drawer-overlay.svelte'

  let {
    ref = $bindable(null),
    class: className,
    portalProps,
    children,
    ...restProps
  }: DrawerPrimitive.ContentProps & {
    portalProps?: DrawerPrimitive.PortalProps;
  } = $props()
</script>

<DrawerPrimitive.Portal {...portalProps}>
  <DrawerOverlay/>
  <DrawerPrimitive.Content
    bind:ref
    class={cn(
      'bg-background fixed inset-x-0 bottom-0 z-50 mt-24 flex h-auto flex-col rounded-t-[10px] border',
      className
    )}
    {...restProps}
  >
    <div class="relative flex items-center justify-center mt-4">
      <div class="bg-muted h-2 w-[100px] rounded-full"></div>
      <DrawerPrimitive.Close>
        <Button variant="ghost" size="icon" class="h-8 w-8 absolute right-4 z-50 -top-1">
          <X class="w-4 h-4"/>
          <span class="sr-only">Close</span>
        </Button>
      </DrawerPrimitive.Close>
    </div>
    {@render children?.()}
  </DrawerPrimitive.Content>
</DrawerPrimitive.Portal>
