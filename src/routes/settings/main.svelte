<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { Config, CookieProfile } from "./type.d.ts";
  import Button from "$lib/components/Button.svelte";
  import CookieProfileList from "./CookieProfileList.svelte";
  import CookieProfileEditor from "./CookieProfileEditor.svelte";

  let config: Config = $state({
    cookies: null,
    output: null,
    scraping_interval_millis: 1000,
    cookie_profiles: [],
    active_profile_id: null,
  });

  let selectedProfileId: string | null = $state(null);

  invoke<Config>("get_config").then((value) => {
    config = { ...value, cookie_profiles: value.cookie_profiles ?? [] };
    if (config.cookie_profiles.length > 0) {
      selectedProfileId = config.cookie_profiles[0].id;
    }
  });

  const selectedProfile = $derived(
    config.cookie_profiles.find((p) => p.id === selectedProfileId) ?? null
  );

  let saveError: string | null = $state(null);

  const saveConfig = async (cfg: Config) => {
    try {
      await invoke("set_config", { newConfig: cfg });
      config = cfg;
      saveError = null;
    } catch (e) {
      saveError = String(e);
      console.error("Failed to save config:", e);
    }
  };

  const handleSaveProfile = (profile: CookieProfile) => {
    saveConfig({
      ...config,
      cookie_profiles: config.cookie_profiles.map((p) =>
        p.id === profile.id ? profile : p
      ),
    });
  };

  const handleAddProfile = () => {
    const newProfile: CookieProfile = {
      id: crypto.randomUUID(),
      name: `Profile ${config.cookie_profiles.length + 1}`,
      cookies: "",
      is_valid: null,
    };
    selectedProfileId = newProfile.id;
    saveConfig({
      ...config,
      cookie_profiles: [...config.cookie_profiles, newProfile],
    });
  };

  const handleDeleteProfile = (id: string) => {
    const remaining = config.cookie_profiles.filter((p) => p.id !== id);
    if (selectedProfileId === id) {
      selectedProfileId = remaining[0]?.id ?? null;
    }
    saveConfig({ ...config, cookie_profiles: remaining });
  };

  const saveGeneralSettings = () => {
    saveConfig(config);
  };
</script>

<div class="flex flex-col p-6 gap-6 app-content h-full overflow-y-auto">

  <!-- General Settings -->
  <div class="md-card p-6 flex flex-col gap-4 max-w-2xl">
    <div class="text-base font-semibold">General Settings</div>

    <div class="flex flex-col gap-1">
      <label class="text-sm font-medium" for="output">Output Directory</label>
      <input
        id="output"
        type="text"
        class="md-select w-full"
        bind:value={config.output}
        placeholder="/path/to/output"
      />
    </div>

    <div class="flex flex-col gap-1">
      <label class="text-sm font-medium" for="interval">Scraping Interval (ms)</label>
      <input
        id="interval"
        type="number"
        class="md-select w-32"
        bind:value={config.scraping_interval_millis}
        min="0"
        step="100"
      />
      <div class="text-xs text-muted">Minimum delay between requests</div>
    </div>

    <div class="flex items-center gap-3">
      <Button variant="contained" onclick={saveGeneralSettings}>Save</Button>
      {#if saveError}
        <span class="text-xs text-error">{saveError}</span>
      {/if}
    </div>
  </div>

  <!-- Cookie Profiles -->
  <div class="md-card p-6 flex flex-col gap-4 max-w-4xl">
    <div class="text-base font-semibold">Cookie Profiles</div>
    <div class="text-xs text-muted">
      登録された全プロファイルをジョブ単位で順番に使い回します。プロファイルが1件の場合はそのプロファイルのみ使用します。
    </div>

    <div class="flex flex-row gap-6">
      <div class="w-56 shrink-0">
        <CookieProfileList
          profiles={config.cookie_profiles}
          {selectedProfileId}
          onselect={(id) => (selectedProfileId = id)}
          onadd={handleAddProfile}
          ondelete={handleDeleteProfile}
        />
      </div>

      <div class="flex-1 min-w-0">
        {#if selectedProfile}
          <CookieProfileEditor
            profile={selectedProfile}
            onsave={handleSaveProfile}
          />
        {:else}
          <div class="text-sm text-muted pt-4">
            プロファイルを選択するか、「Add」で追加してください。
          </div>
        {/if}
      </div>
    </div>
  </div>

</div>
