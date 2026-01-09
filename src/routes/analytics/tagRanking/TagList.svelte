<script lang="ts">
  export let tableItems: any[] = [];
  export let tagMap: Map<string, any> = new Map();
  export let itemsPerPage: number = 10;
  export let page: number = 0; // two-way bind from parent
  export let itemHeight: number = 56;

  let listContainer: HTMLDivElement | null = null;
  let scrollRaf: number | null = null;
  let scrollEndTimeout: any = null;

  // update page when scrolling
  function handleScroll() {
    if (!listContainer) return;
    if (scrollRaf) cancelAnimationFrame(scrollRaf);
    scrollRaf = requestAnimationFrame(() => {
      const st = listContainer!.scrollTop;
      const firstIndex = Math.floor(st / itemHeight);
      const newPage = Math.max(
        0,
        Math.min(
          Math.floor((tableItems.length - 1) / itemsPerPage),
          Math.floor(firstIndex / itemsPerPage)
        )
      );
      if (newPage !== page) page = newPage;
    });

    if (scrollEndTimeout) clearTimeout(scrollEndTimeout);
    scrollEndTimeout = setTimeout(() => {
      if (!listContainer) return;
      const target = page * itemsPerPage * itemHeight;
      listContainer.scrollTo({ top: target, behavior: "smooth" });
    }, 150);
  }

  // when parent updates page, snap scroll
  $: if (listContainer) {
    const target = page * itemsPerPage * itemHeight;
    // small timeout to avoid interfering with user scroll
    listContainer.scrollTo({ top: target });
  }

  // expose a method-ish: not required because page is two-way bound
</script>

<div
  class="md-card p-2 overflow-auto flex-1"
  bind:this={listContainer}
  on:scroll={handleScroll}
>
  {#each tableItems.slice(page * itemsPerPage, (page + 1) * itemsPerPage) as item, idx}
    <div class="m3-list-item" style="height:{itemHeight}px;">
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
