<script lang="ts">
  import Button from "$lib/components/Button.svelte";

  export let headers: string[] = [];
  export let rows: any[] = [];

  let page = 0;
  let itemsPerPage = 20;

  $: totalPages = Math.max(1, Math.ceil(rows.length / itemsPerPage));
  $: paginatedRows = rows.slice(page * itemsPerPage, (page + 1) * itemsPerPage);

  function setPage(p: number) {
    page = Math.max(0, Math.min(p, totalPages - 1));
  }

  function handleWheel(e: WheelEvent) {
    if (totalPages <= 1) return;
    if (e.deltaY > 0) {
      setPage(page + 1);
    } else if (e.deltaY < 0) {
      setPage(page - 1);
    }
  }
</script>

<div class="raw-csv-container gap-2" on:wheel={handleWheel}>
  {#if rows.length > 0}
    <div class="table-container md-card bg-white">
      <div class="table-header">
        {#each headers as header}
          <div class="th">{header}</div>
        {/each}
        {#if headers.length < 11}
          <!-- Fallback if headers are missing but we have 11 columns of data -->
          {#each Array(11 - headers.length) as _}
            <div class="th">Column</div>
          {/each}
        {/if}
      </div>

      <div class="table-body">
        {#each paginatedRows as item (item.id)}
          <div class="table-row hover:bg-gray-50">
            <div class="td" title={item.id}>{item.id}</div>
            <div class="td font-medium" title={item.title}>{item.title}</div>
            <div class="td">{item.isXRestricted ? "Yes" : "No"}</div>
            <div class="td text-xs text-gray-600" title={item.tags.join(", ")}>
              {item.tags.join(", ")}
            </div>
            <div class="td" title={item.userId}>{item.userId}</div>
            <div class="td" title={item.createDate}>{item.createDate}</div>
            <div class="td">{item.generatedByAI ? "Yes" : "No"}</div>
            <div class="td">{item.width}</div>
            <div class="td">{item.height}</div>
            <div class="td font-bold text-gray-700">{item.bookmarkCount}</div>
            <div class="td text-gray-700">{item.viewCount}</div>
          </div>
        {/each}
      </div>
    </div>

    <div class="flex items-center justify-between p-2">
      <div class="text-sm text-gray-500">
        Showing {page * itemsPerPage + 1} - {Math.min(
          (page + 1) * itemsPerPage,
          rows.length
        )} of {rows.length}
      </div>
      <div class="flex gap-2">
        <Button
          variant="outlined"
          onclick={() => setPage(page - 1)}
          disabled={page === 0}>Previous</Button
        >
        <Button
          variant="contained"
          onclick={() => setPage(page + 1)}
          disabled={page >= totalPages - 1}>Next</Button
        >
      </div>
    </div>
  {:else if headers.length > 0}
    <div class="p-4 text-center text-gray-500">
      CSVファイルにデータ行がありません。
    </div>
  {/if}
</div>

<style>
  .raw-csv-container {
    height: 100%;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .table-container {
    flex: 1;
    min-height: 0;
    overflow: auto;
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    position: relative;
  }

  /* Define grid layout for headers and rows */
  .table-header,
  .table-row {
    display: grid;
    /* 11 columns to match the data cells in the template */
    grid-template-columns:
      minmax(60px, 0.5fr) minmax(150px, 2fr) minmax(60px, 0.5fr)
      minmax(200px, 3fr) minmax(80px, 1fr) minmax(120px, 1.5fr) minmax(
        60px,
        0.5fr
      )
      minmax(60px, 0.5fr) minmax(60px, 0.5fr) minmax(80px, 0.8fr) minmax(80px, 0.8fr);
    width: max-content; /* Allow horizontal scrolling if content is wide */
    min-width: 100%;
  }

  .table-header {
    background-color: #f9fafb;
    position: sticky;
    top: 0;
    z-index: 10;
    border-bottom: 1px solid #e5e7eb;
    font-size: 0.75rem;
    font-weight: 600;
    color: #4b5563;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .table-row {
    border-bottom: 1px solid #f3f4f6;
    transition: background-color 0.15s;
  }

  .table-row:last-child {
    border-bottom: none;
  }

  .th,
  .td {
    padding: 12px 16px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    display: flex;
    align-items: center;
  }

  .td {
    font-size: 0.875rem;
    color: #1f2937;
  }

  .th {
    border-right: 1px solid #f3f4f6;
  }
  .td {
    border-right: 1px solid transparent; /* Keep layout consistent */
  }
</style>
