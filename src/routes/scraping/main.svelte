<script lang="ts">
  import TopAppBar from "$lib/components/TopAppBar.svelte";
  import Button from "$lib/components/Button.svelte";
  import ProgressPanel from "./components/ProgressPanel.svelte";
  import ActivityPanel from "./components/ActivityPanel.svelte";
  import OptionsPanel from "./components/OptionsPanel.svelte";
  import TipsPanel from "./components/TipsPanel.svelte";
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

  const handleAddTag = (tag: string) => {
    if (tag && !scrapingOption.tags.includes(tag)) {
      scrapingOption = {
        ...scrapingOption,
        tags: [...scrapingOption.tags, tag],
      };
    }
    console.log("Tags:", scrapingOption.tags);
  };

  const handleRemoveTag = (tag: string) => {
    scrapingOption = {
      ...scrapingOption,
      tags: scrapingOption.tags.filter((t) => t !== tag),
    };
    console.log("Tags:", scrapingOption.tags);
  };

  const handleClearTags = () => {
    scrapingOption = { ...scrapingOption, tags: [] };
    console.log("Tags cleared");
  };

  const addScrapingQueue = () => {
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
    <Button variant="outlined" onclick={addScrapingQueue} disabled={isRunning}
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
      <ProgressPanel {isRunning} {scrapedItems} {totalItems} />
      <ActivityPanel activities={[]} />
    </main>

    <!-- Right / Controls -->
    <aside class="flex flex-col gap-4">
      <OptionsPanel
        {scrapingOption}
        addTag={handleAddTag}
        removeTag={handleRemoveTag}
        clearTags={handleClearTags}
        start={addScrapingQueue}
        update={(field: string, value: any) =>
          (scrapingOption = {
            ...scrapingOption,
            [field]: value,
          })}
      />
      <TipsPanel />
    </aside>
  </div>
</div>
