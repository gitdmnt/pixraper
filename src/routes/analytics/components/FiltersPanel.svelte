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
    <div class="text-xs font-medium text-muted">AI</div>
    <label class="flex items-center gap-2 cursor-pointer">
      <input
        type="checkbox"
        bind:checked={filter.showNotAIGenerated}
        class="w-4 h-4 accent-(--md-primary)"
      />
      <span class="text-sm">Not AI</span>
    </label>
    <label class="flex items-center gap-2 cursor-pointer">
      <input
        type="checkbox"
        bind:checked={filter.showAIGenerated}
        class="w-4 h-4 accent-(--md-primary)"
      />
      <span class="text-sm">AI</span>
    </label>
  </div>

  <div class="flex flex-col gap-2">
    <div class="text-xs font-medium text-muted">Content Rating</div>
    <label class="flex items-center gap-2 cursor-pointer">
      <input
        type="checkbox"
        bind:checked={filter.showNotXRestricted}
        class="w-4 h-4 accent-(--md-primary)"
      />
      <span class="text-sm">Safe</span>
    </label>
    <label class="flex items-center gap-2 cursor-pointer">
      <input
        type="checkbox"
        bind:checked={filter.showXRestricted}
        class="w-4 h-4 accent-(--md-primary)"
      />
      <span class="text-sm">R-18</span>
    </label>
  </div>

  <div class="flex flex-col gap-2 pt-3 border-t border-(--md-outline)">
    <div class="text-xs font-medium text-muted">
      Min Works: {filter.worksCountCutoff}
    </div>
    <input
      type="range"
      min="0"
      max="50"
      class="accent-(--md-primary) cursor-pointer w-full"
      bind:value={filter.worksCountCutoff}
    />
  </div>
</aside>
