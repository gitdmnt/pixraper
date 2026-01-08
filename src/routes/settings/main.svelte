<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { Config } from "./type.d.ts";

  import { Temporal } from "@js-temporal/polyfill";

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

<div class="container">
  {#if settingValue}
    <h2>Settings</h2>
    <button onclick={saveSettings}>Save Settings</button>
    <h3>Pixiv Cookies</h3>
    <textarea rows="10" cols="80" bind:value={settingValue.cookies}></textarea>
    <h3>Output Directory</h3>
    <input type="text" size="80" bind:value={settingValue.output} />
    <h3>Scraping Interval (milliseconds)</h3>
    <input
      type="number"
      bind:value={settingValue.scraping_interval_millis}
      min="0"
      step="100"
    />
  {:else}
    <p>Loading settings...</p>
  {/if}
</div>
