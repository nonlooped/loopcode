<script lang="ts">
  import { Combobox, Popover, Tabs } from 'bits-ui';
  import { IconCheck, IconChevronDown, IconSearch } from '@tabler/icons-svelte';

  import { profileById, profiles } from '../config/providers';
  import type { ModelOption, ProviderModelCatalog, ThreadState } from '../types';

  interface Props {
    thread: ThreadState;
    catalogs: Record<string, ProviderModelCatalog>;
    label: string;
    open: boolean;
    setOpen: (open: boolean) => void;
    choose: (profileId: string, model: ModelOption) => void;
    retryDiscovery: (profileId: string) => void;
  }

  const props: Props = $props();
  let pickerProviderId = $state(profiles[0].id);
  let modelSearch = $state('');
  let modelListOpen = $state(false);
  const pickerProfile = $derived(profileById(pickerProviderId));
  const pickerProvider = $derived(props.thread.providers[pickerProfile.id]);
  const pickerCatalog = $derived(props.catalogs[pickerProfile.id]);
  const visibleModels = $derived(matchingModels(pickerCatalog.models));

  function matchingModels(models: ModelOption[]) {
    const query = modelSearch.trim().toLocaleLowerCase();
    if (!query) return models;
    return models.filter((model) => `${model.name} ${model.id}`.toLocaleLowerCase().includes(query));
  }

  function setOpen(open: boolean) {
    if (open) {
      pickerProviderId = props.thread.profileId;
      modelSearch = '';
      modelListOpen = true;
    }
    props.setOpen(open);
  }

  function chooseModel(modelId: string) {
    const model = pickerCatalog.models.find((item) => item.id === modelId);
    if (model) props.choose(pickerProfile.id, model);
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
    class="model-picker-trigger"
    title="Choose provider and model"
  >
    <img src={profileById(props.thread.profileId).icon} alt="" />
    <span>{props.label}</span>
    <IconChevronDown size={11} stroke={1.7} />
  </Popover.Trigger>
  <Popover.Portal>
    <Popover.Content
      class="model-picker"
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
        <Tabs.List class="model-providers" aria-label="Providers">
          {#each profiles as profile}
            {@const state = props.thread.providers[profile.id]}
            <Tabs.Trigger
              class={`model-provider ${profile.id === props.thread.profileId ? 'selected' : ''}`}
              value={profile.id}
              aria-label={profile.label}
              title={profile.label}
            >
              <span class="provider-icon"><img src={profile.icon} alt="" /></span>
              <span
                class:ready={state.connectionStatus === 'ready'}
                class:error={state.connectionStatus === 'error' || props.catalogs[profile.id].status === 'error'}
                class="provider-status"
              ></span>
            </Tabs.Trigger>
          {/each}
        </Tabs.List>
        <Tabs.Content class="model-options" value={pickerProfile.id}>
          <Combobox.Root
            type="single"
            value={pickerProvider.selectedModelId ?? ''}
            onValueChange={chooseModel}
            open={modelListOpen}
            onOpenChange={(open) => { modelListOpen = open; }}
            inputValue={modelSearch}
            disabled={pickerCatalog.status !== 'ready' || pickerCatalog.models.length === 0}
            allowDeselect={false}
          >
            <label class="model-search">
              <IconSearch size={13} stroke={1.7} />
              <Combobox.Input
                oninput={(event) => { modelSearch = event.currentTarget.value; }}
                aria-label="Search models"
                placeholder="Search models…"
              />
            </label>
            <Combobox.ContentStatic class="model-options-scroll">
              {#if pickerCatalog.status === 'loading'}
                <div class="model-picker-state"><span class="model-spinner"></span>Loading models…</div>
              {:else if pickerCatalog.status === 'error'}
                <div class="model-picker-state error">
                  <span>{pickerCatalog.error ?? `${pickerProfile.label} models are unavailable.`}</span>
                  <button onclick={() => props.retryDiscovery(pickerProfile.id)}>Retry</button>
                </div>
              {:else if pickerCatalog.models.length === 0}
                <div class="model-picker-state">No advertised models.</div>
              {:else if visibleModels.length === 0}
                <div class="model-picker-state">No matching models.</div>
              {:else}
                {#each visibleModels as model (model.id)}
                  {@const label = modelLabel(model)}
                  <Combobox.Item
                    class="model-option"
                    value={model.id}
                    label={`${model.name} ${model.id}`}
                    disabled={pickerProvider.turnStatus === 'running' || pickerProvider.turnStatus === 'blocked'}
                  >
                    {#snippet children({ selected })}
                      <span class="model-option-copy"><strong>{label.name}</strong></span>
                      {#if label.provider}<span class="model-source">{label.provider}</span>{/if}
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
