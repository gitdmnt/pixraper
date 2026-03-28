<script lang="ts">
  import type { CookieProfile } from "./type.d.ts";

  export let profiles: CookieProfile[] = [];
  export let selectedId: string | null = null;

  export let onselect: (id: string) => void = () => {};
  export let onadd: () => void = () => {};
  export let ondelete: (id: string) => void = () => {};

  const validityBadge = (isValid: boolean | null) => {
    if (isValid === true) return { label: "Valid", cls: "text-emerald-700 bg-emerald-50 border-emerald-200" };
    if (isValid === false) return { label: "Invalid", cls: "text-red-700 bg-red-50 border-red-200" };
    return { label: "Unknown", cls: "text-muted bg-surface-container-low border-(--md-outline)" };
  };
</script>

<div class="flex flex-col gap-2">
  <div class="flex items-center justify-between">
    <div class="text-sm font-medium">Cookie Profiles</div>
    <button class="btn-md text-xs px-3 py-1" onclick={onadd}>+ Add</button>
  </div>

  {#if profiles.length === 0}
    <div class="text-sm text-muted py-4 text-center">
      No profiles. Click "Add" to create one.
    </div>
  {:else}
    <div class="flex flex-col gap-1">
      {#each profiles as profile}
        {@const badge = validityBadge(profile.is_valid)}
        <div
          class="flex items-center gap-2 p-2 rounded-lg cursor-pointer transition-colors {selectedId === profile.id ? 'bg-surface-container' : 'hover:bg-surface-container-low'}"
          role="button"
          tabindex="0"
          onclick={() => onselect(profile.id)}
          onkeydown={(e) => e.key === 'Enter' && onselect(profile.id)}
        >
          <div class="flex-1 min-w-0">
            <div class="text-sm font-medium truncate">{profile.name || "Unnamed"}</div>
          </div>

          <span class="text-xs px-2 py-0.5 rounded-full border shrink-0 {badge.cls}">{badge.label}</span>

          <button
            class="text-xs text-muted hover:text-error transition-colors px-1 shrink-0"
            title="Delete profile"
            onclick={(e) => { e.stopPropagation(); ondelete(profile.id); }}
          >✕</button>
        </div>
      {/each}
    </div>
  {/if}
</div>
