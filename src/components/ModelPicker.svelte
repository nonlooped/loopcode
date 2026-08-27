<script lang="ts">
  import { Combobox, Popover, Tabs } from 'bits-ui';
  import { IconCheck, IconChevronDown, IconSearch } from '@tabler/icons-svelte';

  import { profileById as officialProfileById, profiles as officialProfiles } from '../config/providers';
  import type { HarnessProfile, ModelOption, ProviderModelCatalog, ThreadState } from '../types';
  import MotionSpin from './motion/MotionSpin.svelte';

  interface Props {
    thread: ThreadState;
    catalogs: Record<string, ProviderModelCatalog>;
    profiles: HarnessProfile[];
    label: string;
    open: boolean;
    setOpen: (open: boolean) => void;
    choose: (profileId: string, model: ModelOption) => void;
    retryDiscovery: (profileId: string) => void;
  }

  const props: Props = $props();
  let pickerProviderId = $state('');
  let modelSearch = $state('');
  let modelListOpen = $state(false);
  const pickerProfile = $derived(
    props.profiles.find((profile) => profile.id === pickerProviderId)
      ?? props.profiles[0]
      ?? officialProfileById(props.thread.profileId)
      ?? officialProfiles[0],
  );
  const selectedProfile = $derived(
    props.profiles.find((profile) => profile.id === props.thread.profileId) ?? pickerProfile,
  );
  const pickerProvider = $derived(props.thread.providers[pickerProfile.id]);
  const pickerCatalog = $derived(props.catalogs[pickerProfile.id]);
  const providerSwitchLocked = $derived(Object.values(props.thread.providers).some(
    (provider) => provider.turnStatus === 'running' || provider.turnStatus === 'blocked',
  ));
  const visibleModels = $derived(matchingModels(pickerCatalog.models));

  const trigger =
    'flex h-7 max-w-[210px] items-center gap-1.5 rounded-[7px] border border-transparent bg-transparent px-[7px] text-[11px] font-medium text-muted hover:border-line hover:bg-panel hover:text-text-soft aria-expanded:border-line aria-expanded:bg-panel aria-expanded:text-text-soft disabled:opacity-[0.62] [&_img]:size-3.5 [&_img]:shrink-0 [&_img]:opacity-[0.62] [&_span]:min-w-0 [&_span]:truncate [&_svg]:shrink-0';
  const pickerShell =
    'grid h-[min(300px,calc(100vh-150px))] min-h-[210px] w-[min(380px,calc(100vw-42px))] grid-cols-[58px_minmax(0,1fr)] grid-rows-[minmax(0,1fr)] overflow-hidden rounded-overlay border border-line bg-floating text-left whitespace-normal text-text shadow-overlay';
  const providerTab =
    'relative grid size-[42px] shrink-0 place-items-center rounded-[9px] border border-transparent bg-transparent text-muted hover:bg-panel-hover data-[state=active]:border-line data-[state=active]:bg-panel-active [&.selected_[data-state=active]_img]:opacity-[0.78] [&[data-state=active]_img]:opacity-[0.78]';
  const searchField =
    'mx-0.5 mb-[7px] mt-px flex h-8 shrink-0 items-center gap-[7px] rounded-lg border border-line bg-panel px-[9px] text-faint focus-within:border-line-strong focus-within:bg-panel-hover focus-within:text-muted [&_input]:h-full [&_input]:min-w-0 [&_input]:flex-1 [&_input]:border-0 [&_input]:bg-transparent [&_input]:p-0 [&_input]:text-[11px] [&_input]:text-text-soft [&_input]:outline-0 [&_input::placeholder]:text-faint [&_input:disabled]:text-faint';
  const modelOption =
    'grid min-h-[39px] w-full grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-2.5 rounded-[9px] border border-transparent bg-transparent px-2.5 py-[7px] text-left text-muted hover:border-line hover:bg-panel-hover hover:text-text-soft data-[highlighted]:border-line data-[highlighted]:bg-panel-hover data-[highlighted]:text-text-soft data-[selected]:border-line data-[selected]:bg-panel-active data-[selected]:text-text-soft data-[disabled]:text-faint [&>svg]:text-muted';
  const statePanel =
    'flex min-h-[150px] items-center justify-center gap-2 p-[22px] text-center text-[11px] leading-snug text-muted';

  function matchingModels(models: ModelOption[]) {
    const query = modelSearch.trim().toLocaleLowerCase();
    if (!query) return models;
    return models.filter((model) => `${model.name} ${model.id}`.toLocaleLowerCase().includes(query));
  }

  function setOpen(open: boolean) {
    if (open) {
      pickerProviderId = props.profiles.some((profile) => profile.id === props.thread.profileId)
        ? props.thread.profileId
        : (props.profiles[0]?.id ?? '');
      modelSearch = '';
      modelListOpen = true;
    }
    props.setOpen(open);
  }

  function chooseModel(modelId: string) {
    const model = pickerCatalog.models.find((item) => item.id === modelId);
    if (model) props.choose(pickerProfile.id, model);
  }

  function setupCommands() {
    if (pickerCatalog.status !== 'unavailable') return [];
    if (pickerCatalog.unavailableReason === 'unsupported-platform') return [];
    if (pickerCatalog.unavailableReason === 'authentication') return [pickerProfile.loginCommand];
    return [pickerProfile.installCommand, pickerProfile.loginCommand];
  }

  function providerStatusLabel(profileId: string) {
    const catalog = props.catalogs[profileId];
    if (catalog.status === 'ready') return 'ready';
    if (catalog.status === 'unavailable') return 'unavailable';
    return 'loading';
  }

  function modelLabel(model: ModelOption) {
    const separator = model.name.indexOf('/');
    if (separator <= 0 || separator === model.name.length - 1) return { name: model.name };
    return {
      provider: model.name.slice(0, separator).trim(),
      name: model.name.slice(separator + 1).trim(),
    };
  }
