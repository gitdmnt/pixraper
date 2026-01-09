<script lang="ts">
  export let rowsLength: number = 0;
  export let filteredTagsLength: number = 0;
  export let perfTimings: { name: string; ms: number }[] = [];
</script>

<div class="md-card p-3">
  <div class="font-medium">Overview</div>
  <div class="text-sm text-muted">
    CSV rows: {rowsLength} · Filtered tags: {filteredTagsLength}
  </div>

  <div class="mt-2 text-xs">
    <div class="font-medium">Perf (recent)</div>
    <ul>
      {#each perfTimings.slice(0, 5) as p}
        <li>{p.name}: {p.ms.toFixed(2)} ms</li>
      {/each}
    </ul>
  </div>
</div>

<style>
  .text-muted {
    color: var(--md-on-surface-variant, #6b7280);
  }
</style>
