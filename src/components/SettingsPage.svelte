<script lang="ts">
  import { fly } from 'svelte/transition';
  import {
    IconActivity,
    IconArchive,
    IconArrowsHorizontal,
    IconAt,
    IconClock,
    IconCode,
    IconContrast,
    IconDownload,
    IconFolder,
    IconHistory,
    IconKeyboard,
    IconLayoutSidebarRight,
    IconListDetails,
    IconPlayerPlay,
    IconRestore,
    IconRobot,
    IconShieldLock,
    IconTerminal2,
    IconTextSize,
    IconTrash,
    IconWriting,
    IconZoomIn,
  } from '@tabler/icons-svelte';
  import { AlertDialog, RadioGroup, Slider, Switch } from 'bits-ui';

  import { exportDiagnostics } from '../services/native';
  import type { HarnessProfile, PermissionMode, ProviderModelCatalog } from '../types';
  import {
    CONTENT_WIDTH_RANGE,
    INTERFACE_ZOOM_RANGE,
    TERMINAL_FONT_SIZE_RANGE,
    TERMINAL_HEIGHT_RANGE,
    type AppPreferences,
    type SettingsCategory,
  } from '../utils/app-settings';

  interface Props {
    category: SettingsCategory;
    preferences: AppPreferences;
    profiles: HarnessProfile[];
    catalogs: Record<string, ProviderModelCatalog>;
    isLinux: boolean;
    linuxShellTransparency: number;
    linuxShellTransparencyRange: { min: number; max: number };
    permissionMode: PermissionMode;
    terminalHeight: number;
    reducedMotion: boolean;
    setPreference: <K extends keyof AppPreferences>(key: K, value: AppPreferences[K]) => void;
    setLinuxShellTransparency: (value: number) => void;
    setPermissionMode: (value: PermissionMode) => void;
    setTerminalHeight: (value: number) => void;
    archivedThreadCount: number;
    draftThreadCount: number;
    clearArchivedThreads: () => void;
    clearComposerDrafts: () => void;
    resetSettings: () => void;
  }

  const {
    category,
    preferences,
    profiles,
    catalogs,
    isLinux,
    linuxShellTransparency,
    linuxShellTransparencyRange,
    permissionMode,
    terminalHeight,
    reducedMotion,
    setPreference,
    setLinuxShellTransparency,
    setPermissionMode,
    setTerminalHeight,
    archivedThreadCount,
    draftThreadCount,
    clearArchivedThreads,
    clearComposerDrafts,
    resetSettings,
  }: Props = $props();
  let exportState = $state<'idle' | 'exporting' | 'exported' | 'error'>('idle');
  let pendingDataAction = $state<'archived' | 'drafts'>();

  const categoryCopy = {
    general: ['General', 'Set defaults for thread navigation and the sidebar.'],
    appearance: ['Appearance', 'Adjust motion, scale, and the desktop surface.'],
    conversation: ['Conversation', 'Control transcript layout and composer behavior.'],
    agents: ['Agents and permissions', 'Choose how new threads start and what agents may run.'],
    terminal: ['Terminal', 'Tune the terminal drawer and its retained output.'],
    data: ['Data and diagnostics', 'Export troubleshooting data or restore interface defaults.'],
  } satisfies Record<SettingsCategory, [string, string]>;

  async function exportLogs() {
    exportState = 'exporting';
    try {
      const destination = await exportDiagnostics();
      exportState = destination ? 'exported' : 'idle';
    } catch {
      exportState = 'error';
    }
  }

  function selectValue(event: Event) {
    return event.currentTarget instanceof HTMLSelectElement ? event.currentTarget.value : '';
  }

  function providerModelValue(profileId: string) {
    const saved = preferences.providerModelDefaults[profileId];
    return catalogs[profileId]?.models.some((model) => model.id === saved) ? saved : '';
  }

  function setProviderModelDefault(profileId: string, modelId: string) {
    const defaults = { ...preferences.providerModelDefaults };
    if (modelId) defaults[profileId] = modelId;
    else delete defaults[profileId];
    setPreference('providerModelDefaults', defaults);
  }

  function confirmDataAction() {
    if (pendingDataAction === 'archived') clearArchivedThreads();
    else if (pendingDataAction === 'drafts') clearComposerDrafts();
    pendingDataAction = undefined;
  }
