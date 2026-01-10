<script lang="ts">
  interface FiltersPanelProps {
    showAIGenerated: boolean;
    showNotAIGenerated: boolean;
    showXRestricted: boolean;
    showNotXRestricted: boolean;
    worksCountCutoff: number;
    searchQuery: string;
  }
  export let filter: FiltersPanelProps = {
    showAIGenerated: true,
    showNotAIGenerated: true,
    showXRestricted: true,
    showNotXRestricted: true,
    worksCountCutoff: 5,
    searchQuery: "",
  };
</script>

<aside class="md-card p-4 shrink-0 flex flex-col gap-4">
  <div class="text-sm font-medium">Filters</div>
  <div class="flex flex-col gap-2">
    <div class="text-xs font-medium">AI</div>
    <label class="flex items-center gap-2 md-switch"
      ><input type="checkbox" bind:checked={filter.showNotAIGenerated} />
      <span>Not AI</span></label
    >
    <label class="flex items-center gap-2 md-switch"
      ><input type="checkbox" bind:checked={filter.showAIGenerated} />
      <span>AI</span></label
    >
  </div>

  <div class="flex flex-col gap-2">
    <div class="text-xs font-medium">Content Rating</div>
    <label class="flex items-center gap-2 md-switch"
      ><input type="checkbox" bind:checked={filter.showNotXRestricted} />
      <span>Safe</span></label
    >
    <label class="flex items-center gap-2 md-switch"
      ><input type="checkbox" bind:checked={filter.showXRestricted} />
      <span>R-18</span></label
    >
  </div>

  <div class="flex flex-col gap-2 pt-2 border-t border-stone-200">
    <div class="text-xs font-medium">Min Works: {filter.worksCountCutoff}</div>
    <input
      type="range"
      min="0"
      max="50"
      class="accent-blue-600 cursor-pointer"
      bind:value={filter.worksCountCutoff}
    />
  </div>
</aside>

<style>
  .md-switch input[type="checkbox"] {
    width: 18px;
    height: 18px;
    accent-color: var(--md-primary);
  }
</style>
