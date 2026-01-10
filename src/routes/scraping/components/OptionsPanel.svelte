<script lang="ts">
  import Button from "$lib/components/Button.svelte";

  export let scrapingOption: any;
  export let addTag: ((tag: string) => void) | undefined;
  export let removeTag: ((tag: string) => void) | undefined;
  export let clearQueue: (() => void) | undefined;
  export let addQueue: (() => void) | undefined;
  export let update: ((field: string, value: any) => void) | undefined;

  const updateField = (field: string, value: any) => {
    update?.(field, value);
  };

  const handleAddTag = (e: KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault();
      const input = e.target as HTMLInputElement;
      const newTag = input.value.trim();
      if (newTag) {
        addTag?.(newTag);
        input.value = "";
      }
    }
  };

  const handleRemoveTag = (tag: string) => {
    removeTag?.(tag);
  };

  const handleClearQueue = () => {
    clearQueue?.();
  };

  const handleAddQueue = () => {
    addQueue?.();
  };
</script>

<div class="md-card p-4">
  <div class="font-medium">Scraping Options</div>

  <div class="mt-3">
    <div class="text-sm mb-1">Search Mode</div>
    <select
      class="md-select w-full"
      value={scrapingOption.searchMode}
      onchange={(e) =>
        updateField("searchMode", (e.target as HTMLSelectElement).value)}
    >
      <option value="s_tag">Partially Matching Tags</option>
      <option value="s_tag_full">Full Tag Search</option>
      <option value="s_tc">Title and Caption Search</option>
    </select>
  </div>

  <div class="mt-3">
    <div class="text-sm mb-1">Tags to Scrape</div>
    <div class="flex flex-wrap gap-2">
      {#each scrapingOption.tags as tag}
        <div class="md-chip flex items-center gap-2">
          <span class="text-sm">{tag}</span>
          <button
            class="text-xs font-medium"
            onclick={() => handleRemoveTag(tag)}>×</button
          >
        </div>
      {/each}
      <input
        class="md-search-input"
        type="text"
        placeholder="Enter tag and press Enter"
        onkeydown={handleAddTag}
      />
    </div>
  </div>

  <div class="mt-3 grid grid-cols-2 gap-2 items-center">
    <div>
      <div class="text-sm mb-1">Start Date</div>
      <input
        type="date"
        class="md-select w-full"
        value={scrapingOption.scd}
        onchange={(e) =>
          updateField("scd", (e.target as HTMLInputElement).value)}
      />
    </div>
    <div>
      <div class="text-sm mb-1">End Date</div>
      <input
        type="date"
        class="md-select w-full"
        value={scrapingOption.ecd}
        onchange={(e) =>
          updateField("ecd", (e.target as HTMLInputElement).value)}
      />
    </div>
  </div>

  <label class="flex items-center gap-2 text-sm text-muted">
    <input
      type="checkbox"
      bind:checked={scrapingOption.detailed}
      onchange={(e) =>
        updateField("detailed", (e.target as HTMLInputElement).checked)}
    />
    <span>Detailed</span>
  </label>

  <label class="flex items-center gap-2 text-sm text-muted">
    <input
      type="checkbox"
      bind:checked={scrapingOption.isIllust}
      onchange={(e) =>
        updateField("isIllust", (e.target as HTMLInputElement).checked)}
    />
    <span>for Illustrations (else Novels)</span>
  </label>

  <div class="mt-4 flex gap-2">
    <Button variant="contained" onclick={handleAddQueue}>Add Queue</Button>
    <Button variant="outlined" onclick={handleClearQueue}>Clear</Button>
  </div>
</div>
