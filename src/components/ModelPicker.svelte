<script lang="ts">
  import { Combobox, Popover, Tabs } from 'bits-ui';
  import { IconCheck, IconChevronDown, IconSearch } from '@tabler/icons-svelte';

  import { profileById as officialProfileById, profiles as officialProfiles } from '../config/providers';
  import type { HarnessProfile, ModelOption, ProviderModelCatalog, ThreadState } from '../types';

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
  <Popover.Trigger
    class="flex h-7 max-w-[210px] items-center gap-1.5 rounded-md border border-transparent px-2 text-[11px] font-medium text-muted hover:border-line hover:bg-panel hover:text-ink-soft aria-expanded:border-line aria-expanded:bg-panel aria-expanded:text-ink-soft disabled:opacity-60 [&>img]:size-3.5 [&>img]:shrink-0 [&>img]:opacity-60 [&>img]:[filter:var(--provider-filter)] [&>span]:min-w-0 [&>span]:truncate [&>svg]:shrink-0"
    title="Choose provider and model"
  >
    <img class:brand-color-icon={selectedProfile.iconMode === 'brand'} src={selectedProfile.icon} alt="" />
    <span>{props.label}</span>
    <IconChevronDown size={11} stroke={1.55} />
  </Popover.Trigger>
  <Popover.Portal>
    <Popover.Content
      class="grid h-[min(300px,calc(100vh_-_150px))] min-h-[210px] w-[min(380px,calc(100vw_-_42px))] grid-cols-[58px_minmax(0,1fr)] overflow-hidden rounded-xl border border-line bg-floating shadow-overlay text-left text-ink whitespace-normal backdrop-blur-xl [&>[data-tabs-root]]:contents"
      side="top"
      align="end"
      sideOffset={10}
      collisionPadding={12}
      aria-label="Choose provider and model"
    >
      <Tabs.Root
        value={pickerProviderId}
        onValueChange={(profileId) => { pickerProviderId = profileId; modelSearch = ''; }}
        orientation="vertical"
        loop
      >
        <Tabs.List class="flex min-h-0 min-w-0 flex-col items-center gap-1 overflow-y-auto border-r border-line bg-panel px-2 py-2" aria-label="Providers">
          {#each props.profiles as profile}
            {@const state = props.thread.providers[profile.id]}
            <Tabs.Trigger
              class={`relative grid size-[42px] shrink-0 place-items-center rounded-lg border border-transparent text-muted hover:bg-panel-hover data-[state=active]:border-line data-[state=active]:bg-panel-active ${profile.id === props.thread.profileId ? '[&_.provider-icon_img]:opacity-80' : ''}`}
              value={profile.id}
              aria-label={`${profile.label}, ${providerStatusLabel(profile.id)}`}
              title={`${profile.label}, ${providerStatusLabel(profile.id)}`}
              disabled={providerSwitchLocked && profile.id !== props.thread.profileId}
            >
              <span class="grid size-[30px] place-items-center rounded-md"><img class:brand-color-icon={profile.iconMode === 'brand'} class="size-3.5 opacity-60 [filter:var(--provider-filter)]" src={profile.icon} alt="" /></span>
              <span
                class={`absolute right-1 bottom-1 size-1.5 rounded-full border border-shell ${state.connectionStatus === 'ready' ? 'bg-[color-mix(in_srgb,var(--success)_72%,transparent)]' : state.connectionStatus === 'error' || props.catalogs[profile.id].status === 'unavailable' ? 'bg-[color-mix(in_srgb,var(--danger)_72%,transparent)]' : 'bg-faint'}`}
              ></span>
            </Tabs.Trigger>
          {/each}
        </Tabs.List>
        <Tabs.Content class="flex min-h-0 min-w-0 flex-col overflow-hidden p-2" value={pickerProfile.id}>
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
            <label class="m-px mb-2 flex h-8 shrink-0 items-center gap-2 rounded-lg border border-line bg-panel px-2 text-faint focus-within:border-line-strong focus-within:bg-panel-hover focus-within:text-muted [&_input]:h-full [&_input]:min-w-0 [&_input]:flex-1 [&_input]:border-0 [&_input]:bg-transparent [&_input]:p-0 [&_input]:text-[11px] [&_input]:text-ink-soft [&_input]:outline-none [&_input::placeholder]:text-faint">
              <IconSearch size={13} stroke={1.55} />
              <Combobox.Input
                oninput={(event) => { modelSearch = event.currentTarget.value; }}
                aria-label="Search models"
                placeholder="Search models…"
              />
            </label>
            <Combobox.ContentStatic class="min-h-0 flex-1 overflow-auto pr-0.5">
              {#if pickerCatalog.status === 'loading'}
                <div class="flex min-h-[150px] items-center justify-center gap-2 p-5 text-center text-[11px] leading-[1.4] text-muted"><span class="size-3 rounded-full border-[1.5px] border-line-strong border-t-ink-soft"></span>Loading models…</div>
              {:else if pickerCatalog.status === 'unavailable'}
                <div class="flex min-h-[150px] flex-col items-center justify-center gap-2 p-5 text-center text-[11px] leading-[1.4] text-muted [&_code]:max-w-full [&_code]:rounded [&_code]:bg-recessed [&_code]:px-2 [&_code]:py-1 [&_code]:text-left [&_code]:text-[11px] [&_code]:text-ink-soft [&_button]:h-7 [&_button]:rounded-md [&_button]:border [&_button]:border-line-strong [&_button]:bg-panel-hover [&_button]:px-2.5 [&_button]:text-[11px] [&_button]:text-ink-soft [&_button:hover]:bg-panel-active">
                  <span>{pickerCatalog.error}</span>
                  {#each setupCommands() as command (command)}
                    <code>{command}</code>
                  {/each}
                  <button
                    disabled={pickerCatalog.unavailableReason === 'unsupported-platform'}
                    title={pickerCatalog.unavailableReason === 'unsupported-platform' ? 'This provider does not support the current platform' : 'Retry provider discovery'}
                    onclick={() => props.retryDiscovery(pickerProfile.id)}
                  >Retry</button>
                </div>
              {:else if pickerCatalog.models.length === 0}
                <div class="flex min-h-[150px] items-center justify-center p-5 text-center text-[11px] text-muted">No advertised models.</div>
              {:else if visibleModels.length === 0}
                <div class="flex min-h-[150px] items-center justify-center p-5 text-center text-[11px] text-muted">No matching models.</div>
              {:else}
                {#each visibleModels as model (model.id)}
                  {@const label = modelLabel(model)}
                  <Combobox.Item
                    class="grid min-h-[39px] w-full grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-2.5 rounded-lg border border-transparent px-2.5 py-1.5 text-left text-muted hover:border-line hover:bg-panel-hover hover:text-ink-soft data-[highlighted]:border-line data-[highlighted]:bg-panel-hover data-[selected]:border-line data-[selected]:bg-panel-active data-[disabled]:text-faint"
                    value={model.id}
                    label={`${model.name} ${model.id}`}
                    disabled={pickerProvider.turnStatus === 'running' || pickerProvider.turnStatus === 'blocked'}
                  >
                    {#snippet children({ selected })}
                      <span class="min-w-0"><strong class="block truncate text-[11px] font-semibold">{label.name}</strong></span>
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
