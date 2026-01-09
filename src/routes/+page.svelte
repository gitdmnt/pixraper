<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  let name = $state("");
  let greetMsg = $state("");

  async function greet(event: Event) {
    event.preventDefault();
    // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
    greetMsg = await invoke("greet", { name });
  }

  import AnalyticsMain from "./analytics/main.svelte";
  import ScrapingMain from "./scraping/main.svelte";
  import SettingsMain from "./settings/main.svelte";

  let tab = $state(0);
</script>

<main class="text-neutral-800">
  <div
    class="
  flex flex-row mb-4 bg-neutral-200 text-xs
  [&>button]:px-3
  [&>button]:py-2
  [&>button]:bg-white
  [&>button]:border-neutral-300
  [&>button]:cursor-pointer
  [&>button]:transition
  [&>button]:duration-150
  [&>button]:ease-in-out
  [&>button.hover]:font-bold
  [&>button.hover]:bg-neutral-300
  [&>button.active]:font-bold
  [&>button.active]:bg-white
  [&>button.active]:border-b-2
  [&>button.active]:border-b-blue-500
  "
  >
    <button onclick={() => (tab = 0)} class:active={tab === 0}>
      Analytics
    </button>
    <button onclick={() => (tab = 1)} class:active={tab === 1}>
      Scraping
    </button>
    <button onclick={() => (tab = 2)} class:active={tab === 2}>
      Settings
    </button>
  </div>
  <div>
    {#if tab === 0}
      <AnalyticsMain />
    {:else if tab === 1}
      <ScrapingMain />
    {:else if tab === 2}
      <SettingsMain />
    {/if}
  </div>
</main>

<style>
</style>
