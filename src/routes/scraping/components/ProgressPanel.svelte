<script lang="ts">
  export let isRunning = false;
  export let scrapedItems = 0;
  export let totalItems = 0;
  $: progressPercent = totalItems
    ? Math.round((scrapedItems / totalItems) * 100)
    : 0;
</script>

<div class="md-card p-4">
  <div class="flex items-center justify-between">
    <div>
      <div class="text-lg font-medium">Scraping Progress</div>
      <div class="text-sm text-muted">
        Status: {isRunning ? "Running" : "Idle"}
      </div>
    </div>
    <div class="text-sm text-muted">{scrapedItems} / {totalItems || "—"}</div>
  </div>

  <div class="mt-4">
    <div class="w-full h-3 rounded bg-(--md-surface-variant) overflow-hidden">
      {#if isRunning && totalItems === 0}
        <div
          class="h-full bg-linear-to-r from-(--md-primary) to-(--md-primary) animate-pulse"
          style="width:100%"
        ></div>
      {:else}
        <div
          class="h-full bg-(--md-primary)"
          style="width:{progressPercent}%"
        ></div>
      {/if}
    </div>
    <div class="mt-2 text-xs text-muted">{progressPercent}%</div>
  </div>
</div>
