<script lang="ts">
  import type { WeatherScene } from "../services/weather";

  let { scene, isDay = true }: { scene: WeatherScene; isDay?: boolean } = $props();

  const rainDrops = Array.from({ length: 140 }, () => ({
    left: Math.random() * 108 - 4,
    delay: -(Math.random() * 1.8),
    duration: 0.42 + Math.random() * 0.55,
    length: 7 + Math.random() * 12,
    opacity: 0.22 + Math.random() * 0.38,
    drift: -(6 + Math.random() * 10),
  }));
  const showRain = $derived(scene === "rain" || scene === "thunder");
</script>

<div class="sky" data-scene={scene} data-night={isDay ? undefined : true} aria-hidden="true">
  <div class="wash"></div>
  <div class="sun"></div>
  <div class="cloud cloud-a"></div>
  <div class="cloud cloud-b"></div>
  <div class="cloud cloud-c"></div>
  <div class="mist"></div>
  {#if showRain}
    <div class="rain">
      {#each rainDrops as drop}
        <span
          style={`left: ${drop.left}%; animation-delay: ${drop.delay}s; animation-duration: ${drop.duration}s; height: ${drop.length}px; opacity: ${drop.opacity}; --drift: ${drop.drift}vw;`}
        ></span>
      {/each}
    </div>
  {/if}
  <div class="precip precip-back"></div>
  <div class="precip precip-front"></div>
  <div class="flash"></div>
  <div class="scrim"></div>
</div>

<style>
  .sky {
    position: absolute;
    inset: 0;
    overflow: hidden;
    background: #070b12;
    pointer-events: none;
  }

  .wash,
  .sun,
  .cloud,
  .mist,
  .precip,
  .flash,
  .scrim {
    position: absolute;
    inset: 0;
  }

  .wash {
    background:
      radial-gradient(90% 70% at 18% 0%, rgba(90, 120, 160, 0.35), transparent 60%),
      linear-gradient(180deg, #1a2433 0%, #0b1018 55%, #07090d 100%);
  }

  .sky[data-scene="clear"] .wash {
    background:
      radial-gradient(70% 55% at 78% 8%, rgba(255, 196, 92, 0.45), transparent 58%),
      linear-gradient(180deg, #1d4e89 0%, #0d2138 48%, #071018 100%);
  }

  .sky[data-scene="clear"][data-night] .wash {
    background:
      radial-gradient(50% 40% at 80% 12%, rgba(210, 220, 255, 0.22), transparent 60%),
      linear-gradient(180deg, #070b18 0%, #05070e 100%);
  }

  .sky[data-scene="partly"] .wash {
    background:
      radial-gradient(60% 45% at 75% 10%, rgba(255, 210, 130, 0.28), transparent 62%),
      linear-gradient(180deg, #24344a 0%, #121922 100%);
  }

  .sky[data-scene="fog"] .wash,
  .sky[data-scene="cloudy"] .wash {
    background: linear-gradient(180deg, #2a313b 0%, #14181e 100%);
  }

  .sky[data-scene="rain"] .wash,
  .sky[data-scene="thunder"] .wash {
    background: linear-gradient(180deg, #1c2430 0%, #0a0d12 70%);
  }

  .sky[data-scene="snow"] .wash {
    background: linear-gradient(180deg, #2c3644 0%, #12161c 100%);
  }

  .sun {
    display: none;
    inset: auto;
    top: 8%;
    right: 14%;
    width: 18vh;
    height: 18vh;
    border-radius: 50%;
    background: radial-gradient(circle, #fff4c8 0%, #f0c14a 42%, transparent 70%);
    filter: blur(2px);
  }

  .sky[data-scene="clear"] .sun,
  .sky[data-scene="partly"] .sun {
    display: block;
  }

  .sky[data-night] .sun {
    width: 10vh;
    height: 10vh;
    background: radial-gradient(circle, #f4f7ff 0%, #c9d4ee 46%, transparent 72%);
    box-shadow: 0 0 40px rgba(220, 230, 255, 0.25);
  }

  .cloud {
    inset: auto;
    height: 28vh;
    border-radius: 50%;
    background: rgba(210, 220, 230, 0.16);
    filter: blur(18px);
    opacity: 0;
  }

  .sky[data-scene="partly"] .cloud,
  .sky[data-scene="cloudy"] .cloud,
  .sky[data-scene="fog"] .cloud,
  .sky[data-scene="rain"] .cloud,
  .sky[data-scene="snow"] .cloud,
  .sky[data-scene="thunder"] .cloud {
    opacity: 1;
  }

  .cloud-a {
    top: 8%;
    left: -8%;
    width: 46vw;
    animation: drift 28s linear infinite;
  }

  .cloud-b {
    top: 18%;
    left: 28%;
    width: 38vw;
    height: 22vh;
    background: rgba(170, 182, 196, 0.2);
    animation: drift 36s linear infinite reverse;
  }

  .cloud-c {
    top: 4%;
    right: -10%;
    width: 42vw;
    animation: drift 42s linear infinite;
  }

  .sky[data-scene="rain"] .cloud,
  .sky[data-scene="thunder"] .cloud {
    height: 18vh;
    background: rgba(90, 102, 118, 0.22);
    filter: blur(28px);
    opacity: 0.7;
  }

  .mist {
    display: none;
    background:
      radial-gradient(60% 40% at 30% 70%, rgba(230, 236, 242, 0.2), transparent 70%),
      radial-gradient(50% 35% at 70% 60%, rgba(230, 236, 242, 0.16), transparent 72%);
    filter: blur(8px);
  }

  .sky[data-scene="fog"] .mist {
    display: block;
  }

  .rain {
    position: absolute;
    inset: 0;
    overflow: hidden;
  }

  .rain span {
    position: absolute;
    top: -8%;
    width: 1px;
    border-radius: 999px;
    background: linear-gradient(to bottom, transparent, rgba(232, 240, 248, 0.9));
    transform: rotate(16deg);
    animation: fall linear infinite;
  }

  .precip {
    display: none;
    background-repeat: repeat;
  }

  .sky[data-scene="snow"] .precip {
    display: block;
    background-image: radial-gradient(circle, rgba(255, 255, 255, 0.85) 0 1.5px, transparent 2px);
    background-size: 48px 64px;
    animation: snow 8s linear infinite;
  }

  .sky[data-scene="snow"] .precip-front {
    background-size: 28px 40px;
    opacity: 0.7;
    animation-duration: 5.5s;
  }

  .flash {
    display: none;
    background: rgba(235, 242, 255, 0.72);
    opacity: 0;
  }

  .sky[data-scene="thunder"] .flash {
    display: block;
    animation: lightning 8s infinite;
  }

  .scrim {
    background: linear-gradient(90deg, rgba(0, 0, 0, 0.42), rgba(0, 0, 0, 0.18) 45%, rgba(0, 0, 0, 0.4));
  }

  @keyframes drift {
    from {
      transform: translateX(0);
    }
    to {
      transform: translateX(8vw);
    }
  }

  @keyframes fall {
    from {
      transform: translate3d(0, -12vh, 0) rotate(16deg);
    }
    to {
      transform: translate3d(var(--drift), 112vh, 0) rotate(16deg);
    }
  }

  @keyframes snow {
    from {
      background-position: 0 -40px;
    }
    to {
      background-position: 16px 120px;
    }
  }

  @keyframes lightning {
    0%,
    88%,
    100% {
      opacity: 0;
    }
    89% {
      opacity: 0.55;
    }
    90% {
      opacity: 0.05;
    }
    92% {
      opacity: 0.7;
    }
    93% {
      opacity: 0;
    }
  }
</style>
