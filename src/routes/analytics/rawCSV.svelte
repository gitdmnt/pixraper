<script lang="ts">
  // @ts-ignore
  import VirtualList from "svelte-virtual-list/VirtualList.svelte";

  export let headers: string[] = [];
  export let rows: any[] = [];
  console.log("RawCSV received rows:", rows[0]);
</script>

<div class="raw-csv-container">
  {#if rows.length > 0}
    <div class="virtual-table-container">
      <div class="virtual-table-header">
        {#each headers as header}
          <div class="th">{header}</div>
        {/each}
      </div>

      <VirtualList items={rows} let:item>
        <div class="virtual-table-row">
          <div class="td">{item.id}</div>
          <div class="td">{item.title}</div>
          <div class="td">{item.isXRestricted}</div>
          <div class="td">{item.tags.join(", ")}</div>
          <div class="td">{item.userId}</div>
          <div class="td">{item.createDate}</div>
          <div class="td">{item.generatedByAI}</div>
          <div class="td">{item.width}</div>
          <div class="td">{item.height}</div>
          <div class="td">{item.bookmarkCount}</div>
          <div class="td">{item.viewCount}</div>
        </div>
      </VirtualList>
    </div>
  {:else if headers.length > 0}
    <p>CSVファイルにデータ行がありません。</p>
  {/if}
</div>

<style>
  .raw-csv-container {
    height: 100%;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .virtual-table-container {
    flex: 1;
    min-height: 0;
    border: 1px solid #ccc;
    margin-top: 1rem;
    display: flex;
    flex-direction: column;
  }

  /* VirtualListコンポーネントはデフォルトでdivを生成するので、それをflexコンテナにする */
  :global(.virtual-table-container > div) {
    flex: 1;
  }

  .virtual-table-header,
  .virtual-table-row {
    display: grid;
    grid-template-columns: 2fr 2fr 1fr 4fr 1fr 2fr 1fr 1fr 1fr 1fr;
    border-bottom: 1px solid #eee;
  }

  .virtual-table-header {
    background-color: #f0f0f0;
    font-weight: bold;
    position: sticky;
    top: 0;
    z-index: 1;
  }

  .th,
  .td {
    padding: 0.5rem;
    white-space: wrap;
    overflow: hidden;
    text-overflow: clip;

    border-left: 1px solid #ccc;
  }

  .th:first-child,
  .td:first-child {
    border-left: none;
  }
</style>
