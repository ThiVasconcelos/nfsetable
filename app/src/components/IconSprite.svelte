<script lang="ts">
  // Every icon defined once as a <symbol>; <Icon> only places a <use>, so a row of the table that is
  // created again (scrolling) parses no markup.
  import { ICONS } from './icons'

  const symbols = Object.entries(ICONS)
    .map(([name, markup]) => `<symbol id="icon-${name}" viewBox="0 0 24 24">${markup}</symbol>`)
    .join('')
</script>

<svg class="sprite" aria-hidden="true" focusable="false">
  <!-- Static markup bundled in icons.ts (never user input). -->
  {@html symbols}
</svg>

<style>
  .sprite {
    position: absolute;
    width: 0;
    height: 0;
    overflow: hidden;
  }
</style>
