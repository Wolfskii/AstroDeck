<script lang="ts">
  import { parseReleaseNotes } from "../lib/releaseNotes";

  let { body = "" }: { body?: string | null } = $props();

  const blocks = $derived(parseReleaseNotes(body ?? ""));
</script>

{#if blocks.length === 0}
  <p class="update-notes-empty">No release notes were provided.</p>
{:else}
  <div class="update-notes">
    {#each blocks as block, index (index)}
      {#if block.kind === "heading"}
        <h4>{block.text}</h4>
      {:else if block.kind === "list"}
        <ul>
          {#each block.items as item (item)}
            <li>{item}</li>
          {/each}
        </ul>
      {:else}
        <p>{block.text}</p>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .update-notes {
    max-height: 220px;
    overflow: auto;
    margin: 0;
    padding: 14px 16px;
    color: var(--text-secondary);
    background: var(--bg-primary);
    border: 1px solid var(--border-subtle);
    border-radius: 10px;
    font-size: 0.9rem;
    line-height: 1.55;
    user-select: text;
    -webkit-user-select: text;
  }

  .update-notes h4 {
    margin: 0 0 10px;
    color: var(--text-primary);
    font-size: 0.98rem;
  }

  .update-notes ul {
    display: grid;
    gap: 8px;
    margin: 0;
    padding-left: 20px;
  }

  .update-notes li::marker {
    color: var(--accent);
  }

  .update-notes p,
  .update-notes-empty {
    margin: 0;
    color: var(--text-secondary);
    font-size: 0.9rem;
    line-height: 1.55;
  }

  .update-notes > :global(* + *) {
    margin-top: 10px;
  }
</style>
