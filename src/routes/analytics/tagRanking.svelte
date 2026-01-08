<script lang="ts">
  // @ts-ignore
  import VirtualList from "svelte-virtual-list/VirtualList.svelte";

  export let rows: any[] = [];

  // ユーザーが変更するフィルターやソートの条件
  let weightedType:
    | "workCount"
    | "bookmarkCount"
    | "viewCount"
    | "bookmarkPerWork"
    | "viewPerWork"
    | "bookmarkPerView"
    | undefined = "workCount";
  let worksCountCutoff = 5;
  let showAIGenerated = true;
  let showNotAIGenerated = true;
  let showXRestricted = true;
  let showNotXRestricted = true;
  let SearchQuery: string = "";

  // 1. 重い計算：フィルター条件が変更されたときにだけ実行
  $: filteredRows = rows.filter((row) => {
    if (!showAIGenerated && row.generatedByAI) return false;
    if (!showNotAIGenerated && !row.generatedByAI) return false;
    if (!showXRestricted && row.isXRestricted) return false;
    if (!showNotXRestricted && !row.isXRestricted) return false;
    return true;
  });

  // 2. 重い計算：フィルターされた行データや検索クエリが変わったときにだけ実行
  $: tagStats = (() => {
    const worksCounts: Record<
      string,
      { count: number; viewCount: number; bookmarkCount: number }
    > = {};
    const lowerCaseSearchQuery = SearchQuery.toLowerCase();

    for (const row of filteredRows) {
      for (const tag of row.tags) {
        if (SearchQuery && !tag.toLowerCase().includes(lowerCaseSearchQuery)) {
          continue;
        }

        worksCounts[tag] = worksCounts[tag] || {
          count: 0,
          viewCount: 0,
          bookmarkCount: 0,
        };
        worksCounts[tag].count += 1;
        worksCounts[tag].viewCount += row.viewCount;
        worksCounts[tag].bookmarkCount += row.bookmarkCount;
      }
    }
    return Object.entries(worksCounts).map(
      ([tag, { count, viewCount, bookmarkCount }]) => ({
        tag,
        count,
        viewCount,
        bookmarkCount,
        bookmarkPerWork: bookmarkCount / (count || 1),
        viewPerWork: viewCount / (count || 1),
        bookmarkPerView: (bookmarkCount / (viewCount || 1)) * 100,
      })
    );
  })();

  // 3. 中程度の計算：ソート順やタグの統計情報が変わったときにだけ実行
  $: sortedTags = (() => {
    const compareFn = (a: any, b: any) => {
      switch (weightedType) {
        case "bookmarkCount":
          return b.bookmarkCount - a.bookmarkCount;
        case "viewCount":
          return b.viewCount - a.viewCount;
        case "bookmarkPerWork":
          return b.bookmarkPerWork - a.bookmarkPerWork;
        case "viewPerWork":
          return b.viewPerWork - a.viewPerWork;
        case "bookmarkPerView":
          return b.bookmarkPerView - a.bookmarkPerView;
        case "workCount":
        default:
          return b.count - a.count;
      }
    };

    return [...tagStats].sort(compareFn);
  })();

  // 4. 軽い計算：ソート結果やカットオフ値が変わったときにだけ実行
  $: filteredTags = sortedTags.filter((tag) => {
    if (
      weightedType === "bookmarkPerView" ||
      weightedType === "bookmarkPerWork" ||
      weightedType === "viewPerWork"
    ) {
      return tag.count >= worksCountCutoff;
    }
    return true;
  });
</script>

<div class="container">
  <div>
    <div>
      <label for="weightedType">Ranking Type: </label>
      <select id="weightedType" bind:value={weightedType}>
        <option value="workCount">Work Count</option>
        <option value="bookmarkCount">Total Bookmark</option>
        <option value="viewCount">Total View</option>
        <option value="bookmarkPerWork">Bookmark per Works</option>
        <option value="viewPerWork">View per Works</option>
        <option value="bookmarkPerView">Bookmark percentage per View</option>
      </select>
    </div>
    <div>
      <span>AI Type:</span>
      <label>
        <input type="checkbox" bind:checked={showNotAIGenerated} />
        Not AI Generated
      </label>
      <label>
        <input type="checkbox" bind:checked={showAIGenerated} />
        AI Generated
      </label>
    </div>
    <div>
      <span>X-Restricted Type:</span>
      <label>
        <input type="checkbox" bind:checked={showNotXRestricted} />
        Safe
      </label>
      <label>
        <input type="checkbox" bind:checked={showXRestricted} />
        R-18
      </label>
    </div>
    <div>
      <label for="SearchQuery">Search Tag:</label>
      <input
        type="text"
        id="SearchQuery"
        placeholder="Enter tag to search"
        bind:value={SearchQuery}
      />
    </div>
    {#if weightedType === "bookmarkPerWork" || weightedType === "viewPerWork" || weightedType === "bookmarkPerView"}
      <div>
        <label for="bookMarkPerViewWorksCountCutoff"
          >Min Works Count for Bookmark per View:</label
        >
        <input type="number" bind:value={worksCountCutoff} min="0" step="1" />
      </div>
    {/if}
  </div>

  {#if rows.length > 0}
    <div class="virtual-table-container">
      <!-- ヘッダー -->
      <div class="virtual-table-header">
        <div class="th">Tag</div>
        <div class="th">
          {(() => {
            if (weightedType === "workCount") {
              return "Work Count";
            } else if (weightedType === "bookmarkCount") {
              return "Total Bookmark";
            } else if (weightedType === "viewCount") {
              return "Total View";
            } else if (weightedType === "bookmarkPerView") {
              return "Bookmark percentage per View";
            } else if (weightedType === "bookmarkPerWork") {
              return "Bookmark per Work";
            } else if (weightedType === "viewPerWork") {
              return "View per Work";
            } else {
              return "Count";
            }
          })()}
        </div>
      </div>

      <!-- 仮想リスト本体 -->
      <VirtualList items={filteredTags} let:item>
        <div class="virtual-table-row">
          <div class="td">{item.tag}</div>
          <div class="td">
            {(() => {
              if (weightedType === "workCount") {
                return item.count;
              } else if (weightedType === "bookmarkCount") {
                return item.bookmarkCount;
              } else if (weightedType === "viewCount") {
                return item.viewCount;
              } else if (weightedType === "bookmarkPerView") {
                return `${item.bookmarkPerView.toFixed(
                  2
                )} (${item.bookmarkCount} bookmarks / ${item.viewCount} views, n=${item.count})`;
              } else if (weightedType === "bookmarkPerWork") {
                return `${item.bookmarkPerWork.toFixed(
                  2
                )} (${item.bookmarkCount} bookmarks / ${item.count} works, n=${item.count})`;
              } else if (weightedType === "viewPerWork") {
                return `${item.viewPerWork.toFixed(2)} (${item.viewCount} views / ${
                  item.count
                } works, n=${item.count})`;
              } else {
                return item.count;
              }
            })()}
          </div>
        </div>
      </VirtualList>
    </div>
  {:else}
    <p>No data available for tag ranking.</p>
  {/if}
</div>

<style>
  .container {
    height: 100%;
    overflow: hidden;
    display: flex;
    flex-direction: column;
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
    grid-template-columns: 3fr 2fr; /* 列の幅を調整 (例: 3:2) */
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
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .th:first-child,
  .td:first-child {
    border-right: 1px solid #ccc;
  }
</style>
