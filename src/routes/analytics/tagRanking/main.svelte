<script lang="ts">
  export let rows: any[] = [];

  import Button from "$lib/components/Button.svelte";

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

  let filteredTags: {
    tag: string;
    count: number;
    viewCount: number;
    bookmarkCount: number;
  }[] = [];
  let tableItems: any[];

  let page = 0;
  let itemsPerPage = 10;

  // Performance measurement: measure each step and expose recent timings
  let perfTimings: { name: string; ms: number }[] = [];
  function pushPerf(name: string, ms: number) {
    perfTimings.unshift({ name, ms });
    if (perfTimings.length > 20) perfTimings.length = 20;
    // keep console trace for quick inspection
    console.debug("perf", name, ms.toFixed(2) + "ms");
  }

  // UI helpers
  import TopAppBar from "$lib/components/TopAppBar.svelte";
  import TagList from "./TagList.svelte";
  import FiltersPanel from "./FiltersPanel.svelte";
  import OverviewCard from "./OverviewCard.svelte";

  $: {
    const t0 = performance.now();

    // 1. O(n); AIとかR-18とかをフィルター
    const filteredRows = rows.filter((row) => {
      if (!showAIGenerated && row.generatedByAI) return false;
      if (!showNotAIGenerated && !row.generatedByAI) return false;
      if (!showXRestricted && row.isXRestricted) return false;
      if (!showNotXRestricted && !row.isXRestricted) return false;
      return true;
    });
    const t1 = performance.now();

    // 2. O(n*m); タグごとの統計情報 (作品数、閲覧数、ブックマーク数など) を集計
    let tagStats = (() => {
      const worksCounts: Record<
        string,
        { count: number; viewCount: number; bookmarkCount: number }
      > = {};
      const query = SearchQuery.toLowerCase();

      for (const row of filteredRows) {
        for (const tag of row.tags) {
          if (query && !tag.toLowerCase().includes(query)) {
            continue;
          }

          const entry =
            worksCounts[tag] ||
            (worksCounts[tag] = { count: 0, viewCount: 0, bookmarkCount: 0 });
          entry.count += 1;
          entry.viewCount += row.viewCount;
          entry.bookmarkCount += row.bookmarkCount;
        }
      }

      return Object.entries(worksCounts).map(
        ([tag, { count, viewCount, bookmarkCount }]) => ({
          tag,
          count,
          viewCount,
          bookmarkCount,
        })
      );
    })();
    const t2 = performance.now();

    // 3. O(n log n); 統計情報の値に基いてタグをソート
    let sortedTags = (() => {
      const compareFn = (a: any, b: any) => {
        switch (weightedType) {
          case "bookmarkCount":
            return b.bookmarkCount - a.bookmarkCount;
          case "viewCount":
            return b.viewCount - a.viewCount;
          case "bookmarkPerWork":
            return b.bookmarkCount / b.count - a.bookmarkCount / a.count;
          case "viewPerWork":
            return b.viewCount / b.count - a.viewCount / a.count;
          case "bookmarkPerView":
            return (
              b.bookmarkCount / b.viewCount - a.bookmarkCount / a.viewCount
            );
          case "workCount":
          default:
            return b.count - a.count;
        }
      };

      return tagStats.sort(compareFn);
    })();
    const t3 = performance.now();

    // 4. O(n); 作品数カットオフでフィルター
    filteredTags = sortedTags.filter((tag) => {
      if (
        weightedType === "bookmarkPerView" ||
        weightedType === "bookmarkPerWork" ||
        weightedType === "viewPerWork"
      ) {
        return tag.count >= worksCountCutoff;
      }
      return true;
    });
    const t4 = performance.now();

    // record timings
    pushPerf("filter", t1 - t0);
    pushPerf("aggregate", t2 - t1);
    pushPerf("sort", t3 - t2);
    pushPerf("cutoff", t4 - t3);

    // optional: print a concise table
    console.table({
      filter: t1 - t0,
      aggregate: t2 - t1,
      sort: t3 - t2,
      cutoff: t4 - t3,
    });
  }

  $: tableItems = filteredTags.map((tag) => {
    let value = (() => {
      if (weightedType === "workCount") {
        return tag.count;
      } else if (weightedType === "bookmarkCount") {
        return tag.bookmarkCount;
      } else if (weightedType === "viewCount") {
        return tag.viewCount;
      } else if (weightedType === "bookmarkPerView") {
        return `${(tag.bookmarkCount / tag.viewCount).toFixed(
          2
        )} (${tag.bookmarkCount} bookmarks / ${tag.viewCount} views, n=${tag.count})`;
      } else if (weightedType === "bookmarkPerWork") {
        return `${(tag.bookmarkCount / tag.count).toFixed(
          2
        )} (${tag.bookmarkCount} bookmarks / ${tag.count} works, n=${tag.count})`;
      } else if (weightedType === "viewPerWork") {
        return `${(tag.viewCount / tag.count).toFixed(2)} (${tag.viewCount} views / ${
          tag.count
        } works, n=${tag.count})`;
      } else {
        return tag.count;
      }
    })();
    return [tag.tag, value];
  });

  // quick lookup map from tag -> stats for rendering subtitles
  $: tagMap = new Map(filteredTags.map((t) => [t.tag, t]));

  function fmt(n: any) {
    if (typeof n === "number") return n.toLocaleString();
    return n;
  }

  function setPage(p: number) {
    const maxPage = Math.max(
      0,
      Math.floor((tableItems.length - 1) / itemsPerPage)
    );
    const clamped = Math.max(0, Math.min(p, maxPage));
    page = clamped;
  }

  // keep page within range when items change
  $: {
    const maxPage = Math.max(
      0,
      Math.floor((tableItems.length - 1) / itemsPerPage)
    );
    if (page > maxPage) page = maxPage;
  }
