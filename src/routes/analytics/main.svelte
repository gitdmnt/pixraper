<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { invoke } from "@tauri-apps/api/core";

  import Button from "$lib/components/Button.svelte";

  import type { CsvRow } from "./type.d.ts";
  import RawCSV from "./rawCSV.svelte";
  import TagRanking from "./tagRanking/main.svelte";
  import CooccurrenceAnalyze from "./cooccurrenceAnalyze.svelte";

  let tab = 0;
  let headers: string[] = [];

  let rows: CsvRow[] = [];

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
    headers = [];
    rows = [];

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

        // Static headers since we don't parse the CSV line by line in frontend anymore
        headers = [
          "ID",
          "Title",
          "X Restrict",
          "Tags",
          "User ID",
          "Create Date",
          "AI Type",
          "Width",
          "Height",
          "Bookmark Count",
          "View Count",
        ];

        // Map to frontend CsvRow type
        rows = data.map((item) => ({
          id: item.id,
          title: item.title,
          isXRestricted: item.xRestrict,
          tags: item.tags,
          userId: item.userId,
          createDate: item.createDate,
          generatedByAI: item.aiType,
          width: item.width,
          height: item.height,
          bookmarkCount: item.bookmarkCount ?? 0,
          viewCount: item.viewCount ?? 0,
        }));
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

    {#if rows.length > 0}
      <div class="md-card p-4 flex-1">
        {#if tab === 1}
          <CooccurrenceAnalyze {rows} />
        {:else if tab === 0}
          <TagRanking {rows} />
        {:else}
          <RawCSV {headers} {rows} />
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
