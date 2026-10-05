<script lang="ts">
  let {
    checked,
    disabled = false,
    label,
    onchange,
  }: {
    checked: boolean;
    disabled?: boolean;
    label: string;
    onchange: (checked: boolean) => void;
  } = $props();
</script>

<button
  type="button"
  class="switch"
  class:on={checked}
  role="switch"
  aria-checked={checked}
  aria-label={label}
  {disabled}
  onclick={() => onchange(!checked)}
>
  <span class="knob"></span>
</button>

<style>
  .switch {
    position: relative;
    width: 54px;
    height: 32px;
    flex: 0 0 auto;
    padding: 0;
    border: none;
    border-radius: 999px;
    background: var(--md-slider-track, #c5cad3);
    box-shadow: inset 0 1px 2px rgba(17, 24, 39, 0.22);
    cursor: pointer;
  }

  .switch.on {
    background: var(--md-ink, #111827);
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.35);
  }

  .switch:focus-visible {
    outline: 2px solid var(--md-blue, #2563eb);
    outline-offset: 3px;
  }

  .switch:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .knob {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 26px;
    height: 26px;
    border-radius: 50%;
    background: var(--md-slider-thumb, #fff);
    box-shadow: 0 1px 2px rgba(17, 24, 39, 0.28), 0 4px 10px rgba(17, 24, 39, 0.16);
    transform: translateX(0);
    transition: transform 0.22s cubic-bezier(0.2, 0.8, 0.2, 1);
  }

  .switch.on .knob {
    transform: translateX(22px);
  }
</style>
