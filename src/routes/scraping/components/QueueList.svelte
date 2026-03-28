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

<div class="md-card p-4 flex flex-col gap-4">
  <div class="flex justify-between items-center">
    <div class="font-medium">Queue ({queue.length})</div>
    <Button variant="text" onclick={clearQueue} disabled={queue.length === 0}
      >Clear Queue</Button
    >
  </div>

  <div class="flex flex-col gap-2 max-h-60 overflow-y-auto pr-1">
    {#each queue as item, i}
      <div
        class="p-3 bg-surface-container rounded-lg flex justify-between items-center group"
      >
        <div class="flex flex-col gap-1">
          <div class="flex gap-2 flex-wrap">
            {#if item.tags.length > 0}
              {#each item.tags as tag}
                <span
                  class="bg-primary-container text-on-primary-container px-2 py-0.5 rounded-full text-xs font-medium"
                  >{tag}</span
                >
              {/each}
            {:else}
              <span class="text-outline text-sm italic">No tags</span>
            {/if}
          </div>
          <div class="text-xs text-outline">
            {item.searchMode} | {item.scd} ~ {item.ecd} | Detailed: {item.detailed
              ? "Yes"
              : "No"}
          </div>
        </div>
        <div class="flex items-center gap-2 shrink-0">
          <div class="text-xs text-outline font-mono">#{i + 1}</div>
          {#if removeQueueItem}
            <button
              class="opacity-0 group-hover:opacity-100 transition-opacity p-1 hover:bg-surface-dim rounded text-error cursor-pointer"
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
      <div class="text-center text-outline py-6 text-sm">Queue is empty</div>
    {/each}
  </div>
</div>
