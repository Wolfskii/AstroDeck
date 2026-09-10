<script lang="ts">
  import appIconUrl from "../assets/app-icon.png";
  import Icon from "./Icon.svelte";
  import SceneActionButton from "./SceneActionButton.svelte";
  import type { DeckButtonConfig } from "../types";
  import type { TeamsStatus } from "../services/api";

  let {
    buttons,
    showSettingsButton = false,
    onOpenSettings,
    teamsStatus = null,
  }: {
    buttons: DeckButtonConfig[];
    showSettingsButton?: boolean;
    onOpenSettings?: () => void;
    teamsStatus?: TeamsStatus | null;
  } = $props();

  const reactions = $derived(
    buttons.filter((button) => button.action.startsWith("teams.reaction."))
  );
  const controls = $derived(
    buttons.filter((button) => !button.action.startsWith("teams.reaction."))
  );
  let muted = $state(false);
  let cameraOff = $state(false);
  const effectiveMuted = $derived(teamsStatus?.isConnected ? teamsStatus.isMuted : muted);
  const effectiveCameraOff = $derived(
    teamsStatus?.isConnected ? !teamsStatus.isVideoOn : cameraOff
  );
  const effectiveSharing = $derived(teamsStatus?.isConnected ? teamsStatus.isSharing : false);
</script>

<section class="teams" class:has-settings={showSettingsButton && onOpenSettings}>
  {#if showSettingsButton && onOpenSettings}
    <button
      type="button"
      class="settings-btn"
      title="Settings"
      aria-label="Open settings"
      onclick={() => onOpenSettings()}
    >
      <img class="settings-icon" src={appIconUrl} alt="" aria-hidden="true" />
    </button>
  {/if}

  <header class="brand">
    <span class="logo" aria-hidden="true">
      <Icon name="teamsMark" size={28} />
    </span>
    <div>
      <p class="eyebrow">Microsoft Teams</p>
      <h1>Meeting</h1>
    </div>
  </header>

  {#if reactions.length}
    <div class="panel">
      <p class="section-label">Reactions</p>
      <div class="reactions">
        {#each reactions as button (button.action)}
          <SceneActionButton
            label={button.label}
            action={button.action}
            source="Teams window"
            variant="reaction"
          />
        {/each}
      </div>
    </div>
  {/if}

  {#if controls.length}
    <div class="call-bar">
      {#each controls as button (button.action)}
        <SceneActionButton
          label={
            button.action === "teams.toggleMute" && effectiveMuted
              ? "Unmute"
              : button.action === "teams.toggleCamera" && effectiveCameraOff
                ? "Turn camera on"
                : button.action === "teams.shareScreen" && effectiveSharing
                  ? "Stop sharing"
                : button.label
          }
          action={button.action}
          source="Teams window"
          variant="call"
          tone={
              button.action === "teams.shareScreen" && !effectiveSharing
              ? "accent"
              : button.action === "teams.shareScreen" && effectiveSharing
                ? "danger"
              : (button.action === "teams.toggleMute" && effectiveMuted) ||
                  (button.action === "teams.toggleCamera" && effectiveCameraOff)
                ? "danger"
                : "default"
          }
          active={
            (button.action === "teams.toggleMute" && effectiveMuted) ||
            (button.action === "teams.toggleCamera" && effectiveCameraOff) ||
            (button.action === "teams.shareScreen" && effectiveSharing)
          }
          onAction={() => {
            if (!teamsStatus?.isConnected && button.action === "teams.toggleMute") {
              muted = !muted;
            }
            if (!teamsStatus?.isConnected && button.action === "teams.toggleCamera") {
              cameraOff = !cameraOff;
            }
          }}
        />
      {/each}
    </div>
  {/if}
</section>

<style>
  .teams {
    position: relative;
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    min-height: 0;
    padding: 20px 24px 28px;
    background:
      radial-gradient(1200px 480px at 12% -10%, rgba(91, 95, 199, 0.28), transparent 55%),
      #1b1a1f;
    color: #f3f2f1;
  }

  .teams.has-settings {
    padding-top: 92px;
  }

  .settings-btn {
    position: absolute;
    top: 8px;
    left: 12px;
    z-index: 2;
    padding: 4px;
    border: none;
    border-radius: 12px;
    background: transparent;
    line-height: 0;
    cursor: pointer;
  }

  .settings-btn:hover {
    background: rgba(255, 255, 255, 0.08);
  }

  .settings-icon {
    display: block;
    width: 72px;
    height: 72px;
    object-fit: contain;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-bottom: 22px;
  }

  .logo {
    display: grid;
    place-items: center;
    width: 48px;
    height: 48px;
    border-radius: 14px;
    background: #5b5fc7;
    color: #fff;
    box-shadow: 0 10px 24px rgba(91, 95, 199, 0.35);
  }

  .eyebrow {
    margin: 0;
    color: #a8a6c3;
    font-size: 0.78rem;
    font-weight: 650;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  h1 {
    margin: 2px 0 0;
    font-size: 1.45rem;
    font-weight: 700;
    letter-spacing: -0.02em;
  }

  .panel {
    flex: 1 1 auto;
    min-height: 0;
    display: flex;
    flex-direction: column;
    justify-content: center;
    padding: 8px 8px 18px;
  }

  .section-label {
    margin: 0 0 12px;
    color: #c8c6c4;
    font-size: 0.8rem;
    font-weight: 650;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .reactions {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 20px;
    width: 100%;
    max-width: 980px;
    margin: 0 auto;
  }

  .call-bar {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 18px 22px;
    padding: 18px 8px 4px;
    border-top: 1px solid rgba(255, 255, 255, 0.08);
  }
</style>
