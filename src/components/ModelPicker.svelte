<script lang="ts">
  import { fly } from 'svelte/transition';
  import IconCheck from '@tabler/icons-svelte/icons/check';
  import IconSearch from '@tabler/icons-svelte/icons/search';

  import { profileById, profiles } from '../config/providers';
  import type { ModelOption, ProviderModelCatalog, ThreadState } from '../types';

  interface Props {
    thread: ThreadState;
    catalogs: Record<string, ProviderModelCatalog>;
    reducedMotion: boolean;
    choose: (profileId: string, model: ModelOption) => void;
    retryDiscovery: (profileId: string) => void;
  }

  const props: Props = $props();
  let pickerProviderId = $state<string>();
  let modelSearch = $state('');
  const pickerProfile = $derived(profileById(pickerProviderId ?? props.thread.profileId));
  const pickerProvider = $derived(props.thread.providers[pickerProfile.id]);
  const pickerCatalog = $derived(props.catalogs[pickerProfile.id]);
  const visibleModels = $derived(matchingModels(pickerCatalog.models));

  function matchingModels(models: ModelOption[]) {
    const query = modelSearch.trim().toLocaleLowerCase();
    if (!query) return models;
    return models.filter((model) => `${model.name} ${model.id}`.toLocaleLowerCase().includes(query));
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

<div
  class="model-picker"
  role="dialog"
  aria-label="Choose provider and model"
  tabindex="-1"
  transition:fly|global={{ y: props.reducedMotion ? 0 : 5, duration: props.reducedMotion ? 0 : 140 }}
>
  <nav class="model-providers" aria-label="Providers">
    {#each profiles as profile}
      {@const state = props.thread.providers[profile.id]}
      <button
        class:active={profile.id === pickerProviderId}
        class:selected={profile.id === props.thread.profileId}
        class="model-provider"
        aria-label={profile.label}
        title={profile.label}
        onclick={() => { pickerProviderId = profile.id; modelSearch = ''; }}
      >
        <span class="provider-icon"><img src={profile.icon} alt="" /></span>
        <span
          class:ready={state.connectionStatus === 'ready'}
          class:error={state.connectionStatus === 'error' || props.catalogs[profile.id].status === 'error'}
          class="provider-status"
        ></span>
      </button>
    {/each}
  </nav>
  <section class="model-options" aria-label={`${pickerProfile.label} models`}>
    <label class="model-search">
      <IconSearch size={13} stroke={1.7} />
      <input
        bind:value={modelSearch}
        aria-label="Search models"
        placeholder="Search models…"
        disabled={pickerCatalog.status !== 'ready' || pickerCatalog.models.length === 0}
      />
    </label>
    <div class="model-options-scroll">
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
          {@const isSelected = props.thread.profileId === pickerProfile.id && pickerProvider.selectedModelId === model.id}
          {@const label = modelLabel(model)}
          <button
            class:selected={isSelected}
            class="model-option"
            disabled={pickerProvider.turnStatus === 'running' || pickerProvider.turnStatus === 'blocked'}
            onclick={() => props.choose(pickerProfile.id, model)}
          >
            <span class="model-option-copy"><strong>{label.name}</strong></span>
            {#if label.provider}<span class="model-source">{label.provider}</span>{/if}
            {#if isSelected}<IconCheck size={15} stroke={2} />{/if}
          </button>
        {/each}
      {/if}
    </div>
  </section>
</div>
