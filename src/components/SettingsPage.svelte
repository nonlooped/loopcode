<script lang="ts">
  import { tick } from 'svelte';
  import { fly } from 'svelte/transition';
  import { IconArrowLeft, IconCheck, IconPlus, IconTrash } from '@tabler/icons-svelte';
  import { RadioGroup, Slider, Switch } from 'bits-ui';

  import { exportDiagnostics } from '../services/native';
  import type { HarnessProfile, PermissionMode, ProviderModelCatalog } from '../types';
  import {
    CONTENT_WIDTH_RANGE,
    INTERFACE_ZOOM_RANGE,
    TERMINAL_FONT_SIZE_RANGE,
    THEME_OPTIONS,
    type AppPreferences,
    type ProviderPreference,
    type SettingsCategory,
  } from '../utils/app-settings';
  import {
    providerCanToggle,
    providerDisplayStatus,
    providerVersionLabel,
  } from '../utils/provider-availability';

  interface Props {
    category: SettingsCategory;
    preferences: AppPreferences;
    profiles: HarnessProfile[];
    baseProfiles: HarnessProfile[];
    catalogs: Record<string, ProviderModelCatalog>;
    providerVersions: Record<string, string>;
    providerAuthStatuses: Record<string, boolean>;
    permissionMode: PermissionMode;
    reducedMotion: boolean;
    setPreference: <K extends keyof AppPreferences>(key: K, value: AppPreferences[K]) => void;
    setProviderPreference: (profileId: string, preference: ProviderPreference) => void;
    setPermissionMode: (value: PermissionMode) => void;
    defaultWorkingFolder: string;
    chooseDefaultWorkingFolder: () => void;
    resetSettings: () => void;
  }

  const {
    category,
    preferences,
    profiles,
    baseProfiles,
    catalogs,
    providerVersions,
    providerAuthStatuses,
    permissionMode,
    reducedMotion,
    setPreference,
    setProviderPreference,
    setPermissionMode,
    defaultWorkingFolder,
    chooseDefaultWorkingFolder,
    resetSettings,
  }: Props = $props();
  let exportState = $state<'idle' | 'exporting' | 'exported' | 'error'>('idle');
  let selectedProviderId = $state('');
  const selectedProvider = $derived(profiles.find((profile) => profile.id === selectedProviderId));
  const selectedBaseProvider = $derived(baseProfiles.find((profile) => profile.id === selectedProviderId));
  const titleProfiles = $derived(
    profiles.filter((profile) =>
      profile.titleGeneration
      && (
        profile.id === preferences.titleProviderId
        || (preferences.providerSettings[profile.id]?.enabled !== false && catalogs[profile.id]?.status === 'ready')
      )
    ),
  );
  const titleCatalog = $derived(catalogs[preferences.titleProviderId]);

  const categoryCopy = {
    general: ['General', 'Choose what LoopCode opens and where new threads start.'],
    appearance: ['Appearance', 'Choose the color mode and theme, then adjust interface layout.'],
    conversation: ['Conversation', 'Control how transcript content is displayed.'],
    composer: ['Composer', 'Choose how prompt editing and sending behave.'],
    agents: ['Agents and permissions', 'Control agent access and thread title generation.'],
    providers: ['Providers', 'Manage provider availability, defaults, connections, and models.'],
    terminal: ['Terminal', 'Tune the terminal drawer and its retained output.'],
    diagnostics: ['Diagnostics', 'Export troubleshooting data or restore interface defaults.'],
  } satisfies Record<SettingsCategory, [string, string]>;
  const pageCopy = $derived(
    category === 'providers' && selectedProvider
      ? [selectedProvider.label, `Configure the ${selectedProvider.label} ACP connection and models.`]
      : categoryCopy[category],
  );

  $effect(() => {
    if (category !== 'providers') selectedProviderId = '';
  });

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

  function setTheme(value: string) {
    const theme = THEME_OPTIONS.find((option) => option.id === value);
    if (theme) setPreference('theme', theme.id);
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

  function setTitleProvider(profileId: string) {
    setPreference('titleProviderId', profileId);
    setPreference('titleModelId', '');
  }

  function setTitleMode(value: string) {
    if (value === 'local') {
      setPreference('automaticTitleGeneration', false);
      return;
    }
    setPreference('automaticTitleGeneration', true);
    setTitleProvider(value);
  }

  function providerPreference(profileId: string) {
    return preferences.providerSettings[profileId] ?? {};
  }

  function providerStatus(profileId: string) {
    return providerDisplayStatus(
      profileId,
      providerPreference(profileId).enabled !== false,
      catalogs[profileId],
      providerAuthStatuses[profileId],
    );
  }

  function providerVersion(profileId: string) {
    return providerVersionLabel(catalogs[profileId], providerVersions[profileId]);
  }

  function showProvider(profileId: string) {
    selectedProviderId = profileId;
    void tick().then(() => document.getElementById('settings-title')?.focus());
  }

  function hideProvider() {
    const profileId = selectedProviderId;
    selectedProviderId = '';
    void tick().then(() => document.getElementById(`provider-setting-${profileId}`)?.focus());
  }

  function inputValue(event: Event) {
    return event.currentTarget instanceof HTMLInputElement ? event.currentTarget.value : '';
  }

  function updateSelectedProvider(patch: Partial<ProviderPreference>) {
    if (!selectedProvider) return;
    setProviderPreference(selectedProvider.id, { ...providerPreference(selectedProvider.id), ...patch });
  }

  function updateCustomModel(index: number, field: 'id' | 'name', value: string) {
    if (!selectedProvider) return;
    const models = [...(providerPreference(selectedProvider.id).models ?? [])];
    const model = models[index];
    if (!model) return;
    models[index] = { ...model, [field]: value };
    updateSelectedProvider({ models });
  }

  function addCustomModel() {
    if (!selectedProvider) return;
    const models = providerPreference(selectedProvider.id).models ?? [];
    let suffix = models.length + 1;
    while (models.some((model) => model.id === `custom-model-${suffix}`)) suffix += 1;
    updateSelectedProvider({ models: [...models, { id: `custom-model-${suffix}`, name: 'Custom model' }] });
  }

  function removeCustomModel(index: number) {
    if (!selectedProvider) return;
    updateSelectedProvider({
      models: (providerPreference(selectedProvider.id).models ?? []).filter((_, modelIndex) => modelIndex !== index),
    });
  }

  function resetSelectedProvider() {
    if (!selectedProvider) return;
    setProviderPreference(selectedProvider.id, {});
    setProviderModelDefault(selectedProvider.id, '');
  }
</script>

<section
  class="settings-page"
  aria-labelledby="settings-title"
  in:fly|global={{ y: reducedMotion ? 0 : 4, duration: reducedMotion ? 0 : 180 }}
>
  <div class="settings-column">
    <header class="settings-page-header">
      {#if category === 'providers' && selectedProvider}
        <button class="provider-detail-back" onclick={hideProvider}>
          <IconArrowLeft size={13} stroke={1.7} /> All providers
        </button>
      {/if}
      <h1 id="settings-title" tabindex="-1">{pageCopy[0]}</h1>
      <p>{pageCopy[1]}</p>
    </header>

    {#if category === 'general'}
      <div class="settings-card">
        <div class="settings-row">
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
          <span class="settings-row-copy">
            <strong>New threads</strong>
            <small title={defaultWorkingFolder}>Use the selected project or {defaultWorkingFolder || 'the system folder'}.</small>
          </span>
          <div class="settings-combined-control">
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
            {#if preferences.newThreadProject === 'default-folder'}
              <button class="settings-action" onclick={chooseDefaultWorkingFolder}>Choose folder…</button>
            {/if}
          </div>
        </div>
      </div>
    {:else if category === 'appearance'}
      <div class="settings-card">
        <div class="settings-visual-section">
          <span class="settings-row-copy">
            <strong>Color mode</strong>
            <small>Follow the system appearance or keep LoopCode light or dark.</small>
          </span>
          <RadioGroup.Root
            class="settings-mode-grid"
            aria-label="Color mode"
            orientation="horizontal"
            value={preferences.colorMode}
            onValueChange={(value) => setPreference('colorMode', value === 'light' || value === 'dark' ? value : 'system')}
          >
            {#each ['system', 'light', 'dark'] as mode (mode)}
              <RadioGroup.Item class={`settings-choice-card settings-mode-${mode}`} value={mode}>
                <span class="settings-mode-preview" aria-hidden="true">
                  <span class="settings-mode-sidebar"></span>
                  <span class="settings-mode-content">
                    <i></i><i></i><i></i>
                    <b></b>
                  </span>
                </span>
                <span class="settings-choice-label">{mode === 'system' ? 'System' : mode === 'light' ? 'Light' : 'Dark'}</span>
                <span class="settings-choice-check" aria-hidden="true"><IconCheck size={11} stroke={2} /></span>
              </RadioGroup.Item>
            {/each}
          </RadioGroup.Root>
        </div>
        <div class="settings-visual-section settings-row-separated">
          <span class="settings-row-copy">
            <strong>Theme</strong>
            <small>Preview each palette in light and dark mode.</small>
          </span>
          <RadioGroup.Root
            class="settings-theme-grid"
            aria-label="Theme"
            value={preferences.theme}
            onValueChange={setTheme}
          >
            {#each THEME_OPTIONS as theme (theme.id)}
              <RadioGroup.Item class={`settings-choice-card settings-theme-choice theme-preview-${theme.id}`} value={theme.id}>
                <span class="settings-theme-preview" aria-hidden="true">
                  <span class="settings-theme-preview-light"></span>
                  <span class="settings-theme-preview-dark"></span>
                </span>
                <span class="settings-choice-label">{theme.label}</span>
                <span class="settings-choice-check" aria-hidden="true"><IconCheck size={11} stroke={2} /></span>
              </RadioGroup.Item>
            {/each}
          </RadioGroup.Root>
        </div>
        <div class="settings-visual-section settings-row-separated">
          <span class="settings-row-copy">
            <strong>Sidebar thread spacing</strong>
            <small>Preview how much detail each thread shows in the sidebar.</small>
          </span>
          <RadioGroup.Root
            class="settings-density-grid"
            aria-label="Sidebar thread spacing"
            orientation="horizontal"
            value={preferences.compactSessionRows ? 'compact' : 'comfortable'}
            onValueChange={(value) => setPreference('compactSessionRows', value === 'compact')}
          >
            {#each ['comfortable', 'compact'] as density (density)}
              <RadioGroup.Item class="settings-choice-card settings-density-choice" value={density}>
                <span class:compact={density === 'compact'} class="settings-density-preview settings-sidebar-density-preview" aria-hidden="true">
                  <i></i><i></i><i></i><i></i>
                </span>
                <span class="settings-choice-label">{density === 'compact' ? 'Compact' : 'Comfortable'}</span>
                <span class="settings-choice-check" aria-hidden="true"><IconCheck size={11} stroke={2} /></span>
              </RadioGroup.Item>
            {/each}
          </RadioGroup.Root>
        </div>
        <div class="settings-visual-section settings-row-separated">
          <span class="settings-row-copy">
            <strong>Transcript spacing</strong>
            <small>Preview the space between transcript entries.</small>
          </span>
          <RadioGroup.Root
            class="settings-density-grid"
            aria-label="Transcript spacing"
            orientation="horizontal"
            value={preferences.transcriptDensity}
            onValueChange={(value) => setPreference('transcriptDensity', value === 'compact' ? 'compact' : 'comfortable')}
          >
            {#each ['comfortable', 'compact'] as density (density)}
              <RadioGroup.Item class="settings-choice-card settings-density-choice" value={density}>
                <span class:compact={density === 'compact'} class="settings-density-preview settings-transcript-density-preview" aria-hidden="true">
                  <i></i><i></i><i></i>
                </span>
                <span class="settings-choice-label">{density === 'compact' ? 'Compact' : 'Comfortable'}</span>
                <span class="settings-choice-check" aria-hidden="true"><IconCheck size={11} stroke={2} /></span>
              </RadioGroup.Item>
            {/each}
          </RadioGroup.Root>
        </div>
        <div class="settings-row settings-row-separated">
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
      </div>
    {:else if category === 'conversation'}
      <div class="settings-card">
        <div class="settings-row">
          <span class="settings-row-copy">
            <strong>Wrap message code</strong>
            <small>Wrap long lines in message code blocks instead of scrolling horizontally.</small>
          </span>
          <div class="settings-preview-control">
            <span class:wrap={preferences.wrapCode} class="settings-code-preview" aria-hidden="true"><code>const model = provider.create(options);</code></span>
            <Switch.Root
              class="toggle-control"
              aria-label="Wrap message code"
              checked={preferences.wrapCode}
              onCheckedChange={(value) => setPreference('wrapCode', value)}
            >
              <span class="toggle-track" aria-hidden="true"><Switch.Thumb class="toggle-thumb" /></span>
            </Switch.Root>
          </div>
        </div>
        <div class="settings-row settings-row-separated">
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
      </div>
    {:else if category === 'composer'}
      <div class="settings-card">
        <div class="settings-row">
          <span class="settings-row-copy">
            <strong>Spellcheck</strong>
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
          <span class="settings-row-copy">
            <strong>Send shortcut</strong>
            <small>Choose whether Enter sends or inserts a new line in the composer.</small>
          </span>
          <RadioGroup.Root
            class="settings-segmented-control settings-shortcut-control"
            aria-label="Send shortcut"
            orientation="horizontal"
            value={preferences.sendShortcut}
            onValueChange={(value) => setPreference('sendShortcut', value === 'modifier-enter' ? 'modifier-enter' : 'enter')}
          >
            <RadioGroup.Item class="settings-segmented-option settings-shortcut-option" value="enter"><kbd>Enter</kbd></RadioGroup.Item>
            <RadioGroup.Item class="settings-segmented-option settings-shortcut-option" value="modifier-enter"><kbd>Ctrl/⌘</kbd><span>+</span><kbd>Enter</kbd></RadioGroup.Item>
          </RadioGroup.Root>
        </div>
      </div>
    {:else if category === 'agents'}
      <div class="settings-card">
        <div class="settings-row">
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
          <span class="settings-row-copy">
            <strong>Thread titles</strong>
            <small>{preferences.automaticTitleGeneration ? 'Use a separate provider connection to name new threads.' : 'Use the first few words of the prompt.'}</small>
          </span>
          <div class="settings-combined-control">
            <select
              class="settings-select"
              aria-label="Thread title source"
              value={preferences.automaticTitleGeneration ? preferences.titleProviderId : 'local'}
              onchange={(event) => setTitleMode(selectValue(event))}
            >
              <option value="local">Local from prompt</option>
              {#each titleProfiles as profile (profile.id)}
                <option value={profile.id} disabled={catalogs[profile.id]?.status !== 'ready'}>{profile.label}</option>
              {/each}
            </select>
            {#if preferences.automaticTitleGeneration}
              <select
                class="settings-select settings-model-select"
                aria-label="Title model"
                disabled={titleCatalog?.status !== 'ready' || providerPreference(preferences.titleProviderId).enabled === false}
                value={preferences.titleModelId}
                onchange={(event) => setPreference('titleModelId', selectValue(event))}
              >
                <option value="">Provider default</option>
                {#if preferences.titleModelId && !titleCatalog?.models.some((model) => model.id === preferences.titleModelId)}
                  <option value={preferences.titleModelId} disabled>{preferences.titleModelId} (unavailable)</option>
                {/if}
                {#each titleCatalog?.models ?? [] as model (model.id)}
                  <option value={model.id}>{model.name}</option>
                {/each}
              </select>
            {/if}
          </div>
        </div>
      </div>
    {:else if category === 'providers'}
      {#if selectedProvider && selectedBaseProvider}
        {@const setting = providerPreference(selectedProvider.id)}
        {@const catalog = catalogs[selectedProvider.id]}
        <div class="settings-card">
          <div class="settings-row">
            <span class="settings-row-copy">
              <strong>Default model</strong>
              <small>{catalog?.status === 'ready' ? `Use this model when a new ${selectedProvider.label} thread starts.` : catalog?.status === 'unavailable' ? catalog.error : `Loading ${selectedProvider.label} models…`}</small>
            </span>
            <select
              class="settings-select settings-model-select"
              aria-label={`${selectedProvider.label} default model`}
              disabled={catalog?.status !== 'ready' || setting.enabled === false}
              value={providerModelValue(selectedProvider.id)}
              onchange={(event) => setProviderModelDefault(selectedProvider.id, selectValue(event))}
            >
              <option value="">Provider default</option>
              {#each catalog?.models ?? [] as model (model.id)}
                <option value={model.id}>{model.name}</option>
              {/each}
            </select>
          </div>
          <label class="settings-row settings-row-separated">
              <span class="settings-row-copy">
              <strong>Provider command path</strong>
              <small>Override the provider executable while keeping its required ACP arguments.</small>
            </span>
            <input
              class="settings-text-input settings-path-input"
              aria-label="Provider command path"
              spellcheck="false"
              value={setting.command ?? selectedBaseProvider.command}
              onchange={(event) => updateSelectedProvider({ command: inputValue(event) })}
            />
          </label>
          <div class="provider-model-editor settings-row-separated">
            <div class="provider-model-heading">
              <span>
                <strong>Custom models</strong>
                <small>Add models that are not advertised by the provider.</small>
              </span>
              <button class="settings-action" onclick={addCustomModel}><IconPlus size={13} stroke={1.7} /> Add model</button>
            </div>
            {#if (setting.models ?? []).length === 0}
              <p class="provider-model-empty">No custom models.</p>
            {:else}
              <div class="provider-model-list">
                {#each setting.models ?? [] as model, index (`${index}-${model.id}`)}
                  <div class="provider-model-row">
                    <label>
                      <span>Name</span>
                      <input
                        class="settings-text-input"
                        aria-label={`Custom model ${index + 1} name`}
                        value={model.name}
                        onchange={(event) => updateCustomModel(index, 'name', inputValue(event))}
                      />
                    </label>
                    <label>
                      <span>Model ID</span>
                      <input
                        class="settings-text-input"
                        aria-label={`Custom model ${index + 1} ID`}
                        spellcheck="false"
                        value={model.id}
                        onchange={(event) => updateCustomModel(index, 'id', inputValue(event))}
                      />
                    </label>
                    <button
                      class="provider-model-remove"
                      aria-label={`Remove ${model.name}`}
                      title={`Remove ${model.name}`}
                      onclick={() => removeCustomModel(index)}
                    ><IconTrash size={14} stroke={1.7} /></button>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
          {#if Object.keys(setting).length > 0 || preferences.providerModelDefaults[selectedProvider.id]}
            <div class="settings-row settings-row-separated">
              <span class="settings-row-copy">
                <strong>Restore provider defaults</strong>
                <small>Reset the command path, custom models, enabled state, and default model.</small>
              </span>
              <button class="settings-action" onclick={resetSelectedProvider}>Reset</button>
            </div>
          {/if}
        </div>
      {:else}
        <div class="settings-card">
          <div class="settings-row">
            <span class="settings-row-copy">
              <strong>Default provider</strong>
              <small>Use this provider when LoopCode creates a new thread.</small>
            </span>
            <select
              class="settings-select"
              aria-label="Default provider"
              value={preferences.defaultProviderId}
              onchange={(event) => setPreference('defaultProviderId', selectValue(event))}
            >
              {#each profiles as profile (profile.id)}
                <option
                  value={profile.id}
                  disabled={catalogs[profile.id]?.status !== 'ready' || providerPreference(profile.id).enabled === false}
                >{profile.label}</option>
              {/each}
            </select>
          </div>
          {#each profiles as profile (profile.id)}
            {@const enabled = providerPreference(profile.id).enabled !== false}
            {@const version = providerVersion(profile.id)}
            {@const status = providerStatus(profile.id)}
            <div class="settings-row settings-row-separated provider-settings-row">
              <button id={`provider-setting-${profile.id}`} class="provider-settings-open" onclick={() => showProvider(profile.id)}>
                <span class="provider-settings-icon" aria-hidden="true"><img class:brand-color-icon={profile.iconMode === 'brand'} class="settings-provider-icon" src={profile.icon} alt="" /></span>
                <span class="settings-row-copy">
                  <strong>{profile.label}{#if version} <span class="provider-version">{version}</span>{/if}</strong>
                  <small
                    class:authenticated={status === 'Authenticated' || status === 'Connected'}
                    class:disabled={status === 'Disabled'}
                    class:not-installed={status === 'Not installed'}
                    class:not-logged-in={status === 'Not logged in'}
                    class="provider-auth-status"
                  >
                    {status}
                  </small>
                </span>
              </button>
              <Switch.Root
                class="toggle-control"
                aria-label={`${enabled ? 'Disable' : 'Enable'} ${profile.label}`}
                checked={enabled}
                disabled={!providerCanToggle(profile.id, catalogs[profile.id], providerAuthStatuses[profile.id])}
                onCheckedChange={(value) => setProviderPreference(profile.id, { ...providerPreference(profile.id), enabled: value })}
              >
                <span class="toggle-track" aria-hidden="true"><Switch.Thumb class="toggle-thumb" /></span>
              </Switch.Root>
            </div>
          {/each}
        </div>
      {/if}
    {:else if category === 'terminal'}
      <div class="settings-card">
        <div class="settings-row">
          <span class="settings-row-copy">
            <strong>Terminal text size</strong>
            <small>Change the terminal font without scaling the rest of the app.</small>
          </span>
          <div class="settings-range-control settings-terminal-size-control">
            <span class="settings-terminal-sample" style:font-size={`${preferences.terminalFontSize}px`} aria-hidden="true">$ loopcode</span>
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
          <span class="settings-row-copy">
            <strong>Terminal scrollback</strong>
            <small>Keep this many lines available above the visible terminal.</small>
          </span>
          <RadioGroup.Root
            class="settings-segmented-control"
            aria-label="Terminal scrollback"
            orientation="horizontal"
            value={String(preferences.terminalScrollback)}
            onValueChange={(value) => setPreference('terminalScrollback', Number(value))}
          >
            <RadioGroup.Item class="settings-segmented-option" value="1000">1,000</RadioGroup.Item>
            <RadioGroup.Item class="settings-segmented-option" value="5000">5,000</RadioGroup.Item>
            <RadioGroup.Item class="settings-segmented-option" value="10000">10,000</RadioGroup.Item>
          </RadioGroup.Root>
        </div>
      </div>
    {:else}
      <div class="settings-card">
        <div class="settings-row">
          <span class="settings-row-copy">
            <strong>ACP diagnostics</strong>
            <small>Export redacted provider lifecycle, RPC envelope, error, and stderr logs.</small>
          </span>
          <button class="settings-action" disabled={exportState === 'exporting'} onclick={exportLogs} aria-live="polite">
            {exportState === 'exporting' ? 'Exporting…' : exportState === 'exported' ? 'Exported' : exportState === 'error' ? 'Try again' : 'Export'}
          </button>
        </div>
        <div class="settings-row settings-row-separated">
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
