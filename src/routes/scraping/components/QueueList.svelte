<script lang="ts">
  import Button from "$lib/components/Button.svelte";

  interface ScrapingOption {
    id: string;
    tags: string[];
    searchMode: string;
    scd: string;
    ecd: string;
    detailed: boolean;
  }

  export let queue: ScrapingOption[] = [];
  export let clearQueue: (() => void) | undefined = undefined;
  export let removeQueueItem: ((id: string) => void) | undefined = undefined;
</script>

<div class="p-4 bg-surface-container-low rounded-lg mt-4">
  <div class="flex justify-between items-center mb-4">
    <h3 class="text-lg font-bold">Queue ({queue.length})</h3>
    <Button variant="text" onclick={clearQueue} disabled={queue.length === 0}
      >Clear Queue</Button
    >
  </div>

  <div class="space-y-2 max-h-60 overflow-y-auto pr-2">
    {#each queue as item, i}
      <div
        class="p-3 bg-surface-container rounded flex justify-between items-center group"
      >
        <div>
          <div class="font-bold flex gap-2 flex-wrap">
            {#if item.tags.length > 0}
              {#each item.tags as tag}
                <span
                  class="bg-primary-container text-on-primary-container px-2 py-0.5 rounded-full text-xs"
                  >{tag}</span
                >
              {/each}
            {:else}
              <span class="text-outline text-sm italic">No tags</span>
            {/if}
          </div>
          <div class="text-xs text-outline mt-1">
            {item.searchMode} | {item.scd} ~ {item.ecd} | Detailed: {item.detailed
              ? "Yes"
              : "No"}
          </div>
        </div>
        <div class="flex items-center gap-2">
          <div class="text-xs text-outline font-mono">#{i + 1}</div>
          {#if removeQueueItem}
            <button
              class="opacity-0 group-hover:opacity-100 transition-opacity p-1 hover:bg-surface-dim rounded text-error"
              onclick={() => removeQueueItem && removeQueueItem(item.id)}
              title="Remove from queue"
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                height="20"
                viewBox="0 -960 960 960"
                width="20"
                fill="currentColor"
              >
                <path
                  d="M280-120q-33 0-56.5-23.5T200-200v-520h-40v-80h200v-40h240v40h200v80h-40v520q0 33-23.5 56.5T680-120H280Zm400-600H280v520h400v-520ZM360-280h80v-360h-80v360Zm160 0h80v-360h-80v360ZM280-720v520-520Z"
                />
              </svg>
            </button>
          {/if}
        </div>
      </div>
    {:else}
      <div class="text-center text-outline py-4">Queue is empty</div>
    {/each}
  </div>
</div>
