<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { readTextFile } from "@tauri-apps/plugin-fs";

  import type { CsvRow } from "./type.d.ts";
  import RawCSV from "./rawCSV.svelte";
  import TagRanking from "./tagRanking.svelte";

  let tab = 0;
  let headers: string[] = [];

  let rows: CsvRow[] = [];

  let errorMessage: string | null = null;

  async function selectAndParseCsv() {
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
  }
</script>

<div class="container">
  <div class="tabs">
    <button on:click={() => (tab = 0)} class:active={tab === 0}>
      Tag Ranking
    </button>
    <button on:click={() => (tab = -1)} class:active={tab === -1}>
      Raw CSV
    </button>
  </div>

  <div class="analytics-container">
    {#if errorMessage}
      <p class="error">Error: {errorMessage}</p>
    {/if}

    {#if rows.length > 0}
      {#if tab === 0}
        <TagRanking {rows} />
      {:else}
        <RawCSV {headers} {rows} />
      {/if}
    {:else}
      <p>CSVファイルを選択してデータを表示してください。</p>
    {/if}
  </div>

  <button class="select-csv-button" on:click={selectAndParseCsv}
    >CSVファイルを選択</button
  >
</div>

<style>
  .container {
    display: flex;
    flex-direction: column;
    width: 70dvw;
    height: 70dvh;
  }
  .tabs {
    display: flex;
    margin-bottom: -1px;
    > button {
      background-color: #f8f8f8;
      color: #666;
      &.active {
        border-bottom: none;
        color: #222;
        background-color: white;
        z-index: 1;
      }
      + button {
        border-left: none;
      }
    }
  }
  button {
    padding: 0.5rem 1rem;
    cursor: pointer;
    border: 1px solid #ccc;
  }
  .error {
    color: red;
  }
  .analytics-container {
    flex: 1;
    overflow: auto;
    background-color: white;
    border: 1px solid #ccc;
    display: flex;
    flex-direction: column;
  }
  .select-csv-button {
    margin-top: -1px;
  }
</style>
