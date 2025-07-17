<script lang="ts">
  import {Skeleton} from '$lib/components/ui/skeleton/index.js'
</script>

<div class="grid grid-cols-10 items-center justify-between bg-muted/50 aspect-video h-12 w-full rounded-lg">
  <div class="flex gap-3 col-span-4 items-center">
    <Skeleton class="ml-2 h-[38px] w-[38px]"/>
    <Skeleton class="h-[20px] w-[150px]"/>
  </div>
  <Skeleton class="col-span-2 h-[20px] w-[100px]"/>
  <Skeleton class="col-span-2 h-[20px] w-[100px]"/>
  <Skeleton class="col-span-1 h-[20px] w-[100px] ml-6"/>
</div>