</script>

<section
  class="settings-page"
  aria-labelledby="settings-title"
  in:fly|global={{ y: reducedMotion ? 0 : 4, duration: reducedMotion ? 0 : 180 }}
>
  <div class="settings-column">
    <header class="settings-page-header">
      <h1 id="settings-title" tabindex="-1">{categoryCopy[category][0]}</h1>
      <p>{categoryCopy[category][1]}</p>
    </header>

    {#if category === 'general'}
      <div class="settings-card">
        <div class="settings-row">
          <span class="settings-row-icon" aria-hidden="true"><IconListDetails size={17} stroke={1.55} /></span>
          <span class="settings-row-copy">
            <strong>Compact thread rows</strong>
            <small>Show more threads in the sidebar by reducing row spacing.</small>
          </span>
          <Switch.Root
            class="toggle-control"
            aria-label="Compact thread rows"
            checked={preferences.compactSessionRows}
            onCheckedChange={(value) => setPreference('compactSessionRows', value)}
          >
            <span class="toggle-track" aria-hidden="true"><Switch.Thumb class="toggle-thumb" /></span>
          </Switch.Root>
        </div>
        <div class="settings-row settings-row-separated">
          <span class="settings-row-icon" aria-hidden="true"><IconArchive size={17} stroke={1.55} /></span>
          <span class="settings-row-copy">
            <strong>Keep archived threads expanded</strong>
            <small>Remember whether the archived section stays open between launches.</small>
          </span>
          <Switch.Root
            class="toggle-control"
            aria-label="Keep archived threads expanded"
            checked={preferences.showSettled}
            onCheckedChange={(value) => setPreference('showSettled', value)}
          >
            <span class="toggle-track" aria-hidden="true"><Switch.Thumb class="toggle-thumb" /></span>
          </Switch.Root>
        </div>
        <div class="settings-row settings-row-separated">
          <span class="settings-row-icon" aria-hidden="true"><IconPlayerPlay size={17} stroke={1.55} /></span>
          <span class="settings-row-copy">
            <strong>On startup</strong>
            <small>Return to the last thread or open an empty thread when LoopCode starts.</small>
          </span>
          <RadioGroup.Root
            class="settings-segmented-control"
            aria-label="On startup"
            orientation="horizontal"
            value={preferences.startupBehavior}
            onValueChange={(value) => setPreference('startupBehavior', value === 'new-thread' ? 'new-thread' : 'last-thread')}
          >
            <RadioGroup.Item class="settings-segmented-option" value="last-thread">Last thread</RadioGroup.Item>
            <RadioGroup.Item class="settings-segmented-option" value="new-thread">Empty thread</RadioGroup.Item>
          </RadioGroup.Root>
        </div>
        <div class="settings-row settings-row-separated">
          <span class="settings-row-icon" aria-hidden="true"><IconFolder size={17} stroke={1.55} /></span>
          <span class="settings-row-copy">
            <strong>New thread location</strong>
            <small>Start toolbar-created threads in the selected project or the default working folder.</small>
          </span>
          <RadioGroup.Root
            class="settings-segmented-control"
            aria-label="New thread location"
            orientation="horizontal"
            value={preferences.newThreadProject}
            onValueChange={(value) => setPreference('newThreadProject', value === 'default-folder' ? 'default-folder' : 'selected')}
          >
            <RadioGroup.Item class="settings-segmented-option" value="selected">Selected project</RadioGroup.Item>
            <RadioGroup.Item class="settings-segmented-option" value="default-folder">Default folder</RadioGroup.Item>
          </RadioGroup.Root>
        </div>
        <div class="settings-row settings-row-separated">
          <span class="settings-row-icon" aria-hidden="true"><IconRestore size={17} stroke={1.55} /></span>
          <span class="settings-row-copy">
            <strong>Reuse empty threads</strong>
            <small>Use an existing untouched thread instead of adding another empty one.</small>
          </span>
          <Switch.Root
            class="toggle-control"
            aria-label="Reuse empty threads"
            checked={preferences.reuseEmptyThreads}
            onCheckedChange={(value) => setPreference('reuseEmptyThreads', value)}
          >
            <span class="toggle-track" aria-hidden="true"><Switch.Thumb class="toggle-thumb" /></span>
          </Switch.Root>
        </div>
        <div class="settings-row settings-row-separated">
          <span class="settings-row-icon" aria-hidden="true"><IconLayoutSidebarRight size={17} stroke={1.55} /></span>
          <span class="settings-row-copy">
            <strong>Project explorer on startup</strong>
            <small>Choose whether the project explorer starts open or collapsed.</small>
          </span>
          <RadioGroup.Root
            class="settings-segmented-control"
            aria-label="Project explorer on startup"
            orientation="horizontal"
            value={preferences.explorerStartup}
            onValueChange={(value) => setPreference('explorerStartup', value === 'collapsed' ? 'collapsed' : 'open')}
          >
            <RadioGroup.Item class="settings-segmented-option" value="open">Open</RadioGroup.Item>
            <RadioGroup.Item class="settings-segmented-option" value="collapsed">Collapsed</RadioGroup.Item>
          </RadioGroup.Root>
        </div>
      </div>
    {:else if category === 'appearance'}
      <div class="settings-card">
        <div class="settings-row">
          <span class="settings-row-icon" aria-hidden="true"><IconActivity size={17} stroke={1.55} /></span>
          <span class="settings-row-copy">
            <strong>Interface motion</strong>
            <small>Follow the system preference or remove interface animation.</small>
          </span>
          <RadioGroup.Root
            class="settings-segmented-control"
            aria-label="Interface motion"
            orientation="horizontal"
            value={preferences.motionMode}
            onValueChange={(value) => setPreference('motionMode', value === 'reduced' ? 'reduced' : 'system')}
          >
            <RadioGroup.Item class="settings-segmented-option" value="system">System</RadioGroup.Item>
            <RadioGroup.Item class="settings-segmented-option" value="reduced">Reduced</RadioGroup.Item>
          </RadioGroup.Root>
        </div>
        <div class="settings-row settings-row-separated">
          <span class="settings-row-icon" aria-hidden="true"><IconZoomIn size={17} stroke={1.55} /></span>
          <span class="settings-row-copy">
            <strong>Interface zoom</strong>
            <small>Scale the entire app without changing the window size.</small>
          </span>
          <div class="settings-range-control">
            <output>{preferences.interfaceZoom}%</output>
            <Slider.Root
              class="settings-slider"
              aria-label="Interface zoom"
              type="single"
              min={INTERFACE_ZOOM_RANGE.min}
              max={INTERFACE_ZOOM_RANGE.max}
              step={10}
              value={preferences.interfaceZoom}
              onValueChange={(value) => setPreference('interfaceZoom', value)}
            >
              {#snippet children({ thumbItems })}
                <span class="settings-slider-track"><Slider.Range class="settings-slider-range" /></span>
                {#each thumbItems as { index } (index)}
                  <Slider.Thumb class="settings-slider-thumb" {index} />
                {/each}
              {/snippet}
            </Slider.Root>
          </div>
        </div>
        {#if isLinux}
          <div class="settings-row settings-row-separated">
            <span class="settings-row-icon" aria-hidden="true"><IconContrast size={17} stroke={1.55} /></span>
            <span class="settings-row-copy">
              <strong>Background transparency</strong>
              <small>Show the desktop behind LoopCode when the desktop environment supports it.</small>
            </span>
            <div class="settings-range-control">
              <output>{linuxShellTransparency}%</output>
              <Slider.Root
                class="settings-slider"
                aria-label="Background transparency"
                type="single"
                min={linuxShellTransparencyRange.min}
                max={linuxShellTransparencyRange.max}
                step={1}
                value={linuxShellTransparency}
                onValueChange={setLinuxShellTransparency}
              >
                {#snippet children({ thumbItems })}
                  <span class="settings-slider-track"><Slider.Range class="settings-slider-range" /></span>
                  {#each thumbItems as { index } (index)}
                    <Slider.Thumb class="settings-slider-thumb" {index} />
                  {/each}
                {/snippet}
              </Slider.Root>
            </div>
          </div>
        {/if}
      </div>
    {:else if category === 'conversation'}
      <div class="settings-card">
        <div class="settings-row">
          <span class="settings-row-icon" aria-hidden="true"><IconListDetails size={17} stroke={1.55} /></span>
          <span class="settings-row-copy">
            <strong>Transcript spacing</strong>
            <small>Use the standard message spacing or fit more transcript entries on screen.</small>
          </span>
          <RadioGroup.Root
            class="settings-segmented-control"
            aria-label="Transcript spacing"
            orientation="horizontal"
            value={preferences.transcriptDensity}
            onValueChange={(value) => setPreference('transcriptDensity', value === 'compact' ? 'compact' : 'comfortable')}
          >
            <RadioGroup.Item class="settings-segmented-option" value="comfortable">Comfortable</RadioGroup.Item>
            <RadioGroup.Item class="settings-segmented-option" value="compact">Compact</RadioGroup.Item>
          </RadioGroup.Root>
        </div>
        <div class="settings-row settings-row-separated">
          <span class="settings-row-icon" aria-hidden="true"><IconArrowsHorizontal size={17} stroke={1.55} /></span>
          <span class="settings-row-copy">
            <strong>Content width</strong>
            <small>Set the shared maximum width for messages, questions, and the composer.</small>
          </span>
          <div class="settings-range-control">
            <output>{preferences.contentWidth}px</output>
            <Slider.Root
              class="settings-slider"
              aria-label="Content width"
              type="single"
              min={CONTENT_WIDTH_RANGE.min}
              max={CONTENT_WIDTH_RANGE.max}
              step={20}
              value={preferences.contentWidth}
              onValueChange={(value) => setPreference('contentWidth', value)}
            >
              {#snippet children({ thumbItems })}
                <span class="settings-slider-track"><Slider.Range class="settings-slider-range" /></span>
                {#each thumbItems as { index } (index)}
                  <Slider.Thumb class="settings-slider-thumb" {index} />
                {/each}
              {/snippet}
            </Slider.Root>
          </div>
        </div>
        <div class="settings-row settings-row-separated">
          <span class="settings-row-icon" aria-hidden="true"><IconCode size={17} stroke={1.55} /></span>
          <span class="settings-row-copy">
            <strong>Wrap message code</strong>
            <small>Wrap long lines in message code blocks instead of scrolling horizontally.</small>
          </span>
          <Switch.Root
            class="toggle-control"
            aria-label="Wrap message code"
            checked={preferences.wrapCode}
            onCheckedChange={(value) => setPreference('wrapCode', value)}
          >
            <span class="toggle-track" aria-hidden="true"><Switch.Thumb class="toggle-thumb" /></span>
          </Switch.Root>
        </div>
        <div class="settings-row settings-row-separated">
          <span class="settings-row-icon" aria-hidden="true"><IconActivity size={17} stroke={1.55} /></span>
          <span class="settings-row-copy">
            <strong>Follow new output</strong>
            <small>Keep the transcript pinned to the bottom while an agent is responding.</small>
          </span>
          <Switch.Root
            class="toggle-control"
            aria-label="Follow new output"
            checked={preferences.autoFollowOutput}
            onCheckedChange={(value) => setPreference('autoFollowOutput', value)}
          >
            <span class="toggle-track" aria-hidden="true"><Switch.Thumb class="toggle-thumb" /></span>
          </Switch.Root>
        </div>
        <div class="settings-row settings-row-separated">
          <span class="settings-row-icon" aria-hidden="true"><IconClock size={17} stroke={1.55} /></span>
          <span class="settings-row-copy">
            <strong>Message timestamps</strong>
            <small>Show the sender and local time above each user and agent message.</small>
          </span>
          <Switch.Root
            class="toggle-control"
            aria-label="Message timestamps"
            checked={preferences.showMessageTimestamps}
            onCheckedChange={(value) => setPreference('showMessageTimestamps', value)}
          >
            <span class="toggle-track" aria-hidden="true"><Switch.Thumb class="toggle-thumb" /></span>
          </Switch.Root>
        </div>
        <div class="settings-row settings-row-separated">
          <span class="settings-row-icon" aria-hidden="true"><IconWriting size={17} stroke={1.55} /></span>
          <span class="settings-row-copy">
            <strong>Composer spellcheck</strong>
            <small>Use the operating system spellchecker while writing prompts.</small>
          </span>
          <Switch.Root
            class="toggle-control"
            aria-label="Composer spellcheck"
            checked={preferences.composerSpellcheck}
            onCheckedChange={(value) => setPreference('composerSpellcheck', value)}
          >
            <span class="toggle-track" aria-hidden="true"><Switch.Thumb class="toggle-thumb" /></span>
          </Switch.Root>
        </div>
        <div class="settings-row settings-row-separated">
          <span class="settings-row-icon" aria-hidden="true"><IconAt size={17} stroke={1.55} /></span>
          <span class="settings-row-copy">
            <strong>Composer autocomplete</strong>
            <small>Suggest project files, folders, and skills after @ or $.</small>
          </span>
          <Switch.Root
            class="toggle-control"
            aria-label="Composer autocomplete"
            checked={preferences.composerAutocomplete}
            onCheckedChange={(value) => setPreference('composerAutocomplete', value)}
          >
            <span class="toggle-track" aria-hidden="true"><Switch.Thumb class="toggle-thumb" /></span>
          </Switch.Root>
        </div>
        <div class="settings-row settings-row-separated">
          <span class="settings-row-icon" aria-hidden="true"><IconKeyboard size={17} stroke={1.55} /></span>
          <span class="settings-row-copy">
            <strong>Send shortcut</strong>
            <small>Choose whether Enter sends or inserts a new line in the composer.</small>
          </span>
          <RadioGroup.Root
            class="settings-segmented-control"
            aria-label="Send shortcut"
            orientation="horizontal"
            value={preferences.sendShortcut}
            onValueChange={(value) => setPreference('sendShortcut', value === 'modifier-enter' ? 'modifier-enter' : 'enter')}
          >
            <RadioGroup.Item class="settings-segmented-option" value="enter">Enter</RadioGroup.Item>
            <RadioGroup.Item class="settings-segmented-option" value="modifier-enter">Modifier + Enter</RadioGroup.Item>
          </RadioGroup.Root>
        </div>
      </div>
    {:else if category === 'agents'}
      <div class="settings-card">
        <div class="settings-row">
          <span class="settings-row-icon" aria-hidden="true"><IconShieldLock size={17} stroke={1.55} /></span>
          <span class="settings-row-copy">
            <strong>Agent permissions</strong>
            <small>Restricted asks before commands. Full access automatically approves permission requests.</small>
          </span>
          <RadioGroup.Root
            class="settings-segmented-control"
            aria-label="Agent permissions"
            orientation="horizontal"
            value={permissionMode}
            onValueChange={(value) => setPermissionMode(value === 'full' ? 'full' : 'restricted')}
          >
            <RadioGroup.Item class="settings-segmented-option" value="restricted">Restricted</RadioGroup.Item>
            <RadioGroup.Item class="settings-segmented-option" value="full">Full access</RadioGroup.Item>
          </RadioGroup.Root>
        </div>
        <div class="settings-row settings-row-separated">
          <span class="settings-row-icon" aria-hidden="true"><IconRobot size={17} stroke={1.55} /></span>
          <span class="settings-row-copy">
            <strong>Default provider</strong>
            <small>Use this provider when LoopCode creates or reuses an empty thread.</small>
          </span>
          <select
            class="settings-select"
            aria-label="Default provider"
            value={preferences.defaultProviderId}
            onchange={(event) => setPreference('defaultProviderId', selectValue(event))}
          >
            {#each profiles as profile (profile.id)}
              <option value={profile.id}>{profile.label}</option>
            {/each}
          </select>
        </div>
        {#each profiles as profile (profile.id)}
          {@const catalog = catalogs[profile.id]}
          <div class="settings-row settings-row-separated">
            <span class="settings-row-icon" aria-hidden="true"><img class="settings-provider-icon" src={profile.icon} alt="" /></span>
            <span class="settings-row-copy">
              <strong>{profile.label} model</strong>
              <small>{catalog?.status === 'ready' ? `Use this model when a new ${profile.label} thread starts.` : catalog?.status === 'error' ? `${profile.label} models are unavailable.` : `Loading ${profile.label} models…`}</small>
            </span>
            <select
              class="settings-select settings-model-select"
              aria-label={`${profile.label} model`}
              disabled={catalog?.status !== 'ready'}
              value={providerModelValue(profile.id)}
              onchange={(event) => setProviderModelDefault(profile.id, selectValue(event))}
            >
              <option value="">Provider default</option>
              {#each catalog?.models ?? [] as model (model.id)}
                <option value={model.id}>{model.name}</option>
              {/each}
            </select>
          </div>
        {/each}
      </div>
    {:else if category === 'terminal'}
      <div class="settings-card">
        <div class="settings-row">
          <span class="settings-row-icon" aria-hidden="true"><IconTerminal2 size={17} stroke={1.55} /></span>
          <span class="settings-row-copy">
            <strong>Terminal drawer height</strong>
            <small>Set the default terminal height. Dragging the drawer handle also updates it.</small>
          </span>
          <div class="settings-range-control">
            <output>{terminalHeight}px</output>
            <Slider.Root
              class="settings-slider"
              aria-label="Terminal drawer height"
              type="single"
              min={TERMINAL_HEIGHT_RANGE.min}
              max={TERMINAL_HEIGHT_RANGE.max}
              step={10}
              value={terminalHeight}
              onValueChange={setTerminalHeight}
            >
              {#snippet children({ thumbItems })}
                <span class="settings-slider-track"><Slider.Range class="settings-slider-range" /></span>
                {#each thumbItems as { index } (index)}
                  <Slider.Thumb class="settings-slider-thumb" {index} />
                {/each}
              {/snippet}
            </Slider.Root>
          </div>
        </div>
        <div class="settings-row settings-row-separated">
          <span class="settings-row-icon" aria-hidden="true"><IconTextSize size={17} stroke={1.55} /></span>
          <span class="settings-row-copy">
            <strong>Terminal text size</strong>
            <small>Change the terminal font without scaling the rest of the app.</small>
          </span>
          <div class="settings-range-control">
            <output>{preferences.terminalFontSize}px</output>
            <Slider.Root
              class="settings-slider"
              aria-label="Terminal text size"
              type="single"
              min={TERMINAL_FONT_SIZE_RANGE.min}
              max={TERMINAL_FONT_SIZE_RANGE.max}
              step={1}
              value={preferences.terminalFontSize}
              onValueChange={(value) => setPreference('terminalFontSize', value)}
            >
              {#snippet children({ thumbItems })}
                <span class="settings-slider-track"><Slider.Range class="settings-slider-range" /></span>
                {#each thumbItems as { index } (index)}
                  <Slider.Thumb class="settings-slider-thumb" {index} />
                {/each}
              {/snippet}
            </Slider.Root>
          </div>
        </div>
        <div class="settings-row settings-row-separated">
          <span class="settings-row-icon" aria-hidden="true"><IconHistory size={17} stroke={1.55} /></span>
          <span class="settings-row-copy">
            <strong>Terminal scrollback</strong>
            <small>Keep this many lines available above the visible terminal.</small>
          </span>
          <select
            class="settings-select"
            aria-label="Terminal scrollback"
            value={preferences.terminalScrollback}
            onchange={(event) => setPreference('terminalScrollback', Number(selectValue(event)))}
          >
            <option value={1000}>1,000 lines</option>
            <option value={5000}>5,000 lines</option>
            <option value={10000}>10,000 lines</option>
          </select>
        </div>
      </div>
    {:else}
      <div class="settings-card">
        <div class="settings-row">
          <span class="settings-row-icon" aria-hidden="true"><IconDownload size={17} stroke={1.55} /></span>
          <span class="settings-row-copy">
            <strong>ACP diagnostics</strong>
            <small>Export redacted provider lifecycle, RPC envelope, error, and stderr logs.</small>
          </span>
          <button class="settings-action" disabled={exportState === 'exporting'} onclick={exportLogs} aria-live="polite">
            {exportState === 'exporting' ? 'Exporting…' : exportState === 'exported' ? 'Exported' : exportState === 'error' ? 'Try again' : 'Export'}
          </button>
        </div>
        <div class="settings-row settings-row-separated">
          <span class="settings-row-icon" aria-hidden="true"><IconArchive size={17} stroke={1.55} /></span>
          <span class="settings-row-copy">
            <strong>Delete archived threads</strong>
            <small>Permanently delete {archivedThreadCount} archived {archivedThreadCount === 1 ? 'thread' : 'threads'} and their history.</small>
          </span>
          <button
            class="settings-action danger"
            disabled={archivedThreadCount === 0}
            onclick={() => { pendingDataAction = 'archived'; }}
          >Delete</button>
        </div>
        <div class="settings-row settings-row-separated">
          <span class="settings-row-icon" aria-hidden="true"><IconTrash size={17} stroke={1.55} /></span>
          <span class="settings-row-copy">
            <strong>Clear composer drafts</strong>
            <small>Discard unsent text, references, and images from {draftThreadCount} {draftThreadCount === 1 ? 'thread' : 'threads'}.</small>
          </span>
          <button
            class="settings-action danger"
            disabled={draftThreadCount === 0}
            onclick={() => { pendingDataAction = 'drafts'; }}
          >Clear</button>
        </div>
        <div class="settings-row settings-row-separated">
          <span class="settings-row-icon" aria-hidden="true"><IconRestore size={17} stroke={1.55} /></span>
          <span class="settings-row-copy">
            <strong>Reset interface settings</strong>
            <small>Restore settings, panel sizes, terminal size, and zoom without deleting projects or threads.</small>
          </span>
          <button class="settings-action" onclick={resetSettings}>Reset</button>
        </div>
      </div>
    {/if}
  </div>
</section>

<AlertDialog.Root
  open={Boolean(pendingDataAction)}
  onOpenChange={(open) => { if (!open) pendingDataAction = undefined; }}
>
  <AlertDialog.Portal>
    <AlertDialog.Overlay class="modal-overlay" />
    <AlertDialog.Content class="permission-modal confirmation-modal">
      <AlertDialog.Title>{pendingDataAction === 'archived' ? 'Delete archived threads?' : 'Clear composer drafts?'}</AlertDialog.Title>
      <AlertDialog.Description>
        {pendingDataAction === 'archived'
          ? `${archivedThreadCount} archived ${archivedThreadCount === 1 ? 'thread' : 'threads'} and their history will be permanently deleted.`
          : `Unsent content from ${draftThreadCount} ${draftThreadCount === 1 ? 'thread' : 'threads'} will be permanently discarded.`}
      </AlertDialog.Description>
      <footer class="confirmation-actions">
        <AlertDialog.Cancel>Cancel</AlertDialog.Cancel>
        <button class="danger" onclick={confirmDataAction}>
          {pendingDataAction === 'archived' ? 'Delete' : 'Clear'}
        </button>
      </footer>
    </AlertDialog.Content>
  </AlertDialog.Portal>
</AlertDialog.Root>
