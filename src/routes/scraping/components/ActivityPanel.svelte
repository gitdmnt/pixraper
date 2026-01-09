<script lang="ts">
  export let activities: string[] = [];
</script>

<div class="md-card p-4">
  <div class="text-lg font-medium mb-2">Queue / Recent Activity</div>
  {#if activities.length === 0}
    <div class="text-sm text-muted">No activity yet.</div>
  {:else}
    <ul class="text-sm text-muted list-disc pl-4">
      {#each activities as act}
        <li>{act}</li>
      {/each}
    </ul>
  {/if}
</div>