</script>

<div class="w-full h-full flex flex-col gap-4">
  <TopAppBar title="Tag Ranking">
    <div slot="actions" class="flex items-center gap-3">
      <input
        id="SearchQuery"
        type="text"
        placeholder="Search tags…"
        bind:value={SearchQuery}
        class="md-search-input"
      />
      <select bind:value={weightedType} class="md-select">
        <option value="workCount">Work Count</option>
        <option value="bookmarkCount">Total Bookmark</option>
        <option value="viewCount">Total View</option>
        <option value="bookmarkPerWork">Bookmark per Works</option>
        <option value="viewPerWork">View per Works</option>
        <option value="bookmarkPerView">Bookmark percentage per View</option>
      </select>
      <Button variant="outlined" onclick={() => setPage(0)}>Reset</Button>
    </div>
  </TopAppBar>

  {#if filteredTags.length > 0}
    <div class="flex gap-4 items-start">
      <main class="flex-1 flex flex-col min-h-0 gap-3">
        <TagList {tableItems} {tagMap} {itemsPerPage} bind:page />

        <div class="flex items-center justify-between mt-2">
          <div class="text-sm">
            Showing {page * itemsPerPage + 1} - {Math.min(
              (page + 1) * itemsPerPage,
              tableItems.length
            )} of {tableItems.length}
          </div>
          <div class="flex gap-2">
            <Button
              variant="outlined"
              onclick={() => setPage(Math.max(0, page - 1))}>Previous</Button
            >
            <Button
              variant="contained"
              onclick={() =>
                setPage(
                  Math.min(
                    Math.floor((tableItems.length - 1) / itemsPerPage),
                    page + 1
                  )
                )}>Next</Button
            >
          </div>
        </div>
      </main>

      <aside class="flex flex-col gap-4">
        <FiltersPanel
          bind:showAIGenerated
          bind:showNotAIGenerated
          bind:showXRestricted
          bind:showNotXRestricted
        />

        <div class="md-card p-4 shrink-0 hidden md:flex flex-col gap-3">
          <div class="font-medium">Quick Controls</div>
          <div class="text-sm">Ranking Type</div>
          <select bind:value={weightedType} class="md-select">
            <option value="workCount">Work Count</option>
            <option value="bookmarkCount">Total Bookmark</option>
            <option value="viewCount">Total View</option>
            <option value="bookmarkPerWork">Bookmark per Works</option>
            <option value="viewPerWork">View per Works</option>
            <option value="bookmarkPerView">Bookmark percentage per View</option
            >
          </select>
          <div class="mt-auto text-xs text-muted">
            Tip: Use search to filter tags quickly.
          </div>
        </div>
        <OverviewCard
          rowsLength={rows.length}
          filteredTagsLength={filteredTags.length}
          {perfTimings}
        ></OverviewCard>
      </aside>
    </div>
  {:else}
    <div class="md-card p-6">
      <p class="text-center">No data available for tag ranking.</p>
    </div>
  {/if}
</div>

<style>
  /* Card base - use existing CSS variables for colors */
  .md-card {
    background: var(--md-surface);
    color: var(--md-on-surface);
    border-radius: 12px;
    box-shadow:
      0 1px 3px rgba(16, 24, 40, 0.06),
      0 1px 2px rgba(16, 24, 40, 0.02);
  }

  .md-search-input {
    background: var(--md-surface-variant, #f3f4f6);
    border-radius: 20px;
    padding: 8px 12px;
    border: 1px solid var(--md-outline);
    min-width: 220px;
  }

  .md-select {
    padding: 8px 10px;
    border-radius: 8px;
    border: 1px solid var(--md-outline);
    background: var(--md-surface);
  }

  .text-muted {
    color: var(--md-on-surface-variant, #6b7280);
  }

  /* old table styles removed */
</style>
