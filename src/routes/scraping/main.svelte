<script lang="ts">
  import TopAppBar from "$lib/components/TopAppBar.svelte";
  import Button from "$lib/components/Button.svelte";
  import ProgressPanel from "./components/ProgressPanel.svelte";
  import ActivityPanel from "./components/ActivityPanel.svelte";
  import OptionsPanel from "./components/OptionsPanel.svelte";
  import TipsPanel from "./components/TipsPanel.svelte";
  import QueueList from "./components/QueueList.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { Temporal } from "@js-temporal/polyfill";
  import { onMount, onDestroy } from "svelte";

  interface ScrapingOption {
    id: string;
    tags: string[];
    searchMode: string;
    scd: string;
    ecd: string;
    detailed: boolean;
    isIllust: boolean;
  }

  let scrapingOption: ScrapingOption = $state({
    id: crypto.randomUUID(),
    tags: [] as string[],
    searchMode: "s_tag",
    scd: Temporal.Now.plainDateISO().toString(),
    ecd: Temporal.Now.plainDateISO().toString(),
    detailed: false,
    isIllust: true,
  });

  // UI state
  let isRunning = $state(false);
  let scrapedItems = $state(0);
  let totalItems = $state(0);
  let queue: ScrapingOption[] = $state([]);
  let currentItem: string | null = $state(null);
  let lastError: string | null = $state(null);

  const progressPercent = $derived(
    totalItems ? Math.round((scrapedItems / totalItems) * 100) : 0
  );

  // Polling logic
  let pollInterval: number | undefined;

  interface ScrapingProgress {
    status: "Running" | "Stopped";
    total: number | null;
    current: number | null;
    error: string | null;
    current_item: string | null;
  }

  const fetchQueue = async () => {
    try {
      queue = await invoke<ScrapingOption[]>("get_queue");
    } catch (error) {
      console.error("Failed to fetch queue:", error);
    }
  };

  const startPolling = () => {
    if (pollInterval) return;
    isRunning = true;
    pollInterval = window.setInterval(pollProgress, 1000);
  };

  const stopPolling = () => {
    if (pollInterval) {
      clearInterval(pollInterval);
      pollInterval = undefined;
    }
  };

  const pollProgress = async () => {
    try {
      const progress = await invoke<ScrapingProgress>("get_progress");
      isRunning = progress.status === "Running";
      scrapedItems = progress.current || 0;
      totalItems = progress.total || 0;
      currentItem = progress.current_item ?? null;
      if (!isRunning) {
        lastError = progress.error ?? null;
      }

      console.log(
        `Polled progress: ${scrapedItems}/${totalItems} (${progress.status})`
      );
      // Always fetch queue during polling to see changes
      fetchQueue();
      if (!isRunning) {
        stopPolling();
      }
    } catch (error) {
      console.error("Failed to poll progress:", error);
      stopPolling();
    }
  };

  onMount(async () => {
    // Initial check
    try {
      fetchQueue();
      const progress = await invoke<ScrapingProgress>("get_progress");
      isRunning = progress.status === "Running";
      scrapedItems = progress.current ?? 0;
      totalItems = progress.total ?? 0;
      currentItem = progress.current_item ?? null;
      lastError = progress.error ?? null;

      if (isRunning) {
        startPolling();
      }
    } catch (error) {
      console.error("Failed to check initial progress:", error);
    }
  });

  onDestroy(() => {
    stopPolling();
  });

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
    const option = { ...scrapingOption, id: crypto.randomUUID() };
    console.log("Adding to queue:", option);
    invoke("add_queue", { option })
      .then(() => {
        console.log("Scraping option added to queue");
        fetchQueue();
      })
      .catch((error) => {
        console.error("Error adding scraping option to queue:", error);
        isRunning = false;
      });
  };

  const clearQueue = () => {
    invoke("clear_queue")
      .then(() => {
        console.log("Queue cleared");
        fetchQueue();
      })
      .catch((error) => {
        console.error("Error clearing scraping queue:", error);
      });
  };

  const removeQueueItem = (id: string) => {
    invoke("remove_queue_item", { id })
      .then(() => {
        fetchQueue();
      })
      .catch((error) => {
        console.error("Error removing item from queue:", error);
      });
  };

  const startScraping = () => {
    lastError = null;
    // isRunning will be updated by pollProgress
    startPolling();
    invoke("start_scraping")
      .then(() => {
        console.log("Scraping started");
      })
      .catch((error) => {
        console.error("Error starting scraping:", error);
      });
  };

  const stopScraping = () => {
    console.log("Stopping scraping...");
    invoke("stop_scraping")
      .then((message) => {
        console.log(message);
      })
      .catch((error) => {
        console.error("Error stopping scraping:", error);
      });
  };
</script>

<TopAppBar title="Scraping">
  <div slot="actions" class="flex items-center gap-3">
    <Button variant="outlined" onclick={startScraping} disabled={isRunning}
      >Start</Button
    >
    <Button variant="contained" onclick={stopScraping}>Stop</Button>
  </div>
</TopAppBar>

<div class="p-4 h-full flex flex-col gap-4">
  <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
    <!-- Left / Main -->
    <main class="md:col-span-2 flex flex-col gap-4">
      <ProgressPanel {isRunning} {scrapedItems} {totalItems} />

      {#if isRunning && currentItem}
        <div class="rounded-md bg-blue-50 border border-blue-200 px-4 py-2 text-sm text-blue-700">
          処理中: {currentItem}
        </div>
      {/if}

      {#if !isRunning && lastError}
        <div class="rounded-md bg-red-50 border border-red-300 px-4 py-3 text-sm text-red-700 flex flex-col gap-1">
          <span class="font-semibold">エラーが発生しました</span>
          <span>{lastError}</span>
          <span class="text-xs text-red-500">次の開始時にクリアされます</span>
        </div>
      {/if}

      <QueueList {queue} {clearQueue} {removeQueueItem} />
    </main>

    <!-- Right / Controls -->
    <aside class="flex flex-col gap-4">
      <OptionsPanel
        {scrapingOption}
        {addTag}
        {removeTag}
        {clearQueue}
        {addQueue}
        update={(field, value) =>
          (scrapingOption = {
            ...scrapingOption,
            [field]: value,
          })}
      />
      <TipsPanel />
    </aside>
  </div>
</div>
