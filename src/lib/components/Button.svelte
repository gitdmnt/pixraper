<script lang="ts">
  export let variant: "contained" | "outlined" | "text" = "contained";
  export let disabled: boolean = false;
  export let type: "button" | "submit" = "button";
  export let onclick: (() => void) | null = null;
</script>

<button
  class="btn-md {variant === 'outlined' ? 'btn-outlined' : ''} {variant ===
  'text'
    ? 'btn-text'
    : ''}"
  {disabled}
  {type}
  {onclick}
>
  <slot />
</button>

<style>
  .btn-outlined {
    background: transparent;
    color: var(--md-primary);
    border: 1px solid var(--md-outline);
  }
  .btn-text {
    background: transparent;
    color: var(--md-primary);
    box-shadow: none;
    padding: 6px 10px;
  }
</style>
