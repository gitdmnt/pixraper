<script lang="ts">
  import TopAppBar from "$lib/components/TopAppBar.svelte";
  import Button from "$lib/components/Button.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { Temporal } from "@js-temporal/polyfill";

  let detailedMode = false;
  let scrapingOption = {
    tags: [] as string[],
    searchMode: "s_tag",
    scd: Temporal.Now.plainDateISO().toString(),
    ecd: Temporal.Now.plainDateISO().toString(),
  };

  // UI state
  let isRunning = false;
  let scrapedItems = 0;
  let totalItems = 0;
  $: progressPercent = totalItems
    ? Math.round((scrapedItems / totalItems) * 100)
    : 0;

  const addTag = (e: KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault();
      const input = e.target as HTMLInputElement;
      const newTag = input.value.trim();
      if (newTag && !scrapingOption.tags.includes(newTag)) {
        scrapingOption.tags = [...scrapingOption.tags, newTag];
        input.value = "";
      }
      console.log("Tags:", scrapingOption.tags);
    }
  };

  const removeTag = (tag: string) => {
    scrapingOption.tags = scrapingOption.tags.filter((t) => t !== tag);
    console.log("Tags:", scrapingOption.tags);
  };

  const startScraping = () => {
    isRunning = true;
    scrapedItems = 0;
    totalItems = 0; // backend will report actual totals; show indeterminate state until then

    if (detailedMode) {
      invoke("start_detailed_scraping", { scrapingOption })
        .then(() => {
          console.log("Detailed scraping started");
        })
        .catch((error) => {
          console.error("Error starting detailed scraping:", error);
          isRunning = false;
        });
    } else {
      invoke("start_rough_scraping", { scrapingOption })
        .then(() => {
          console.log("Scraping started");
        })
        .catch((error) => {
          console.error("Error starting scraping:", error);
          isRunning = false;
        });
    }
    console.log("Starting scraping with options:", scrapingOption);
  };

  const stopScraping = () => {
    invoke("stop_scraping")
      .then((message) => {
        console.log(message);
      })
      .finally(() => {
        isRunning = false;
      });
  };
</script>

<TopAppBar title="Scraping">
  <div slot="actions" class="flex items-center gap-3">
    <label class="flex items-center gap-2 text-sm text-muted">
      <input type="checkbox" bind:checked={detailedMode} />
      <span>Detailed</span>
    </label>
    <Button variant="outlined" onclick={startScraping} disabled={isRunning}
      >Start</Button
    >
    <Button variant="contained" onclick={stopScraping} disabled={!isRunning}
      >Stop</Button
    >
  </div>
</TopAppBar>

<div class="p-4 h-full flex flex-col gap-4">
  <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
    <!-- Left / Main -->
    <main class="md:col-span-2 flex flex-col gap-4">
      <div class="md-card p-4">
        <div class="flex items-center justify-between">
          <div>
            <div class="text-lg font-medium">Scraping Progress</div>
            <div class="text-sm text-muted">
              Status: {isRunning ? "Running" : "Idle"}
            </div>
          </div>
          <div class="text-sm text-muted">
            {scrapedItems} / {totalItems || "—"}
          </div>
        </div>

        <div class="mt-4">
          <div
            class="w-full h-3 rounded bg-(--md-surface-variant) overflow-hidden"
          >
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

      <div class="md-card p-4">
        <div class="text-lg font-medium mb-2">Queue / Recent Activity</div>
        <div class="text-sm text-muted">No activity yet.</div>
      </div>
    </main>

    <!-- Right / Controls -->
    <aside class="flex flex-col gap-4">
      <div class="md-card p-4">
        <div class="font-medium">Scraping Options</div>

        <div class="mt-3">
          <div class="text-sm mb-1">Search Mode</div>
          <select
            bind:value={scrapingOption.searchMode}
            class="md-select w-full"
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
                  onclick={() => removeTag(tag)}>×</button
                >
              </div>
            {/each}
            <input
              class="md-search-input"
              type="text"
              placeholder="Enter tag and press Enter"
              onkeydown={addTag}
            />
          </div>
        </div>

        <div class="mt-3 grid grid-cols-2 gap-2 items-center">
          <div>
            <div class="text-sm mb-1">Start Date</div>
            <input
              type="date"
              bind:value={scrapingOption.scd}
              class="md-select w-full"
            />
          </div>
          <div>
            <div class="text-sm mb-1">End Date</div>
            <input
              type="date"
              bind:value={scrapingOption.ecd}
              class="md-select w-full"
            />
          </div>
        </div>

        <div class="mt-4 flex gap-2">
          <Button
            variant="outlined"
            onclick={() => {
              scrapingOption = { ...scrapingOption, tags: [] };
            }}>Clear</Button
          >
          <Button variant="contained" onclick={startScraping}
            >Start Scrape</Button
          >
        </div>
      </div>

      <div class="md-card p-4 hidden md:block">
        <div class="font-medium">Tips</div>
        <div class="text-sm text-muted mt-2">
          Use tags to narrow the search. Use Detailed mode for deeper scraping.
        </div>
      </div>
    </aside>
  </div>
</div>
