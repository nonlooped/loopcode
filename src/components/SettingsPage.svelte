<script lang="ts">
  import { fly } from 'svelte/transition';
  import { IconContrast, IconDownload, IconListDetails, IconShieldLock } from '@tabler/icons-svelte';

  import { exportDiagnostics } from '../services/native';
  import type { PermissionMode } from '../types';

  interface Props {
    compactSessionRows: boolean;
    isLinux: boolean;
    linuxShellTransparency: number;
    linuxShellTransparencyRange: { min: number; max: number };
    permissionMode: PermissionMode;
    reducedMotion: boolean;
    setCompactSessionRows: (value: boolean) => void;
    setLinuxShellTransparency: (value: number) => void;
    setPermissionMode: (value: PermissionMode) => void;
  }

  const {
    compactSessionRows,
    isLinux,
    linuxShellTransparency,
    linuxShellTransparencyRange,
    permissionMode,
    reducedMotion,
    setCompactSessionRows,
    setLinuxShellTransparency,
    setPermissionMode,
  }: Props = $props();
  let exportState = $state<'idle' | 'exporting' | 'exported' | 'error'>('idle');

  async function exportLogs() {
    exportState = 'exporting';
    try {
      const destination = await exportDiagnostics();
      exportState = destination ? 'exported' : 'idle';
    } catch {
      exportState = 'error';
    }
  }
</script>

<section
  class="settings-page"
  aria-labelledby="settings-title"
  in:fly|global={{ y: reducedMotion ? 0 : 4, duration: reducedMotion ? 0 : 180 }}
>
  <div class="settings-column">
    <header class="settings-page-header">
      <h1 id="settings-title">General</h1>
      <p>Customize how LoopCode looks and feels.</p>
    </header>
    <div class="settings-card">
      <div class="settings-row">
        <span class="settings-row-icon" aria-hidden="true"><IconShieldLock size={17} stroke={1.55} /></span>
        <span class="settings-row-copy">
          <strong>Agent permissions</strong>
          <small>Restricted asks before commands. Full Access automatically approves every permission request.</small>
        </span>
        <fieldset class="permission-mode-control" aria-label="Agent permissions">
          <label class:active={permissionMode === 'restricted'}>
            <input
              type="radio"
              name="permission-mode"
              value="restricted"
              checked={permissionMode === 'restricted'}
              onchange={() => setPermissionMode('restricted')}
            />
            Restricted
          </label>
          <label class:active={permissionMode === 'full'}>
            <input
              type="radio"
              name="permission-mode"
              value="full"
              checked={permissionMode === 'full'}
              onchange={() => setPermissionMode('full')}
            />
            Full Access
          </label>
        </fieldset>
      </div>
      <label class="settings-row settings-row-separated">
        <span class="settings-row-icon" aria-hidden="true"><IconListDetails size={17} stroke={1.55} /></span>
        <span class="settings-row-copy">
          <strong>Compact thread rows</strong>
          <small>Show more threads in the sidebar by reducing row spacing.</small>
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
      {#if isLinux}
        <div class="settings-row settings-row-separated transparency-setting">
          <span class="settings-row-icon" aria-hidden="true"><IconContrast size={17} stroke={1.55} /></span>
          <span class="settings-row-copy">
            <strong>Background transparency</strong>
            <small>Shows the desktop behind LoopCode. Your desktop environment controls whether it is available.</small>
          </span>
          <label class="transparency-control" for="linux-shell-transparency">
            <output>{linuxShellTransparency}%</output>
            <input
              id="linux-shell-transparency"
              type="range"
              min={linuxShellTransparencyRange.min}
              max={linuxShellTransparencyRange.max}
              step="1"
              value={linuxShellTransparency}
              oninput={(event) => setLinuxShellTransparency(Number(event.currentTarget.value))}
            />
          </label>
        </div>
      {/if}
      <div class="settings-row settings-row-separated">
        <span class="settings-row-icon" aria-hidden="true"><IconDownload size={17} stroke={1.55} /></span>
        <span class="settings-row-copy">
          <strong>ACP diagnostics</strong>
          <small>Export redacted provider lifecycle, RPC envelope, error, and stderr logs.</small>
        </span>
        <button class="settings-action" disabled={exportState === 'exporting'} onclick={exportLogs}>
          {exportState === 'exporting' ? 'Exporting…' : exportState === 'exported' ? 'Exported' : exportState === 'error' ? 'Try again' : 'Export'}
        </button>
      </div>
    </div>
  </div>
</section>
