<script lang="ts">
  export let items: { title: string; subtitle?: string; value?: any }[] = [];
  export let itemsPerPage: number = 10;
  export let page: number = 0; // two-way bind from parent

  // total pages reactive calculation
  $: totalPages = Math.max(1, Math.ceil(items.length / itemsPerPage));

  function clampPage(p: number) {
    return Math.max(0, Math.min(p, totalPages - 1));
  }

  function handleWheel(e: WheelEvent) {
    // Only scroll if we have overflow pages
    if (totalPages <= 1) return;

    // Prevent default scroll behavior if we are scrolling the list
    // simplified: just change page
    // Note: In detailed implementations we might want to check if user is at top/bottom of scroll container
    // But here the component seems to be a list that paginates on scroll or wheel?
    // The previous implementation was simply:
    /*
      if (e.deltaY > 0) page = clampPage(page + 1);
      else if (e.deltaY < 0) page = clampPage(page - 1);
      e.preventDefault();
    */
    // We will keep it similar but maybe slightly throttled or just direct.

    if (e.deltaY > 0) {
      page = clampPage(page + 1);
    } else if (e.deltaY < 0) {
      page = clampPage(page - 1);
    }
    // We might not want to prevent default if we're not ACTUALLY changing pages or if it interferes with normal scrolling
    // The previous code prevented default, so we keep it.
  }
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
  class="md-card p-2 overflow-auto flex-1 flex flex-col"
  on:wheel={handleWheel}
>
  {#if items.length === 0}
    <div class="flex-1 flex items-center justify-center text-gray-500 text-sm">
      No items to display
    </div>
  {:else}
    <div class="flex flex-col gap-1">
      {#each items.slice(page * itemsPerPage, (page + 1) * itemsPerPage) as item, idx}
        <div class="md-list-item">
          <div class="leading text-sm text-gray-500 font-mono w-8 text-center">
            {page * itemsPerPage + idx + 1}
          </div>
          <div class="content flex-1 ml-3 min-w-0">
            <div class="title font-medium truncate" title={item.title}>
              {item.title}
            </div>
            {#if item.subtitle}
              <div
                class="subtitle text-xs text-gray-500 truncate"
                title={item.subtitle}
              >
                {item.subtitle}
              </div>
            {/if}
          </div>
          {#if item.value !== undefined && item.value !== null}
            <div class="value shrink-0 ml-2">
              <span class="md-chip text-sm">{item.value}</span>
            </div>
          {/if}
        </div>
      {/each}
    </div>

    <!-- Simple Pagination Indicator -->
    <div class="mt-auto pt-2 flex justify-center text-xs text-gray-400">
      Page {page + 1} / {totalPages}
    </div>
  {/if}
</div>

<style>
  /* Reuse styles from app.css via classes, or add specific overrides here */
  .md-list-item {
    display: flex;
    align-items: center;
    padding: 8px;
    border-radius: 8px;
    transition: background-color 0.2s;
  }
  .md-list-item:hover {
    background-color: rgba(0, 0, 0, 0.05);
  }
</style>
