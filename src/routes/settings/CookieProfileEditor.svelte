<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import Button from "$lib/components/Button.svelte";
  import type { CookieProfile } from "./type.d";
  import { parseCookies, hasPhpSessionId } from "./cookieParser";

  export let profile: CookieProfile;
  export let onsave: (profile: CookieProfile) => void = () => {};

  let validating = false;
  let validateError: string | null = null;

  $: parsed = parseCookies(profile.cookies);
  $: missingPhpSession = profile.cookies.trim().length > 0 && !hasPhpSessionId(parsed);

  const handleValidate = async () => {
    validating = true;
    validateError = null;
    try {
      const result = await invoke<boolean>("validate_cookies", {
        cookies: profile.cookies,
      });
      profile = { ...profile, is_valid: result };
      onsave(profile);
    } catch (e) {
      validateError = String(e);
    } finally {
      validating = false;
    }
  };

  const handleSave = () => {
    onsave(profile);
  };
</script>

<div class="flex flex-col gap-4">
  <!-- Header -->
  <div class="flex items-center gap-3">
    <input
      type="text"
      class="md-select flex-1 font-medium"
      placeholder="Profile name"
      bind:value={profile.name}
    />
  </div>

  <!-- Cookie input -->
  <div class="flex flex-col gap-1">
    <div class="text-sm font-medium">Cookies</div>
    <textarea
      rows="5"
      class="md-select resize-y font-mono text-xs w-full leading-relaxed"
      placeholder="PHPSESSID=xxx; device_token=yyy; ..."
      bind:value={profile.cookies}
    ></textarea>

    {#if missingPhpSession}
      <div class="text-xs px-3 py-2 rounded-lg bg-amber-50 text-amber-700 border border-amber-200">
        PHPSESSID が含まれていません。ログインが必要なリクエストは失敗する可能性があります。
      </div>
    {/if}
  </div>

  <!-- Parsed view -->
  {#if parsed.length > 0}
    <div class="flex flex-col gap-1">
      <div class="text-sm font-medium">Parsed ({parsed.length} cookies)</div>
      <div class="rounded-lg border border-(--md-outline) overflow-hidden">
        <div class="grid grid-cols-[auto_1fr] text-xs bg-surface-container-low px-3 py-1 font-medium text-muted border-b border-(--md-outline)">
          <span class="pr-4">Key</span>
          <span>Value</span>
        </div>
        <div class="max-h-48 overflow-y-auto">
          {#each parsed as cookie}
            <div class="grid grid-cols-[auto_1fr] px-3 py-1.5 border-b border-(--md-outline) last:border-0 hover:bg-surface-container-low transition-colors">
              <span
                class="pr-4 font-mono font-medium shrink-0 {cookie.important ? 'text-(--md-primary)' : 'text-muted'}"
                title={cookie.important ? 'Important Pixiv cookie' : ''}
              >
                {cookie.key}{cookie.important ? ' ★' : ''}
              </span>
              <span class="font-mono text-muted truncate" title={cookie.value}>
                {cookie.value}
              </span>
            </div>
          {/each}
        </div>
      </div>
    </div>
  {/if}

  <!-- Actions -->
  <div class="flex items-center gap-2 flex-wrap">
    <Button variant="contained" onclick={handleSave}>Save</Button>
    <Button
      variant="outlined"
      onclick={handleValidate}
      disabled={validating || !profile.cookies.trim()}
    >
      {validating ? "Validating…" : "Validate"}
    </Button>

    {#if profile.is_valid === true}
      <span class="text-xs px-2 py-1 rounded-full bg-emerald-50 text-emerald-700 border border-emerald-200">✓ Valid</span>
    {:else if profile.is_valid === false}
      <span class="text-xs px-2 py-1 rounded-full bg-red-50 text-red-700 border border-red-200">✕ Invalid</span>
    {/if}

    {#if validateError}
      <span class="text-xs text-error">{validateError}</span>
    {/if}
  </div>
</div>