</script>

<Popover.Root open={props.open} onOpenChange={setOpen}>
  <Popover.Trigger class={trigger} title="Choose provider and model">
    <img class:brand-color-icon={selectedProfile.iconMode === 'brand'} src={selectedProfile.icon} alt="" />
    <span>{props.label}</span>
    <IconChevronDown size={11} stroke={1.55} />
  </Popover.Trigger>
  <Popover.Portal>
    <Popover.Content
      class={pickerShell}
      side="top"
      align="end"
      sideOffset={10}
      collisionPadding={12}
      aria-label="Choose provider and model"
    >
      <Tabs.Root
        class="contents"
        value={pickerProviderId}
        onValueChange={(profileId) => { pickerProviderId = profileId; modelSearch = ''; }}
        orientation="vertical"
        loop
      >
        <Tabs.List
          class="flex min-h-0 min-w-0 flex-col items-center gap-1 overflow-y-auto border-r border-line bg-panel px-[7px] py-2"
          aria-label="Providers"
        >
          {#each props.profiles as profile}
            {@const state = props.thread.providers[profile.id]}
            <Tabs.Trigger
              class="{providerTab} {profile.id === props.thread.profileId ? 'selected' : ''} [&[data-state=active]_img]:opacity-[0.78] [&.selected_img]:opacity-[0.78]"
              value={profile.id}
              aria-label={`${profile.label}, ${providerStatusLabel(profile.id)}`}
              title={`${profile.label}, ${providerStatusLabel(profile.id)}`}
              disabled={providerSwitchLocked && profile.id !== props.thread.profileId}
            >
              <span class="grid size-[30px] place-items-center rounded-[7px] [&_img]:size-3.5 [&_img]:[filter:var(--provider-filter)]">
                <img class:brand-color-icon={profile.iconMode === 'brand'} src={profile.icon} alt="" />
              </span>
              <span
                class="absolute right-[5px] bottom-[5px] size-[5px] rounded-full border border-shell-solid {state.connectionStatus === 'ready' ? 'bg-[color-mix(in_srgb,var(--success)_72%,transparent)]' : state.connectionStatus === 'error' || props.catalogs[profile.id].status === 'unavailable' ? 'bg-[color-mix(in_srgb,var(--danger)_72%,transparent)]' : 'bg-faint'}"
              ></span>
            </Tabs.Trigger>
          {/each}
        </Tabs.List>
        <Tabs.Content class="flex min-h-0 min-w-0 flex-col overflow-hidden p-[7px]" value={pickerProfile.id}>
          <Combobox.Root
            type="single"
            value={pickerProfile.id === props.thread.profileId ? pickerProvider.selectedModelId ?? '' : ''}
            onValueChange={chooseModel}
            open={modelListOpen}
            onOpenChange={(open) => { modelListOpen = open; }}
            inputValue={modelSearch}
            disabled={
              pickerCatalog.status !== 'ready'
              || pickerCatalog.models.length === 0
              || (providerSwitchLocked && pickerProfile.id !== props.thread.profileId)
            }
            allowDeselect={false}
          >
            <label class={searchField}>
              <IconSearch size={13} stroke={1.55} />
              <Combobox.Input
                oninput={(event) => { modelSearch = event.currentTarget.value; }}
                aria-label="Search models"
                placeholder="Search models…"
              />
            </label>
            <Combobox.ContentStatic class="min-h-0 flex-1 overflow-auto pr-0.5">
              {#if pickerCatalog.status === 'loading'}
                <div class={statePanel}>
                  <MotionSpin class="inline-flex">
                    <span class="size-3 rounded-full border-[1.5px] border-line-strong border-t-text-soft"></span>
                  </MotionSpin>
                  Loading models…
                </div>
              {:else if pickerCatalog.status === 'unavailable'}
                <div class="{statePanel} flex-col">
                  <span>{pickerCatalog.error}</span>
                  {#each setupCommands() as command (command)}
                    <code class="block max-w-full rounded-[5px] bg-recessed px-[7px] py-[5px] text-left text-[11px] break-anywhere text-text-soft select-text">{command}</code>
                  {/each}
                  <button
                    class="h-7 rounded-[7px] border border-line-strong bg-panel-hover px-2.5 text-[11px] text-text-soft hover:bg-panel-active disabled:opacity-[0.55]"
                    disabled={pickerCatalog.unavailableReason === 'unsupported-platform'}
                    title={pickerCatalog.unavailableReason === 'unsupported-platform' ? 'This provider does not support the current platform' : 'Retry provider discovery'}
                    onclick={() => props.retryDiscovery(pickerProfile.id)}
                  >Retry</button>
                </div>
              {:else if pickerCatalog.models.length === 0}
                <div class={statePanel}>No advertised models.</div>
              {:else if visibleModels.length === 0}
                <div class={statePanel}>No matching models.</div>
              {:else}
                {#each visibleModels as model (model.id)}
                  {@const label = modelLabel(model)}
                  <Combobox.Item
                    class={modelOption}
                    value={model.id}
                    label={`${model.name} ${model.id}`}
                    disabled={pickerProvider.turnStatus === 'running' || pickerProvider.turnStatus === 'blocked'}
                  >
                    {#snippet children({ selected })}
                      <span class="min-w-0 [&_strong]:block [&_strong]:truncate [&_strong]:text-[11px] [&_strong]:font-semibold"><strong>{label.name}</strong></span>
                      {#if label.provider}<span class="max-w-[92px] truncate text-[11px] font-medium text-faint">{label.provider}</span>{/if}
                      {#if selected}<IconCheck size={15} stroke={2} />{/if}
                    {/snippet}
                  </Combobox.Item>
                {/each}
              {/if}
            </Combobox.ContentStatic>
          </Combobox.Root>
        </Tabs.Content>
      </Tabs.Root>
    </Popover.Content>
  </Popover.Portal>
</Popover.Root>
