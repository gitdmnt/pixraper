<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { invoke } from "@tauri-apps/api/core";

  import Button from "$lib/components/Button.svelte";

  import type { CsvRow } from "./type.d.ts";
  import RawCSV from "./rawCSV.svelte";
  import TagRanking from "./tagRanking/main.svelte";
  import CooccurrenceAnalyze from "./cooccurrenceAnalyze.svelte";

  let tab = 0;
  let loaded = false;

  let errorMessage: string | null = null;

  // Rust側から返ってくるデータ構造
  interface ItemRecord {
    id: number;
    title: string;
    xRestrict: boolean;
    tags: string[];
    userId: number;
    createDate: string;
    aiType: boolean;
    width: number;
    height: number;
    bookmarkCount: number | null;
    viewCount: number | null;
  }

  const selectAndParseCsv = async () => {
    errorMessage = null;
    loaded = false;

    try {
      // Show the file open dialog to select a CSV file
      const selectedPath = await open({
        multiple: false,
        filters: [
          {
            name: "CSV",
            extensions: ["csv"],
          },
        ],
      });

      if (typeof selectedPath === "string") {
        // Rust backend reads the CSV
        const data = await invoke<ItemRecord[]>("load_dataset", {
          path: selectedPath,
        });

        loaded = true;
      }
    } catch (err) {
      console.error(err);
      errorMessage =
        err instanceof Error ? err.message : "An unknown error occurred.";
    }
  };
</script>

<div class="flex flex-row w-full h-full gap-4 p-4 app-content">
  <aside class="md-card w-40 p-3 flex flex-col items-center">
    <div class="text-xs font-medium text-neutral-600 mb-2">Analytics Menu</div>
    <div class="flex flex-col gap-2">
      <div class="md-segment flex-col">
        <button onclick={() => (tab = 0)} class:active={tab === 0}
          >Tag Ranking</button
        >
        <button onclick={() => (tab = 1)} class:active={tab === 1}
          >Co-occurrence</button
        >
        <button onclick={() => (tab = -1)} class:active={tab === -1}
          >Raw CSV</button
        >
      </div>
    </div>

    <div class="mt-auto">
      <Button variant="contained" onclick={selectAndParseCsv}>Import CSV</Button
      >
    </div>
  </aside>

  <main class="flex flex-col flex-1 gap-4">
    {#if errorMessage}
      <div class="md-card p-4 text-sm text-red-600">Error: {errorMessage}</div>
    {/if}

    {#if loaded}
      <div class="md-card p-4 flex-1">
        {#if tab === 1}
          <CooccurrenceAnalyze />
        {:else if tab === 0}
          <TagRanking />
        {:else}
          <RawCSV />
        {/if}
      </div>
    {:else}
      <div class="md-card p-6 text-neutral-600">
        CSVファイルを選択してデータを表示してください。
      </div>
    {/if}
  </main>
</div>

<style>
</style>
