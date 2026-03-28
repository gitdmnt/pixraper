<script lang="ts">
  import TopAppBar from "$lib/components/TopAppBar.svelte";
  import TagList from "$lib/components/TagList.svelte";
  import FiltersPanel from "./components/FiltersPanel.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, tick } from "svelte";
  // 型定義
  interface TagCount {
    tag: string;
    count: number;
  }

  // 状態変数
  let searchQuery = "";
  let suggestedTags: string[] = [];
  let tagCounts: TagCount[] = [];
  let targetTag: string | null = null;

  interface Filter {
    showAIGenerated: boolean;
    showNotAIGenerated: boolean;
    showXRestricted: boolean;
    showNotXRestricted: boolean;
    worksCountCutoff: number;
    searchQuery: string;
  }

  let filter: Filter = {
    showAIGenerated: true,
    showNotAIGenerated: true,
    showXRestricted: true,
    showNotXRestricted: true,
    worksCountCutoff: 5,
    searchQuery: "",
  };

  let cooccurrenceResults: TagCount[] = [];

  // タグ一覧を取得
  const fetchAllTags = async () => {
    try {
      const entries = await invoke<TagCount[]>("get_all_tags");
      tagCounts = entries ?? [];
    } catch (e) {
      console.error("Failed to fetch all tags:", e);
      tagCounts = [];
    }
  };

  onMount(() => {
    fetchAllTags();
  });

  // 検索サジェスト（件数順で上位10）
  $: suggestedTags = (() => {
    const q = searchQuery.trim().toLowerCase();
    const names = tagCounts.map((entry) => entry.tag);
    const lookup = Object.fromEntries(tagCounts.map((e) => [e.tag, e.count]));
    let list = q ? names.filter((t) => t.toLowerCase().includes(q)) : names;
    list.sort((a, b) => (lookup[b] ?? 0) - (lookup[a] ?? 0));
    return list.slice(0, 10);
  })();

  let selectedIndex: number = -1;

  $: {
    if (!suggestedTags || suggestedTags.length === 0) selectedIndex = -1;
    else if (selectedIndex >= suggestedTags.length)
      selectedIndex = suggestedTags.length - 1;
  }

  const scrollToSuggestion = async (i: number) => {
    await tick();
    const el = document.getElementById(`suggest-${i}`);
    el?.scrollIntoView({ block: "nearest" });
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (!suggestedTags || suggestedTags.length === 0) return;

    if (e.key === "ArrowDown") {
      e.preventDefault();
      if (selectedIndex < 0) selectedIndex = 0;
      else
        selectedIndex = Math.min(selectedIndex + 1, suggestedTags.length - 1);
      scrollToSuggestion(selectedIndex);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      if (selectedIndex <= 0) selectedIndex = suggestedTags.length - 1;
      else selectedIndex = Math.max(selectedIndex - 1, 0);
      scrollToSuggestion(selectedIndex);
    } else if (e.key === "Enter") {
      if (selectedIndex >= 0 && selectedIndex < suggestedTags.length) {
        e.preventDefault();
        selectTag(suggestedTags[selectedIndex]);
      } else if (suggestedTags.length === 1) {
        e.preventDefault();
        selectTag(suggestedTags[0]);
      }
    } else if (e.key === "Escape") {
      selectedIndex = -1;
    }
  };

  // 共起分析結果取得
  let isLoading = false;
  let totalInSubset = 0;
  let analyzeTimer: ReturnType<typeof setTimeout> | null = null;

  const selectTag = (tag: string) => {
    targetTag = tag;
    searchQuery = tag;
    analyze(tag);
  };

  const analyze = async (tag: string): Promise<void> => {
    if (!tag) return;
    isLoading = true;
    try {
      const res = await invoke<{ counts: TagCount[]; total: number }>(
        "calculate_co_occurence",
        {
          filter,
          tag,
        }
      );
      cooccurrenceResults = res.counts;
      totalInSubset = res.total;
    } catch (e) {
      console.error("Failed to fetch co-occurrence:", e);
      cooccurrenceResults = [];
      totalInSubset = 0;
    } finally {
      isLoading = false;
    }
  };

  // filter が変更されたときはデバウンスして再分析
  $: if (targetTag) {
    if (analyzeTimer) clearTimeout(analyzeTimer);
    analyzeTimer = setTimeout(() => analyze(targetTag as string), 200);
  }

  // 共起分析のタグをクリックするとそのタグについて分析し直す
  const handleTagClick = (item: any) => {
    selectTag(item.title);
  };
</script>

<div class="h-full flex flex-col gap-4 overflow-hidden">
  <TopAppBar title="Tag Co-occurrence Analysis">
    <div slot="actions" class="flex flex-row gap-4 items-center">
      <div class="text-sm text-muted">
        選択中: <span class="font-bold text-(--md-primary)"
          >{targetTag || "なし"}</span
        >
        (母数: <span class="font-mono">{targetTag ? totalInSubset : 0}</span>件)
      </div>
      <div class="relative w-64">
        <input
          type="text"
          bind:value={searchQuery}
          onkeydown={handleKeyDown}
          placeholder="タグを入力して検索..."
          class="md-search-input w-full"
        />
        {#if searchQuery && targetTag !== searchQuery}
          <div
            class="absolute top-full left-0 w-full md-card border border-(--md-outline) mt-1 max-h-60 overflow-y-auto z-50"
          >
            {#each suggestedTags as tag, i}
              <button
                id={"suggest-" + i}
                class="w-full text-left px-3 py-2 text-sm transition-colors {i ===
                selectedIndex
                  ? 'bg-surface-container font-semibold'
                  : 'hover:bg-surface-container-low'}"
                onclick={() => selectTag(tag)}
                onmouseenter={() => (selectedIndex = i)}
                onmouseleave={() => (selectedIndex = -1)}
              >
                {tag}
                <span class="text-muted text-xs ml-1"
                  >({tagCounts.find((entry) => entry.tag === tag)?.count ||
                    0})</span
                >
              </button>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  </TopAppBar>

  <div class="h-full w-full flex flex-row gap-2">
    <main class="flex-1 flex flex-col min-h-0 md-card overflow-hidden">
      {#if targetTag && cooccurrenceResults.length > 0}
        <TagList
          items={cooccurrenceResults.map((e) => ({
            title: e.tag,
            count: e.count,
            subtitle: `共起率: ${((e.count / totalInSubset) * 100).toFixed(1)}%`,
            value: ((e.count / totalInSubset) * 100).toFixed(2) + "%",
          }))}
          onclick={handleTagClick}
        />
      {:else if targetTag}
        <div class="p-8 text-center text-muted text-sm">
          共起するタグが見つかりませんでした。
        </div>
      {:else}
        <div
          class="p-8 text-center text-muted flex flex-col items-center justify-center h-full gap-2"
        >
          <div class="text-3xl">🔍</div>
          <p class="text-sm">左上のボックスから分析したいタグを検索して選択してください。</p>
        </div>
      {/if}
    </main>

    <aside class="flex flex-col gap-4 w-72 shrink-0 overflow-y-auto">
      <FiltersPanel bind:filter />
    </aside>
  </div>
</div>
