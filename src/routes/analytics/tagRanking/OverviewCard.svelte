<script lang="ts">
  export let totalWorks: number = 0;
  export let totalBookmarks: number = 0;
  export let totalViews: number = 0;
  export let totalTags: number = 0;
</script>

<div class="md-card p-3">
  <div class="font-medium">Overview</div>
  <div class="text-sm text-muted">
    Total works: {totalWorks} · Total bookmarks: {totalBookmarks} · Total views:
    {totalViews} · Total tags: {totalTags}
  </div>
</div>

<style>
  .text-muted {
    color: var(--md-on-surface-variant, #6b7280);
  }
</style>
