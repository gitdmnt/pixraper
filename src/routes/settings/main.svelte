<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { Config } from "./type.d.ts";
  import Button from "$lib/components/Button.svelte";

  let settingValue: Config = $state({
    cookies: "",
    output: "",
    scraping_interval_millis: 1000,
  });

  invoke<Config>("get_config").then((value) => {
    settingValue = value;
  });

  const saveSettings = () => {
    invoke("set_config", { newConfig: settingValue }).then(() => {
      console.log("Settings saved");
      invoke<Config>("get_config").then((value) => {
        console.log("Current settings:", value);
      });
    });
  };
</script>

<div class="flex flex-col p-6 gap-4 app-content h-full overflow-y-auto">
  {#if settingValue}
    <div class="md-card p-6 flex flex-col gap-5 max-w-2xl">
      <div class="text-base font-semibold">Settings</div>

      <div class="flex flex-col gap-1">
        <label class="text-sm font-medium" for="cookies">Pixiv Cookies</label>
        <textarea
          id="cookies"
          rows="6"
          class="md-select resize-y font-mono text-xs w-full"
          bind:value={settingValue.cookies}
          placeholder="Paste your Pixiv cookies here..."
        ></textarea>
      </div>

      <div class="flex flex-col gap-1">
        <label class="text-sm font-medium" for="output">Output Directory</label>
        <input
          id="output"
          type="text"
          class="md-select w-full"
          bind:value={settingValue.output}
          placeholder="/path/to/output"
        />
      </div>

      <div class="flex flex-col gap-1">
        <label class="text-sm font-medium" for="interval"
          >Scraping Interval (ms)</label
        >
        <input
          id="interval"
          type="number"
          class="md-select w-32"
          bind:value={settingValue.scraping_interval_millis}
          min="0"
          step="100"
        />
        <div class="text-xs text-muted">Minimum delay between requests</div>
      </div>

      <div>
        <Button variant="contained" onclick={saveSettings}>Save Settings</Button>
      </div>
    </div>
  {:else}
    <div class="md-card p-6 text-muted text-sm">Loading settings...</div>
  {/if}
</div>
