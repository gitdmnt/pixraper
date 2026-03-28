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
    config = {
      ...value,
      cookie_profiles: value.cookie_profiles ?? [],
    };
    if (config.active_profile_id) {
      selectedProfileId = config.active_profile_id;
    } else if (config.cookie_profiles.length > 0) {
      selectedProfileId = config.cookie_profiles[0].id;
    }
  });

  const selectedProfile = $derived(
    config.cookie_profiles.find((p) => p.id === selectedProfileId) ?? null
  );

  const activeProfileCookies = (cfg: Config): string | null => {
    if (!cfg.active_profile_id) return null;
    const profile = cfg.cookie_profiles.find((p) => p.id === cfg.active_profile_id);
    return profile?.cookies ?? null;
  };

  let saveError: string | null = $state(null);

  const saveConfig = async (cfg: Config) => {
    const updated: Config = {
      ...cfg,
      cookies: activeProfileCookies(cfg),
    };
    try {
      await invoke("set_config", { newConfig: updated });
      config = updated;
      saveError = null;
    } catch (e) {
      saveError = String(e);
      console.error("Failed to save config:", e);
    }
  };

  const handleSaveProfile = (profile: CookieProfile) => {
    const updated: Config = {
      ...config,
      cookie_profiles: config.cookie_profiles.map((p) =>
        p.id === profile.id ? profile : p
      ),
    };
    saveConfig(updated);
  };

  const handleAddProfile = () => {
    const newProfile: CookieProfile = {
      id: crypto.randomUUID(),
      name: `Profile ${config.cookie_profiles.length + 1}`,
      cookies: "",
      is_valid: null,
    };
    const updated: Config = {
      ...config,
      cookie_profiles: [...config.cookie_profiles, newProfile],
    };
    selectedProfileId = newProfile.id;
    saveConfig(updated);
  };

  const handleDeleteProfile = (id: string) => {
    const remaining = config.cookie_profiles.filter((p) => p.id !== id);
    const newActiveId =
      config.active_profile_id === id
        ? (remaining[0]?.id ?? null)
        : config.active_profile_id;
    const updated: Config = {
      ...config,
      cookie_profiles: remaining,
      active_profile_id: newActiveId,
    };
    if (selectedProfileId === id) {
      selectedProfileId = remaining[0]?.id ?? null;
    }
    saveConfig(updated);
  };

  const handleSetActive = (id: string) => {
    saveConfig({ ...config, active_profile_id: id });
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
      複数の Pixiv アカウントの Cookie を保存できます。アクティブなプロファイルがスクレイパーに使用されます。
    </div>

    <div class="flex flex-row gap-6">
      <!-- Profile list -->
      <div class="w-56 shrink-0">
        <CookieProfileList
          profiles={config.cookie_profiles}
          activeProfileId={config.active_profile_id}
          {selectedProfileId}
          onselect={(id) => (selectedProfileId = id)}
          onadd={handleAddProfile}
          ondelete={handleDeleteProfile}
          onsetActive={handleSetActive}
        />
      </div>

      <!-- Editor -->
      <div class="flex-1 min-w-0">
        {#if selectedProfile}
          <CookieProfileEditor
            profile={selectedProfile}
            activeProfileId={config.active_profile_id}
            onsave={handleSaveProfile}
            onsetActive={handleSetActive}
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
