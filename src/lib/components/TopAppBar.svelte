<script lang="ts">
  export let title: string = "";
</script>

<header class="md-top-app-bar md-card flex items-center gap-3 px-4 py-2">
  <div class="flex items-center gap-3">
    {#if title}
      <div class="text-sm font-semibold">{title}</div>
    {/if}
    <slot name="leading" />
  </div>
  <div class="ml-auto flex items-center gap-2">
    <slot name="actions" />
  </div>
</header>

<style>
  .md-top-app-bar {
    height: 56px;
    background-color: var(--md-surface);
  }
</style>
