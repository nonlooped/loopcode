<script lang="ts">
  import { tick } from 'svelte';
  import { IconArrowLeft, IconCheck, IconPlus, IconTrash } from '@tabler/icons-svelte';
  import { AlertDialog, RadioGroup, Slider, Switch } from 'bits-ui';

  import MotionEnter from './motion/MotionEnter.svelte';

  import { version as appVersion } from '../../package.json';
  import appIcon from '../../assets/loopcode-mark.png';
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

  const settingsPage = 'min-h-0 flex-1 overflow-y-auto [scrollbar-gutter:stable]';
  const settingsColumn = 'mx-auto w-full max-w-3xl px-6 pt-8 pb-24 max-[680px]:px-3 max-[680px]:pt-6 max-[680px]:pb-16';
  const settingsCard = 'mt-6 overflow-hidden rounded-xl border border-line bg-raised';
  const settingsCardPlain = 'overflow-hidden rounded-xl border border-line bg-raised';
  const settingsSectionHeading = 'mt-6 mb-2 px-1 text-[11px] font-semibold tracking-wide text-faint uppercase';
  const settingsRow =
    'flex min-h-[58px] items-center gap-3.5 px-5 py-3.5 hover:bg-panel-hover max-[680px]:grid max-[680px]:grid-cols-1 max-[680px]:p-3.5';
  const settingsRowSeparated = 'border-t border-line';
  const settingsRowCopy = 'grid min-w-0 flex-1 gap-1';
  const settingsRowStrong = 'text-sm font-medium leading-tight text-text';
  const settingsRowSmall = 'text-xs leading-snug text-muted';
  const settingsVisualSection = 'grid gap-3 px-5 py-4 pb-[18px] max-[680px]:p-3.5';
  const settingsSegmentedControl =
    'flex shrink-0 gap-0.5 rounded-lg border border-line bg-recessed p-0.5 max-[680px]:col-start-1 max-[680px]:w-full max-[680px]:justify-self-start';
  const settingsSegmentedOption =
    'min-h-7 cursor-pointer rounded-md border-0 bg-transparent px-[9px] py-[5px] text-[11px] font-medium text-muted hover:bg-panel-hover hover:text-text data-[state=checked]:bg-panel-hover data-[state=checked]:text-text focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-focus-ring max-[680px]:flex-1';
  const settingsChoiceCard =
    'settings-choice-card relative grid min-w-0 gap-[7px] rounded-[10px] border border-line bg-transparent p-[7px] text-left text-muted hover:border-line-strong hover:bg-panel-hover hover:text-text data-[state=checked]:border-accent data-[state=checked]:bg-panel-hover data-[state=checked]:text-text focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus-ring';
  const settingsChoiceCheck =
    'settings-choice-check absolute top-1 right-1 grid size-[17px] scale-80 place-items-center rounded-full bg-accent text-accent-contrast opacity-0';
  const settingsChoiceLabel =
    'overflow-hidden px-0.5 text-[11px] font-medium leading-4 text-ellipsis whitespace-nowrap';
  const settingsPageHeader =
    'settings-page-header [&_h1]:m-0 [&_h1]:text-xl [&_h1]:font-[650] [&_h1]:tracking-tight [&_h1]:text-text [&_p]:mt-1 [&_p]:max-w-lg [&_p]:text-xs [&_p]:leading-snug [&_p]:text-muted';
  const settingsModePreview =
    'settings-mode-preview grid h-[52px] grid-cols-[28px_1fr] gap-1 overflow-hidden rounded-md border border-line bg-recessed p-1';
  const settingsModeSidebar = 'settings-mode-sidebar rounded-sm bg-panel-active';
  const settingsModeContent = 'settings-mode-content grid gap-0.5 p-0.5';
  const settingsThemePreview = 'settings-theme-preview grid h-[52px] grid-cols-2 gap-1';
  const settingsThemePreviewLight = 'settings-theme-preview-light relative overflow-hidden rounded-sm border border-line';
  const settingsThemePreviewDark = 'settings-theme-preview-dark relative overflow-hidden rounded-sm border border-line';
  const settingsModeGrid = 'grid grid-cols-3 gap-2';
  const settingsThemeGrid = 'grid grid-cols-[repeat(auto-fit,minmax(112px,1fr))] gap-2';
  const settingsCombinedControl =
    'flex max-w-[380px] shrink-0 flex-wrap items-center justify-end gap-1.5 max-[680px]:col-start-1 max-[680px]:w-full max-[680px]:justify-start';
  const settingsSelect =
    'h-[30px] min-w-24 max-w-[180px] rounded-[7px] border border-line bg-panel px-[11px] text-[11px] font-medium text-text-soft hover:border-line-strong hover:bg-panel-hover hover:text-text disabled:opacity-[0.62] max-[680px]:col-start-1 max-[680px]:w-full';
  const settingsModelSelect = 'w-[180px] max-[680px]:w-full';
  const settingsAction =
    'inline-flex h-[30px] min-w-[76px] items-center justify-center gap-[5px] rounded-[7px] border border-line bg-panel px-[11px] text-[11px] font-medium text-text-soft hover:border-line-strong hover:bg-panel-hover hover:text-text disabled:opacity-[0.55] max-[680px]:col-start-1';
  const settingsTextInput =
    'h-[30px] w-[180px] min-w-0 rounded-[7px] border border-line bg-panel px-[9px] text-[11px] text-text-soft hover:border-line-strong max-[680px]:col-start-1 max-[680px]:w-full';
  const settingsPathInput = 'font-mono';
  const settingsRangeControl =
    'grid w-[164px] shrink-0 gap-1 max-[680px]:col-start-1 max-[680px]:w-full [&_output]:text-right [&_output]:text-[11px] [&_output]:text-muted [&_output]:tabular-nums';
  const settingsSlider = 'relative flex h-5 w-full touch-none items-center select-none';
  const settingsSliderTrack = 'relative block h-1 w-full overflow-hidden rounded-full border border-line bg-panel-active';
  const settingsSliderRange = 'absolute h-full bg-accent';
  const settingsSliderThumb =
    'block size-3.5 cursor-grab rounded-full border border-line-strong bg-floating shadow-[0_1px_3px_rgba(0,0,0,0.35)] data-[active]:cursor-grabbing focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus-ring';
  const settingsPreviewControl =
    'flex shrink-0 items-center gap-2.5 max-[680px]:col-start-1 max-[680px]:w-full max-[680px]:justify-between';
  const settingsCodePreview =
    'h-[42px] w-[142px] overflow-hidden rounded-md border border-line bg-recessed px-2 py-[7px] text-muted [&_code]:font-mono [&_code]:text-[11px] [&_code]:leading-snug [&_code]:whitespace-nowrap [&.wrap_code]:[&_code]:whitespace-normal';
  const toggleControl = 'relative block h-5 w-9 shrink-0 border-0 bg-transparent p-0 focus-visible:outline-0 disabled:cursor-default disabled:opacity-[0.45] max-[680px]:col-start-1';
  const toggleTrack =
    'absolute inset-0 rounded-full border border-line-strong bg-panel-active p-0.5 [[data-state=checked]_&]:border-line-strong [[data-state=checked]_&]:bg-accent';
  const toggleThumb =
    'block size-3.5 rounded-full bg-muted shadow-[0_1px_3px_rgba(0,0,0,0.35)] [[data-state=checked]_&]:translate-x-4 [[data-state=checked]_&]:bg-accent-contrast';
  const providerDetailBack =
    'mb-3 -ml-[7px] inline-flex min-h-7 items-center gap-[5px] rounded-md border-0 bg-transparent px-[7px] py-[5px] text-[11px] text-muted hover:bg-panel-hover hover:text-text';
  const providerSettingsRow = 'py-0! pr-5! pl-0! max-[680px]:flex max-[680px]:pr-3.5!';
  const providerSettingsOpen =
    'flex min-h-[65px] min-w-0 flex-1 items-center gap-3.5 border-0 bg-transparent py-3.5 pl-5 text-left hover:[&_.provider-settings-icon]:border-line-strong hover:[&_.provider-settings-icon]:bg-panel-hover max-[680px]:pl-3.5';
  const providerSettingsIcon =
    'provider-settings-icon grid size-9 shrink-0 place-items-center rounded-[10px] border border-line bg-panel text-muted';
  const providerVersionClass = 'ml-1.5 inline-block text-[11px] font-medium text-faint';
  const providerAuthStatus = 'text-xs leading-snug text-muted';
  const providerModelEditor = 'px-5 py-4 pb-5 max-[680px]:px-3.5 max-[680px]:py-4';
  const providerModelHeading = 'flex items-center justify-between gap-3.5 max-[680px]:items-start';
  const providerModelEmpty = 'mt-4 text-xs leading-snug text-muted';
  const providerModelList = 'mt-3.5 grid gap-2';
  const providerModelRow = 'grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)_30px] items-end gap-2 max-[680px]:grid-cols-[minmax(0,1fr)_30px]';
  const providerModelRowLabel = 'grid min-w-0 gap-1 text-[11px] text-faint max-[680px]:col-start-1';
  const providerModelRemove =
    'grid size-[30px] place-items-center rounded-[7px] border border-transparent bg-transparent text-muted hover:border-line hover:bg-panel-hover hover:text-danger max-[680px]:col-start-2 max-[680px]:row-span-2 max-[680px]:self-center';
  const settingsAboutRow = 'flex items-center gap-3.5 p-5';
  const settingsAboutLogo = 'size-10 shrink-0 [filter:var(--provider-filter)]';
  const settingsProviderIcon = 'size-[18px] opacity-[0.72] [filter:var(--provider-filter)]';
  const settingsShortcutControl = settingsSegmentedControl;
  const settingsShortcutOption = 'flex items-center gap-[3px] [&_kbd]:rounded [&_kbd]:border [&_kbd]:border-line-strong [&_kbd]:bg-panel [&_kbd]:px-1 [&_kbd]:py-px [&_kbd]:font-mono [&_kbd]:text-[11px] [&_kbd]:font-medium [&_kbd]:leading-[14px]';
  const settingsTerminalSizeControl = settingsRangeControl;
  const settingsTerminalSample =
    'flex min-h-[30px] items-center overflow-hidden rounded-md border border-line bg-recessed px-2 py-[5px] font-mono leading-none whitespace-nowrap text-text-soft';
  const confirmationOverlay = 'fixed inset-0 z-[90] bg-overlay backdrop-blur-[3px]';
  const confirmationModal =
    'fixed top-1/2 left-1/2 z-[91] w-[min(400px,calc(100vw-40px))] -translate-x-1/2 -translate-y-1/2 overflow-hidden rounded-overlay border border-line-strong bg-decision p-[18px] shadow-overlay outline-0 backdrop-blur-[20px] backdrop-saturate-[115%] [&_[role=heading]]:text-[15px] [&_[role=heading]]:leading-snug [&_[role=heading]]:font-semibold [&_[role=heading]]:tracking-tight [&_[role=heading]]:text-text [&_[data-alert-dialog-description]]:mt-[5px] [&_[data-alert-dialog-description]]:text-xs [&_[data-alert-dialog-description]]:leading-snug [&_[data-alert-dialog-description]]:text-muted max-[680px]:w-[min(400px,calc(100vw-24px))] max-[680px]:p-4';
  const confirmationActions =
    'mt-[18px] flex flex-wrap justify-end gap-[7px] [&_button]:rounded-[7px] [&_button]:border [&_button]:border-line [&_button]:bg-transparent [&_button]:px-3 [&_button]:py-[7px] [&_button]:text-text-soft hover:[&_button]:border-line-strong hover:[&_button]:bg-panel-hover hover:[&_button]:text-text max-[680px]:[&_button]:flex-1 max-[680px]:[&_button]:text-center';

  let exportState = $state<'idle' | 'exporting' | 'exported' | 'error'>('idle');
  let resetPending = $state(false);
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
    appearance: ['Appearance', 'Choose the color mode and theme, then adjust interface layout.'],
    conversation: ['Conversation', 'Control how transcript content is displayed.'],
    composer: ['Composer', 'Choose how prompt editing and sending behave.'],
    agents: ['Agents and permissions', 'Control agent access and thread title generation.'],
    providers: ['Providers', 'Manage provider availability, defaults, connections, and models.'],
    terminal: ['Terminal', 'Tune the terminal drawer and its retained output.'],
    about: ['About', 'Startup behavior, app details, diagnostics, and interface defaults.'],
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

