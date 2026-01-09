<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import TopAppBar from "$lib/components/TopAppBar.svelte";
  import Button from "$lib/components/Button.svelte";
  import TagList from "$lib/components/TagList.svelte";
  import FiltersPanel from "./FiltersPanel.svelte";
  import OverviewCard from "./OverviewCard.svelte";

  // Parent still passes rows, we use it for stats only (or triggering re-fetch)
  export let rows: any[] = [];

  // Data from backend
  let filteredTags: {
    tag: string;
    count: number;
    viewCount: number;
    bookmarkCount: number;
  }[] = [];

  // Sort & Filters
  let weightedType:
    | "workCount"
    | "bookmarkCount"
    | "viewCount"
    | "bookmarkPerWork"
    | "viewPerWork"
    | "bookmarkPerView" = "workCount";

  let worksCountCutoff = 5;
  let showAIGenerated = true;
  let showNotAIGenerated = true;
  let showXRestricted = true;
  let showNotXRestricted = true;
  let SearchQuery: string = "";

  let isLoading = false;
  let error: string | null = null;
  let perfMs = 0;

  // Fetch logic
  const fetchData = async () => {
    isLoading = true;
    error = null;
    const t0 = performance.now();
    const filters = {
      showAiGenerated: showAIGenerated,
      showNotAiGenerated: showNotAIGenerated,
      showXRestricted: showXRestricted,
      showNotXRestricted: showNotXRestricted,
      searchQuery: SearchQuery.trim() === "" ? null : SearchQuery.trim(),
    };

    const res = await invoke<
      {
        tag: string;
        count: number;
        viewCount: number;
        bookmarkCount: number;
      }[]
    >("calculate_tag_ranking", {
      filters,
      sortKey: weightedType,
    })
      .then((r) => (filteredTags = r))
      .catch((e) => {
        console.error(e);
        // If error is "No dataset loaded", treat as empty or show error
        error = String(e);
      })
      .finally(() => {
        perfMs = performance.now() - t0;
        isLoading = false;
      });
  };

  // Watchers
  $: {
    // We depend on these variables for fetching
    const _ = [
      weightedType,
      showAIGenerated,
      showNotAIGenerated,
      showXRestricted,
      showNotXRestricted,
      SearchQuery,
      rows, // re-fetch if dataset changes
    ];
    if (typeof window !== "undefined") {
      // debounce slightly? or just run
      fetchData();
    }
  }

  // Client-side filtering for cutoff (backend doesn't support it yet)
  $: displayTags = filteredTags.filter((t) => t.count >= worksCountCutoff);

  // Map to Table Items (for TagList)
  $: tableItems = displayTags.map((tag) => {
    let value: number | string = 0;
    if (weightedType === "workCount") {
      value = tag.count;
    } else if (weightedType === "bookmarkCount") {
      value = tag.bookmarkCount;
    } else if (weightedType === "viewCount") {
      value = tag.viewCount;
    } else if (weightedType === "bookmarkPerView") {
      value = `${(tag.bookmarkCount / Math.max(1, tag.viewCount)).toFixed(2)} (${tag.bookmarkCount}/${tag.viewCount} views)`;
    } else if (weightedType === "bookmarkPerWork") {
      value = `${(tag.bookmarkCount / Math.max(1, tag.count)).toFixed(2)} (${tag.bookmarkCount}/${tag.count} works)`;
    } else if (weightedType === "viewPerWork") {
      value = `${(tag.viewCount / Math.max(1, tag.count)).toFixed(2)} (${tag.viewCount}/${tag.count} works)`;
    } else {
      value = tag.count;
    }

    return {
      title: tag.tag,
      subtitle: `${tag.count.toLocaleString()} works · ${tag.viewCount.toLocaleString()} views · ${tag.bookmarkCount.toLocaleString()} bookmarks`,
      value: value,
    };
  });

  // Pagination
  let page = 0;
  let itemsPerPage = 10;

  const setPage = (p: number) => {
    const maxPage = Math.max(
      0,
      Math.floor((tableItems.length - 1) / itemsPerPage)
    );
    page = Math.max(0, Math.min(p, maxPage));
  };

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
      <span class="text-xs text-muted whitespace-nowrap"
        >Fetch: {perfMs.toFixed(0)}ms</span
      >
      <input
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
    </div>
  </TopAppBar>

  <div class="flex gap-4 items-start flex-1 min-h-0">
    <main class="flex-1 flex flex-col min-h-0 gap-3 h-full">
      {#if isLoading && filteredTags.length === 0}
        <div class="flex items-center justify-center h-full">Loading...</div>
      {:else if error && !error.includes("No dataset")}
        <div class="p-4 bg-red-50 text-red-600 rounded">{error}</div>
      {:else if tableItems.length > 0}
        <TagList items={tableItems} {itemsPerPage} bind:page />

        <div class="flex items-center justify-between mt-2 px-2 pb-2">
          <div class="text-sm text-muted">
            Showing {page * itemsPerPage + 1} - {Math.min(
              (page + 1) * itemsPerPage,
              tableItems.length
            )} of {tableItems.length}
          </div>
          <div class="flex gap-2">
            <Button variant="outlined" onclick={() => setPage(page - 1)}
              >Previous</Button
            >
            <Button variant="contained" onclick={() => setPage(page + 1)}
              >Next</Button
            >
          </div>
        </div>
      {:else}
        <div class="md-card p-8 text-center text-muted">
          {#if error}
            {error}
          {:else}
            No data found. Upload a CSV file first?
          {/if}
        </div>
      {/if}
    </main>

    <aside class="flex flex-col gap-4 w-72 shrink-0 overflow-y-auto">
      <FiltersPanel
        bind:showAIGenerated
        bind:showNotAIGenerated
        bind:showXRestricted
        bind:showNotXRestricted
        bind:worksCountCutoff
      />
    </aside>
  </div>
</div>
