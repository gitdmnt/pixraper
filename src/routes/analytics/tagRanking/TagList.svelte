<script lang="ts">
  export let tableItems: any[] = [];
  export let tagMap: Map<string, any> = new Map();
  export let itemsPerPage: number = 10;
  export let page: number = 0; // two-way bind from parent

  // total pages reactive calculation
  $: totalPages = Math.max(1, Math.ceil(tableItems.length / itemsPerPage));

  function clampPage(p: number) {
    return Math.max(0, Math.min(p, totalPages - 1));
  }

  function handleWheel(e: WheelEvent) {
    if (e.deltaY > 0) {
      // scroll down -> next page
      page = clampPage(page + 1);
    } else if (e.deltaY < 0) {
      // scroll up -> previous page
      page = clampPage(page - 1);
    }

    e.preventDefault();
  }
</script>

<div class="md-card p-2 overflow-auto flex-1" onwheel={handleWheel}>
  {#each tableItems.slice(page * itemsPerPage, (page + 1) * itemsPerPage) as item, idx}
    <div class="m3-list-item">
      <div class="leading text-sm text-muted">
        {page * itemsPerPage + idx + 1}
      </div>
      <div class="content flex-1 ml-3">
        <div class="title font-medium">{item[0]}</div>
        <div class="subtitle text-xs text-muted">
          {#if tagMap.get(item[0])}
            {`${tagMap.get(item[0])!.count.toLocaleString()} works · ${tagMap.get(item[0])!.viewCount.toLocaleString()} views · ${tagMap.get(item[0])!.bookmarkCount.toLocaleString()} bookmarks`}
          {/if}
        </div>
      </div>
      <div class="value">
        <span class="md-chip">{item[1]}</span>
      </div>
    </div>
  {/each}
</div>

<style>
  /* keep basic styles local to component */
  .m3-list-item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px;
    border-radius: 8px;
  }
  .m3-list-item:hover {
    background: rgba(0, 0, 0, 0.04);
  }
  .leading {
    width: 36px;
    text-align: center;
  }
  .subtitle {
    color: var(--md-on-surface-variant, #6b7280);
  }
  .md-chip {
    display: inline-block;
    padding: 6px 10px;
    border-radius: 999px;
    background: var(--md-primary);
    color: var(--md-on-primary);
    min-width: 56px;
    text-align: center;
  }
</style>
