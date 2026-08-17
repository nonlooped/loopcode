<script lang="ts">
  import { IconListDetails } from '@tabler/icons-svelte';

  interface Props {
    compactSessionRows: boolean;
    setCompactSessionRows: (value: boolean) => void;
  }

  const { compactSessionRows, setCompactSessionRows }: Props = $props();
</script>

<section class="settings-page" aria-labelledby="settings-title">
  <div class="settings-column">
    <header class="settings-page-header">
      <h1 id="settings-title">General</h1>
      <p>Customize how LoopCode looks and feels.</p>
    </header>
    <div class="settings-card">
      <label class="settings-row">
        <span class="settings-row-icon" aria-hidden="true"><IconListDetails size={17} stroke={1.55} /></span>
        <span class="settings-row-copy">
          <strong>Compact session rows</strong>
          <small>Show more sessions in the sidebar by reducing row spacing.</small>
        </span>
        <span class="toggle-control">
          <input
            type="checkbox"
            checked={compactSessionRows}
            onchange={(event) => setCompactSessionRows(event.currentTarget.checked)}
          />
          <span class="toggle-track" aria-hidden="true"><span class="toggle-thumb"></span></span>
        </span>
      </label>
    </div>
  </div>
</section>
