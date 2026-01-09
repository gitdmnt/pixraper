<script lang="ts">
  import TopAppBar from "$lib/components/TopAppBar.svelte";
  import TagList from "$lib/components/TagList.svelte";

  export let rows: any[] = [];

  let searchQuery = "";
  let targetTag: string | null = null;
  let cooccurrenceResults: {
    title: string;
    subtitle: string;
    value: string;
  }[] = [];

  let allTagsMap: Map<string, number>;
  let sortedAllTags: string[];
  let suggestedTags: string[];

  // 全タグのユニークリストと出現回数を作成（検索用）
  // rowsが変更されたら再計算
  $: allTagsMap = rows.reduce((acc, row) => {
    row.tags.forEach((tag: string) => {
      acc.set(tag, (acc.get(tag) || 0) + 1);
    });
    return acc;
  }, new Map<string, number>());

  $: sortedAllTags = Array.from(allTagsMap.entries())
    .sort((a, b) => b[1] - a[1])
    .map(([tag]) => tag);

  // 検索候補
  $: suggestedTags = searchQuery
    ? sortedAllTags
        .filter((t) => t.toLowerCase().includes(searchQuery.toLowerCase()))
        .slice(0, 10)
    : sortedAllTags.slice(0, 10);

  // suggestion keyboard navigation state
  let selectedIndex: number = -1;

  // Keep selectedIndex in range when suggestions change
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

  const selectTag = (tag: string) => {
    targetTag = tag;
    searchQuery = tag;
    analyze(tag);
  };

  const analyze = (tag: string) => {
    if (!tag) return;

    // そのタグを含む投稿を抽出
    const subset = rows.filter((row) => row.tags.includes(tag));
    const totalInSubset = subset.length;

    if (totalInSubset === 0) {
      cooccurrenceResults = [];
      return;
    }

    // 共起タグをカウント
    const coocMap = new Map<string, number>();
    subset.forEach((row) => {
      row.tags.forEach((t: string) => {
        if (t !== tag) {
          coocMap.set(t, (coocMap.get(t) || 0) + 1);
        }
      });
    });

    // ソートして結果形式に変換
    cooccurrenceResults = Array.from(coocMap.entries())
      .map(([t, count]) => {
        const rate = count / totalInSubset;
        return {
          title: t,
          subtitle: `共起回数: ${count}回 · 共起率: ${(rate * 100).toFixed(1)}%`,
          value: `${(rate * 100).toFixed(1)}%`,
          // internal sort key
          rawCount: count,
        };
      })
      .sort((a, b) => b.rawCount - a.rawCount)
      .map(({ title, subtitle, value }) => ({ title, subtitle, value }));
  };

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
          <span class="font-mono"
            >{rows.filter((r) => r.tags.includes(targetTag || "")).length ||
              0}</span
          >件)
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
                    >({allTagsMap.get(tag)})</span
                  >
                </button>
              {/each}
            </div>
          {/if}
        </div>
      </div>
      <p class="text-sm text-neutral-600">
        特定のタグと一緒に付けられることが多いタグを分析します。
      </p>
    </div>
  </TopAppBar>

  <!-- 結果表示エリア -->
  <div
    class="flex-1 flex flex-col min-h-0 bg-white rounded-xl border border-neutral-200 overflow-hidden"
  >
    {#if targetTag && cooccurrenceResults.length > 0}
      <TagList items={cooccurrenceResults} onclick={handleTagClick} />
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
  </div>
</div>
