<script lang="ts">
  import type { IconName } from './icons'

  interface Props {
    name: IconName
    size?: number
    strokeWidth?: number
    /** Accessible label; omit for decorative icons (the default). */
    label?: string
    class?: string
  }

  let { name, size = 16, strokeWidth = 1.8, label, class: className = '' }: Props = $props()
</script>

<svg
  class="icon {className}"
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={strokeWidth}
  stroke-linecap="round"
  stroke-linejoin="round"
  role={label ? 'img' : undefined}
  aria-label={label}
  aria-hidden={label ? undefined : 'true'}
  focusable="false"
>
  <!-- The drawing is a <symbol> of IconSprite (mounted once by App). -->
  <use href="#icon-{name}" />
</svg>

<style>
  .icon {
    flex: none;
    display: block;
  }
</style>
