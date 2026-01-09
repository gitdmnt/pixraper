<script lang="ts">
  import TopAppBar from "$lib/components/TopAppBar.svelte";
  import Button from "$lib/components/Button.svelte";
  import ProgressPanel from "./components/ProgressPanel.svelte";
  import ActivityPanel from "./components/ActivityPanel.svelte";
  import OptionsPanel from "./components/OptionsPanel.svelte";
  import TipsPanel from "./components/TipsPanel.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { Temporal } from "@js-temporal/polyfill";

  let scrapingOption = {
    tags: [] as string[],
    searchMode: "s_tag",
    scd: Temporal.Now.plainDateISO().toString(),
    ecd: Temporal.Now.plainDateISO().toString(),
    detailed: false,
  };

  // UI state
  let isRunning = false;
  let scrapedItems = 0;
  let totalItems = 0;
  $: progressPercent = totalItems
    ? Math.round((scrapedItems / totalItems) * 100)
    : 0;

  const addTag = (tag: string) => {
    if (tag && !scrapingOption.tags.includes(tag)) {
      scrapingOption = {
        ...scrapingOption,
        tags: [...scrapingOption.tags, tag],
      };
    }
    console.log("Tags:", scrapingOption.tags);
  };

  const removeTag = (tag: string) => {
    scrapingOption = {
      ...scrapingOption,
      tags: scrapingOption.tags.filter((t) => t !== tag),
    };
    console.log("Tags:", scrapingOption.tags);
  };

  const addQueue = () => {
    console.log("Adding to queue:", scrapingOption);
    invoke("add_queue", { scrapingOption })
      .then(() => {
        console.log("Scraping option added to queue");
      })
      .catch((error) => {
        console.error("Error adding scraping option to queue:", error);
        isRunning = false;
      });
  };

  const clearQueue = () => {
    invoke("clear_queue")
      .then((message) => {
        console.log(message);
      })
      .catch((error) => {
        console.error("Error clearing scraping queue:", error);
      });
  };

  const startScraping = () => {
    isRunning = true;
    invoke("start_scraping")
      .then(() => {
        console.log("Scraping started");
        isRunning = true;
      })
      .catch((error) => {
        console.error("Error starting scraping:", error);
      });
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
      <ProgressPanel {isRunning} {scrapedItems} {totalItems} />
      <ActivityPanel activities={[]} />
    </main>

    <!-- Right / Controls -->
    <aside class="flex flex-col gap-4">
      <OptionsPanel
        {scrapingOption}
        {addTag}
        {removeTag}
        {clearQueue}
        {addQueue}
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
