<script lang="ts">
  import TopAppBar from "$lib/components/TopAppBar.svelte";
  import TagList from "$lib/components/TagList.svelte";
  import FiltersPanel from "./components/FiltersPanel.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  // 型定義
  interface TagStats {
    tag: string;
    count: number;
    _viewCount: number;
    _bookmarkCount: number;
  }

  // 状態変数
  let searchQuery = "";
  let suggestedTags: string[];
  let tagCounts: TagStats[] = [];
  let targetTag: string | null = null;
  let filter = {
    showAIGenerated: true,
    showNotAIGenerated: true,
    showXRestricted: true,
    showNotXRestricted: true,
    worksCountCutoff: 5,
    searchQuery: "",
  };
  let cooccurrenceResults: TagStats[] = [];

  // タグ一覧を取得
  const fetchAllTags = async () => {
    await invoke<TagStats[]>("get_all_tags")
      .then((entries) => {
        tagCounts = entries;
      })
      .catch((e) => {
        console.error("Failed to fetch all tags:", e);
        tagCounts = [];
      });
  };

  onMount(() => {
    fetchAllTags();
  });

  // 検索サジェスト
  $: suggestedTags = searchQuery
    ? tagCounts
        .map((entry) => entry.tag)
        .filter((t) => t.toLowerCase().includes(searchQuery.toLowerCase()))
        .slice(0, 10)
    : tagCounts.map((entry) => entry.tag).slice(0, 10);

  let selectedIndex: number = -1;

  $: if (!suggestedTags || suggestedTags.length === 0) selectedIndex = -1;
  $: if (selectedIndex >= 0 && selectedIndex >= suggestedTags.length)
    selectedIndex = suggestedTags.length - 1;

  const handleKeyDown = (e: KeyboardEvent) => {
    if (!suggestedTags || suggestedTags.length === 0) return;

    if (e.key === "ArrowDown") {
      e.preventDefault();
      if (selectedIndex < 0) selectedIndex = 0;
      else
        selectedIndex = Math.min(selectedIndex + 1, suggestedTags.length - 1);
      setTimeout(() => {
        const el = document.getElementById(`suggest-${selectedIndex}`);
        el?.scrollIntoView({ block: "nearest" });
      }, 0);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      if (selectedIndex <= 0) selectedIndex = suggestedTags.length - 1;
      else selectedIndex = Math.max(selectedIndex - 1, 0);
      setTimeout(() => {
        const el = document.getElementById(`suggest-${selectedIndex}`);
        el?.scrollIntoView({ block: "nearest" });
      }, 0);
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

  const selectTag = (tag: string) => {
    targetTag = tag;
    searchQuery = tag;
    analyze(tag);
  };

  const analyze = async (tag: string) => {
    if (!tag) return;
    isLoading = true;
    await invoke<{
      counts: TagStats[];
      total: number;
    }>("calculate_co_occurence", {
      filters: {
        showAiGenerated: true,
        showNotAiGenerated: true,
        showXRestricted: true,
        showNotXRestricted: true,
        searchQuery: null,
      },
      tag,
    })
      .then((res) => {
        cooccurrenceResults = res.counts;
        totalInSubset = res.total;
      })
      .catch((e) => {
        console.error("Failed to fetch co-occurrence:", e);
        cooccurrenceResults = [];
        totalInSubset = 0;
      })
      .finally(() => {
        isLoading = false;
      });
  };

  // 共起分析のタグをクリックするとそのタグについて分析し直す
  const handleTagClick = (item: any) => {
    selectTag(item.title);
  };
</script>

<div class="h-full flex flex-col gap-4 overflow-hidden">
  <TopAppBar title="Tag Co-occurrence Analysis">
    <div slot="actions" class="p-4 flex flex-row gap-4 items-center">
      <div class="flex flex-row gap-2 items-center">
        <div class="text-sm text-neutral-700">
          選択中: <span class="font-bold text-(--md-primary)"
            >{targetTag || "なし"}</span
          >
          (母数:
          <span class="font-mono">{targetTag ? totalInSubset : 0}</span>件)
        </div>
        <div class="w-64">
          <input
            type="text"
            bind:value={searchQuery}
            onkeydown={handleKeyDown}
            placeholder="タグを入力して検索..."
            class="w-full px-3 py-2 border border-neutral-300 rounded-lg text-neutral-800 focus:outline-none focus:ring-2 focus:ring-(--md-primary)"
          />
          <!-- 簡易的なサジェストリスト -->
          {#if searchQuery && targetTag !== searchQuery}
            <div
              class="absolute w-64 bg-white border border-neutral-200 rounded-lg shadow-xl mt-1 max-h-60 overflow-y-auto"
            >
              {#each suggestedTags as tag, i}
                <button
                  id={"suggest-" + i}
                  class={`w-full text-left px-3 py-2 text-neutral-800 text-sm ${
                    i === selectedIndex
                      ? "bg-neutral-100 font-semibold"
                      : "hover:bg-neutral-100"
                  }`}
                  onclick={() => selectTag(tag)}
                  onmouseenter={() => (selectedIndex = i)}
                  onmouseleave={() => (selectedIndex = -1)}
                >
                  {tag}
                  <span class="text-neutral-400 text-xs"
                    >({tagCounts.find((entry) => entry.tag === tag)?.count ||
                      0})</span
                  >
                </button>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    </div>
  </TopAppBar>

  <div class="h-full w-full flex flex-row gap-2">
    <!-- 結果表示エリア -->
    <main
      class="flex-1 flex flex-col min-h-0 bg-white rounded-xl border border-neutral-200 overflow-hidden"
    >
      {#if targetTag && cooccurrenceResults.length > 0}
        <TagList
          items={cooccurrenceResults.map((e) => ({
            title: e.tag,
            count: e.count,
            subtitle: `共起率: ${((e.count / totalInSubset) * 100).toFixed(1)}%`,
            value: (e.count / totalInSubset) * 100,
          }))}
          onclick={handleTagClick}
        />
      {:else if targetTag}
        <div class="p-8 text-center text-neutral-500">
          <p>共起するタグが見つかりませんでした。</p>
        </div>
      {:else}
        <div
          class="p-8 text-center text-neutral-400 flex flex-col items-center justify-center h-full"
        >
          <div class="text-4xl mb-2">🔍</div>
          <p>左上のボックスから分析したいタグを検索して選択してください。</p>
        </div>
      {/if}
    </main>

    <aside class="flex flex-col gap-4 w-72 shrink-0 overflow-y-auto">
      <FiltersPanel bind:filter />
    </aside>
  </div>
</div>
