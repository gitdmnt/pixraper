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

  import TopAppBar from "$lib/components/TopAppBar.svelte";
  import Button from "$lib/components/Button.svelte";

  let tab = $state(0);
</script>

<main
  class="
  flex flex-col
w-dvw h-dvh overflow-hidden
text-neutral-800"
>
  <TopAppBar title="pixraper">
    <div slot="leading" class="md-segment">
      <button onclick={() => (tab = 0)} class:active={tab === 0}
        >Analytics</button
      >
      <button onclick={() => (tab = 1)} class:active={tab === 1}
        >Scraping</button
      >
      <button onclick={() => (tab = 2)} class:active={tab === 2}
        >Settings</button
      >
    </div>
  </TopAppBar>
  <div class="w-full flex-1">
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
