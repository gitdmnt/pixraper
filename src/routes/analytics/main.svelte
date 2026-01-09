<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { readTextFile } from "@tauri-apps/plugin-fs";

  import Button from "$lib/components/Button.svelte";

  import type { CsvRow } from "./type.d.ts";
  import RawCSV from "./rawCSV.svelte";
  import TagRanking from "./tagRanking/main.svelte";
  import CooccurrenceAnalyze from "./cooccurrenceAnalyze.svelte";

  let tab = 0;
  let headers: string[] = [];

  let rows: CsvRow[] = [];

  let errorMessage: string | null = null;

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
        // Read the selected file as text
        const content = await readTextFile(selectedPath);

        // Parse the CSV content
        const lines = content.trim().split("\n");
        if (lines.length > 0) {
          // First line as headers
          headers = lines[0].split(",").map((h) => h.trim());
          // Remaining lines as data rows
          rows = lines
            .slice(1)
            .filter((line) => line.trim() !== "")
            .map((line) => {
              //ID,Title,X Restrict,Tags,Create Date,AI Type,Width,Height,Bookmark Count,View Count
              const [
                id,
                title,
                isXRestricted,
                tags,
                userId,
                createDate,
                generatedByAI,
                width,
                height,
                bookmarkCount,
                viewCount,
              ] = line
                .split(/,(?=(?:(?:[^"]*"){2})*[^"]*$)/)
                .map((cell) => cell.replace(/^"|"$/g, "").trim());

              return {
                id: Number(id),
                title,
                isXRestricted: isXRestricted === "true",
                tags: (tags ?? "").split(";"),
                userId: Number(userId),
                createDate,
                generatedByAI: generatedByAI === "true",
                width: Number(width),
                height: Number(height),
                bookmarkCount: Number(bookmarkCount),
                viewCount: Number(viewCount),
              };
            });
        }
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