<MotionEnter y={reducedMotion ? 0 : 4} duration={reducedMotion ? 0 : 180} class={settingsPage}>
  <section aria-labelledby="settings-title" class="contents">
  <div class={settingsColumn}>
    <header class="mb-0">
      {#if category === 'providers' && selectedProvider}
        <button class={providerDetailBack} onclick={hideProvider}>
          <IconArrowLeft size={13} stroke={1.55} /> All providers
        </button>
      {/if}
      <h1 id="settings-title" class="m-0 text-xl font-[650] tracking-tight text-text" tabindex="-1">{pageCopy[0]}</h1>
      <p class="mt-1 max-w-lg text-[13px] leading-5 text-muted">{pageCopy[1]}</p>
    </header>

    {#if category === 'appearance'}
      <h2 class={settingsSectionHeading}>Theme</h2>
      <div class={settingsCardPlain}>
        <div class={settingsVisualSection}>
          <span class={settingsRowCopy}>
            <strong class={settingsRowStrong}>Color mode</strong>
            <small class={settingsRowSmall}>Follow the system appearance or keep LoopCode light or dark.</small>
          </span>
          <RadioGroup.Root
            class={settingsModeGrid}
            aria-label="Color mode"
            orientation="horizontal"
            value={preferences.colorMode}
            onValueChange={(value) => setPreference('colorMode', value === 'light' || value === 'dark' ? value : 'system')}
          >
            {#each ['system', 'light', 'dark'] as mode (mode)}
              <RadioGroup.Item class={`${settingsChoiceCard} settings-mode-${mode}`} value={mode}>
                <span class={settingsModePreview} aria-hidden="true">
                  <span class={settingsModeSidebar}></span>
                  <span class={settingsModeContent}>
                    <i></i><i></i><i></i>
                    <b></b>
                  </span>
                </span>
                <span class={settingsChoiceLabel}>{mode === 'system' ? 'System' : mode === 'light' ? 'Light' : 'Dark'}</span>
                <span class={settingsChoiceCheck} aria-hidden="true"><IconCheck size={11} stroke={2} /></span>
              </RadioGroup.Item>
            {/each}
          </RadioGroup.Root>
        </div>
        <div class="{settingsVisualSection} {settingsRowSeparated}">
          <span class={settingsRowCopy}>
            <strong class={settingsRowStrong}>Theme</strong>
            <small class={settingsRowSmall}>Preview each palette in light and dark mode.</small>
          </span>
          <RadioGroup.Root
            class={settingsThemeGrid}
            aria-label="Theme"
            value={preferences.theme}
            onValueChange={setTheme}
          >
            {#each THEME_OPTIONS as theme (theme.id)}
              <RadioGroup.Item class={`${settingsChoiceCard} p-2 theme-preview-${theme.id}`} value={theme.id}>
                <span class={settingsThemePreview} aria-hidden="true">
                  <span class={settingsThemePreviewLight}></span>
                  <span class={settingsThemePreviewDark}></span>
                </span>
                <span class={settingsChoiceLabel}>{theme.label}</span>
                <span class={settingsChoiceCheck} aria-hidden="true"><IconCheck size={11} stroke={2} /></span>
              </RadioGroup.Item>
            {/each}
          </RadioGroup.Root>
        </div>
      </div>
      <h2 class={settingsSectionHeading}>Density</h2>
      <div class={settingsCardPlain}>
        <div class={settingsRow}>
          <span class={settingsRowCopy}>
            <strong class={settingsRowStrong}>Sidebar thread spacing</strong>
            <small class={settingsRowSmall}>How much detail each thread shows in the sidebar.</small>
          </span>
          <RadioGroup.Root
            class={settingsSegmentedControl}
            aria-label="Sidebar thread spacing"
            orientation="horizontal"
            value={preferences.compactSessionRows ? 'compact' : 'comfortable'}
            onValueChange={(value) => setPreference('compactSessionRows', value === 'compact')}
          >
            <RadioGroup.Item class={settingsSegmentedOption} value="comfortable">Comfortable</RadioGroup.Item>
            <RadioGroup.Item class={settingsSegmentedOption} value="compact">Compact</RadioGroup.Item>
          </RadioGroup.Root>
        </div>
        <div class="{settingsRow} {settingsRowSeparated}">
          <span class={settingsRowCopy}>
            <strong class={settingsRowStrong}>Transcript spacing</strong>
            <small class={settingsRowSmall}>The space between transcript entries.</small>
          </span>
          <RadioGroup.Root
            class={settingsSegmentedControl}
            aria-label="Transcript spacing"
            orientation="horizontal"
            value={preferences.transcriptDensity}
            onValueChange={(value) => setPreference('transcriptDensity', value === 'compact' ? 'compact' : 'comfortable')}
          >
            <RadioGroup.Item class={settingsSegmentedOption} value="comfortable">Comfortable</RadioGroup.Item>
            <RadioGroup.Item class={settingsSegmentedOption} value="compact">Compact</RadioGroup.Item>
          </RadioGroup.Root>
        </div>
        <div class="{settingsRow} {settingsRowSeparated}">
          <span class={settingsRowCopy}>
            <strong class={settingsRowStrong}>Content width</strong>
            <small class={settingsRowSmall}>Set the shared maximum width for messages, questions, and the composer.</small>
          </span>
          <div class={settingsRangeControl}>
            <output>{preferences.contentWidth}px</output>
            <Slider.Root
              class={settingsSlider}
              aria-label="Content width"
              type="single"
              min={CONTENT_WIDTH_RANGE.min}
              max={CONTENT_WIDTH_RANGE.max}
              step={20}
              value={preferences.contentWidth}
              onValueChange={(value) => setPreference('contentWidth', value)}
            >
              {#snippet children({ thumbItems })}
                <span class={settingsSliderTrack}><Slider.Range class={settingsSliderRange} /></span>
                {#each thumbItems as { index } (index)}
                  <Slider.Thumb class={settingsSliderThumb} {index} />
                {/each}
              {/snippet}
            </Slider.Root>
          </div>
        </div>
      </div>
      <h2 class={settingsSectionHeading}>Motion &amp; scale</h2>
      <div class={settingsCardPlain}>
        <div class={settingsRow}>
          <span class={settingsRowCopy}>
            <strong class={settingsRowStrong}>Interface motion</strong>
            <small class={settingsRowSmall}>Follow the system preference or remove interface animation.</small>
          </span>
          <RadioGroup.Root
            class={settingsSegmentedControl}
            aria-label="Interface motion"
            orientation="horizontal"
            value={preferences.motionMode}
            onValueChange={(value) => setPreference('motionMode', value === 'reduced' ? 'reduced' : 'system')}
          >
            <RadioGroup.Item class={settingsSegmentedOption} value="system">System</RadioGroup.Item>
            <RadioGroup.Item class={settingsSegmentedOption} value="reduced">Reduced</RadioGroup.Item>
          </RadioGroup.Root>
        </div>
        <div class="{settingsRow} {settingsRowSeparated}">
          <span class={settingsRowCopy}>
            <strong class={settingsRowStrong}>Interface zoom</strong>
            <small class={settingsRowSmall}>Scale the entire app without changing the window size.</small>
          </span>
          <div class={settingsRangeControl}>
            <output>{preferences.interfaceZoom}%</output>
            <Slider.Root
              class={settingsSlider}
              aria-label="Interface zoom"
              type="single"
              min={INTERFACE_ZOOM_RANGE.min}
              max={INTERFACE_ZOOM_RANGE.max}
              step={10}
              value={preferences.interfaceZoom}
              onValueChange={(value) => setPreference('interfaceZoom', value)}
            >
              {#snippet children({ thumbItems })}
                <span class={settingsSliderTrack}><Slider.Range class={settingsSliderRange} /></span>
                {#each thumbItems as { index } (index)}
                  <Slider.Thumb class={settingsSliderThumb} {index} />
                {/each}
              {/snippet}
            </Slider.Root>
          </div>
        </div>
      </div>
    {:else if category === 'conversation'}
      <div class={settingsCard}>
        <div class={settingsRow}>
          <span class={settingsRowCopy}>
            <strong class={settingsRowStrong}>Wrap message code</strong>
            <small class={settingsRowSmall}>Wrap long lines in message code blocks instead of scrolling horizontally.</small>
          </span>
          <div class={settingsPreviewControl}>
            <span class:wrap={preferences.wrapCode} class="{settingsCodePreview} {preferences.wrapCode ? 'wrap' : ''}" aria-hidden="true"><code>const model = provider.create(options);</code></span>
            <Switch.Root
              class={toggleControl}
              aria-label="Wrap message code"
              checked={preferences.wrapCode}
              onCheckedChange={(value) => setPreference('wrapCode', value)}
            >
              <span class={toggleTrack} aria-hidden="true"><Switch.Thumb class={toggleThumb} /></span>
            </Switch.Root>
          </div>
        </div>
        <div class="{settingsRow} {settingsRowSeparated}">
          <span class={settingsRowCopy}>
            <strong class={settingsRowStrong}>Message timestamps</strong>
            <small class={settingsRowSmall}>Show the sender and local time above each user and agent message.</small>
          </span>
          <Switch.Root
            class={toggleControl}
            aria-label="Message timestamps"
            checked={preferences.showMessageTimestamps}
            onCheckedChange={(value) => setPreference('showMessageTimestamps', value)}
          >
            <span class={toggleTrack} aria-hidden="true"><Switch.Thumb class={toggleThumb} /></span>
          </Switch.Root>
        </div>
      </div>
    {:else if category === 'composer'}
      <div class={settingsCard}>
        <div class={settingsRow}>
          <span class={settingsRowCopy}>
            <strong class={settingsRowStrong}>Spellcheck</strong>
            <small class={settingsRowSmall}>Use the operating system spellchecker while writing prompts.</small>
          </span>
          <Switch.Root
            class={toggleControl}
            aria-label="Composer spellcheck"
            checked={preferences.composerSpellcheck}
            onCheckedChange={(value) => setPreference('composerSpellcheck', value)}
          >
            <span class={toggleTrack} aria-hidden="true"><Switch.Thumb class={toggleThumb} /></span>
          </Switch.Root>
        </div>
        <div class="{settingsRow} {settingsRowSeparated}">
          <span class={settingsRowCopy}>
            <strong class={settingsRowStrong}>Send shortcut</strong>
            <small class={settingsRowSmall}>Choose whether Enter sends or inserts a new line in the composer.</small>
          </span>
          <RadioGroup.Root
            class={settingsShortcutControl}
            aria-label="Send shortcut"
            orientation="horizontal"
            value={preferences.sendShortcut}
            onValueChange={(value) => setPreference('sendShortcut', value === 'modifier-enter' ? 'modifier-enter' : 'enter')}
          >
            <RadioGroup.Item class="{settingsSegmentedOption} {settingsShortcutOption}" value="enter"><kbd>Enter</kbd></RadioGroup.Item>
            <RadioGroup.Item class="{settingsSegmentedOption} {settingsShortcutOption}" value="modifier-enter"><kbd>Ctrl/⌘</kbd><span>+</span><kbd>Enter</kbd></RadioGroup.Item>
          </RadioGroup.Root>
        </div>
      </div>
    {:else if category === 'agents'}
      <div class={settingsCard}>
        <div class={settingsRow}>
          <span class={settingsRowCopy}>
            <strong class={settingsRowStrong}>Agent permissions</strong>
            <small class={settingsRowSmall}>Restricted asks before commands. Full access automatically approves permission requests.</small>
          </span>
          <RadioGroup.Root
            class={settingsSegmentedControl}
            aria-label="Agent permissions"
            orientation="horizontal"
            value={permissionMode}
            onValueChange={(value) => setPermissionMode(value === 'full' ? 'full' : 'restricted')}
          >
            <RadioGroup.Item class={settingsSegmentedOption} value="restricted">Restricted</RadioGroup.Item>
            <RadioGroup.Item class={settingsSegmentedOption} value="full">Full access</RadioGroup.Item>
          </RadioGroup.Root>
        </div>
        <div class="{settingsRow} {settingsRowSeparated}">
          <span class={settingsRowCopy}>
            <strong class={settingsRowStrong}>Thread titles</strong>
            <small class={settingsRowSmall}>{preferences.automaticTitleGeneration ? 'Use a separate provider connection to name new threads.' : 'Use the first few words of the prompt.'}</small>
          </span>
          <div class={settingsCombinedControl}>
            <select
              class={settingsSelect}
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
                class="{settingsSelect} {settingsModelSelect}"
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
        <div class={settingsCard}>
          <div class={settingsRow}>
            <span class={settingsRowCopy}>
              <strong class={settingsRowStrong}>Default model</strong>
              <small class={settingsRowSmall}>{catalog?.status === 'ready' ? `Use this model when a new ${selectedProvider.label} thread starts.` : catalog?.status === 'unavailable' ? catalog.error : `Loading ${selectedProvider.label} models…`}</small>
            </span>
            <select
              class="{settingsSelect} {settingsModelSelect}"
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
          <label class="{settingsRow} {settingsRowSeparated}">
              <span class={settingsRowCopy}>
              <strong class={settingsRowStrong}>Provider command path</strong>
              <small class={settingsRowSmall}>Override the provider executable while keeping its required ACP arguments.</small>
            </span>
            <input
              class="{settingsTextInput} {settingsPathInput}"
              aria-label="Provider command path"
              spellcheck="false"
              value={setting.command ?? selectedBaseProvider.command}
              onchange={(event) => updateSelectedProvider({ command: inputValue(event) })}
            />
          </label>
          <div class="{providerModelEditor} {settingsRowSeparated}">
            <div class={providerModelHeading}>
              <span class="grid gap-1">
                <strong class={settingsRowStrong}>Custom models</strong>
                <small class={settingsRowSmall}>Add models that are not advertised by the provider.</small>
              </span>
              <button class={settingsAction} onclick={addCustomModel}><IconPlus size={13} stroke={1.55} /> Add model</button>
            </div>
            {#if (setting.models ?? []).length === 0}
              <p class={providerModelEmpty}>No custom models.</p>
            {:else}
              <div class={providerModelList}>
                {#each setting.models ?? [] as model, index (`${index}-${model.id}`)}
                  <div class={providerModelRow}>
                    <label class={providerModelRowLabel}>
                      <span>Name</span>
                      <input
                        class={settingsTextInput}
                        aria-label={`Custom model ${index + 1} name`}
                        value={model.name}
                        onchange={(event) => updateCustomModel(index, 'name', inputValue(event))}
                      />
                    </label>
                    <label class={providerModelRowLabel}>
                      <span>Model ID</span>
                      <input
                        class={settingsTextInput}
                        aria-label={`Custom model ${index + 1} ID`}
                        spellcheck="false"
                        value={model.id}
                        onchange={(event) => updateCustomModel(index, 'id', inputValue(event))}
                      />
                    </label>
                    <button
                      class={providerModelRemove}
                      aria-label={`Remove ${model.name}`}
                      title={`Remove ${model.name}`}
                      onclick={() => removeCustomModel(index)}
                    ><IconTrash size={14} stroke={1.55} /></button>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
          {#if Object.keys(setting).length > 0 || preferences.providerModelDefaults[selectedProvider.id]}
            <div class="{settingsRow} {settingsRowSeparated}">
              <span class={settingsRowCopy}>
                <strong class={settingsRowStrong}>Restore provider defaults</strong>
                <small class={settingsRowSmall}>Reset the command path, custom models, enabled state, and default model.</small>
              </span>
              <button class={settingsAction} onclick={resetSelectedProvider}>Reset</button>
            </div>
          {/if}
        </div>
      {:else}
        <div class={settingsCard}>
          <div class={settingsRow}>
            <span class={settingsRowCopy}>
              <strong class={settingsRowStrong}>Default provider</strong>
              <small class={settingsRowSmall}>Use this provider when LoopCode creates a new thread.</small>
            </span>
            <select
              class={settingsSelect}
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
            {@const canToggle = providerCanToggle(profile.id, catalogs[profile.id], providerAuthStatuses[profile.id])}
            <div class="{settingsRow} {settingsRowSeparated} {providerSettingsRow}">
              <button id={`provider-setting-${profile.id}`} class={providerSettingsOpen} onclick={() => showProvider(profile.id)}>
                <span class={providerSettingsIcon} aria-hidden="true"><img class:brand-color-icon={profile.iconMode === 'brand'} class={settingsProviderIcon} src={profile.icon} alt="" /></span>
                <span class={settingsRowCopy}>
                  <strong class={settingsRowStrong}>{profile.label}{#if version} <span class={providerVersionClass}>{version}</span>{/if}</strong>
                  <small
                    class="{providerAuthStatus} {status === 'Authenticated' || status === 'Connected' ? 'text-success' : status === 'Disabled' ? 'text-faint' : status === 'Not logged in' ? 'text-warning' : status === 'Not installed' ? 'text-danger' : ''}"
                  >
                    {status}
                  </small>
                </span>
              </button>
              <Switch.Root
                class={toggleControl}
                aria-label={`${enabled ? 'Disable' : 'Enable'} ${profile.label}`}
                checked={enabled && canToggle}
                disabled={!canToggle}
                onCheckedChange={(value) => setProviderPreference(profile.id, { ...providerPreference(profile.id), enabled: value })}
              >
                <span class={toggleTrack} aria-hidden="true"><Switch.Thumb class={toggleThumb} /></span>
              </Switch.Root>
            </div>
          {/each}
        </div>
      {/if}
    {:else if category === 'terminal'}
      <div class={settingsCard}>
        <div class={settingsRow}>
          <span class={settingsRowCopy}>
            <strong class={settingsRowStrong}>Terminal text size</strong>
            <small class={settingsRowSmall}>Change the terminal font without scaling the rest of the app.</small>
          </span>
          <div class="{settingsRangeControl} {settingsTerminalSizeControl}">
            <span class={settingsTerminalSample} style:font-size={`${preferences.terminalFontSize}px`} aria-hidden="true">$ loopcode</span>
            <output>{preferences.terminalFontSize}px</output>
            <Slider.Root
              class={settingsSlider}
              aria-label="Terminal text size"
              type="single"
              min={TERMINAL_FONT_SIZE_RANGE.min}
              max={TERMINAL_FONT_SIZE_RANGE.max}
              step={1}
              value={preferences.terminalFontSize}
              onValueChange={(value) => setPreference('terminalFontSize', value)}
            >
              {#snippet children({ thumbItems })}
                <span class={settingsSliderTrack}><Slider.Range class={settingsSliderRange} /></span>
                {#each thumbItems as { index } (index)}
                  <Slider.Thumb class={settingsSliderThumb} {index} />
                {/each}
              {/snippet}
            </Slider.Root>
          </div>
        </div>
        <div class="{settingsRow} {settingsRowSeparated}">
          <span class={settingsRowCopy}>
            <strong class={settingsRowStrong}>Terminal scrollback</strong>
            <small class={settingsRowSmall}>Keep this many lines available above the visible terminal.</small>
          </span>
          <RadioGroup.Root
            class={settingsSegmentedControl}
            aria-label="Terminal scrollback"
            orientation="horizontal"
            value={String(preferences.terminalScrollback)}
            onValueChange={(value) => setPreference('terminalScrollback', Number(value))}
          >
            <RadioGroup.Item class={settingsSegmentedOption} value="1000">1,000</RadioGroup.Item>
            <RadioGroup.Item class={settingsSegmentedOption} value="5000">5,000</RadioGroup.Item>
            <RadioGroup.Item class={settingsSegmentedOption} value="10000">10,000</RadioGroup.Item>
          </RadioGroup.Root>
        </div>
      </div>
    {:else if category === 'about'}
      <div class={settingsCard}>
        <div class={settingsAboutRow}>
          <img class={settingsAboutLogo} src={appIcon} alt="" aria-hidden="true" />
          <span class={settingsRowCopy}>
            <strong class={settingsRowStrong}>LoopCode</strong>
            <small class={settingsRowSmall}>Version {appVersion}</small>
          </span>
        </div>
        <div class="{settingsRow} {settingsRowSeparated}">
          <span class={settingsRowCopy}>
            <strong class={settingsRowStrong}>On startup</strong>
            <small class={settingsRowSmall}>Return to the last thread or open an empty thread when LoopCode starts.</small>
          </span>
          <RadioGroup.Root
            class={settingsSegmentedControl}
            aria-label="On startup"
            orientation="horizontal"
            value={preferences.startupBehavior}
            onValueChange={(value) => setPreference('startupBehavior', value === 'new-thread' ? 'new-thread' : 'last-thread')}
          >
            <RadioGroup.Item class={settingsSegmentedOption} value="last-thread">Last thread</RadioGroup.Item>
            <RadioGroup.Item class={settingsSegmentedOption} value="new-thread">Empty thread</RadioGroup.Item>
          </RadioGroup.Root>
        </div>
        <div class="{settingsRow} {settingsRowSeparated}">
          <span class={settingsRowCopy}>
            <strong class={settingsRowStrong}>New threads</strong>
            <small title={defaultWorkingFolder}>Use the selected project or {defaultWorkingFolder || 'the system folder'}.</small>
          </span>
          <div class={settingsCombinedControl}>
            <RadioGroup.Root
              class={settingsSegmentedControl}
              aria-label="New thread location"
              orientation="horizontal"
              value={preferences.newThreadProject}
              onValueChange={(value) => setPreference('newThreadProject', value === 'default-folder' ? 'default-folder' : 'selected')}
            >
              <RadioGroup.Item class={settingsSegmentedOption} value="selected">Selected project</RadioGroup.Item>
              <RadioGroup.Item class={settingsSegmentedOption} value="default-folder">Default folder</RadioGroup.Item>
            </RadioGroup.Root>
            {#if preferences.newThreadProject === 'default-folder'}
              <button class={settingsAction} onclick={chooseDefaultWorkingFolder}>Choose folder…</button>
            {/if}
          </div>
        </div>
      </div>
      <div class={settingsCard}>
        <div class={settingsRow}>
          <span class={settingsRowCopy}>
            <strong class={settingsRowStrong}>ACP diagnostics</strong>
            <small class={settingsRowSmall}>Export redacted provider lifecycle, RPC envelope, error, and stderr logs.</small>
          </span>
          <button class={settingsAction} disabled={exportState === 'exporting'} onclick={exportLogs} aria-live="polite">
            {exportState === 'exporting' ? 'Exporting…' : exportState === 'exported' ? 'Exported' : exportState === 'error' ? 'Try again' : 'Export'}
          </button>
        </div>
        <div class="{settingsRow} {settingsRowSeparated}">
          <span class={settingsRowCopy}>
            <strong class={settingsRowStrong}>Reset interface settings</strong>
            <small class={settingsRowSmall}>Restore the theme, layout, transcript display, panel sizes, terminal height, and zoom.</small>
          </span>
          <button class={settingsAction} onclick={() => { resetPending = true; }}>Reset</button>
        </div>
      </div>
    {/if}
  </div>
  </section>
</MotionEnter>

<AlertDialog.Root open={resetPending} onOpenChange={(open) => { resetPending = open; }}>
  <AlertDialog.Portal>
    <AlertDialog.Overlay class={confirmationOverlay} />
    <AlertDialog.Content class="{confirmationModal} max-w-[400px]">
      <AlertDialog.Title>Reset interface settings?</AlertDialog.Title>
      <AlertDialog.Description>
        This restores the theme, layout, transcript display, panel sizes, terminal height, and zoom. Provider, title, and permission settings will not change.
      </AlertDialog.Description>
      <footer class={confirmationActions}>
        <AlertDialog.Cancel>Cancel</AlertDialog.Cancel>
        <button onclick={() => {
          resetPending = false;
          resetSettings();
        }}>Reset</button>
      </footer>
    </AlertDialog.Content>
  </AlertDialog.Portal>
</AlertDialog.Root>
