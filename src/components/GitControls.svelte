<script lang="ts">
  import { Combobox, Dialog, DropdownMenu, Popover } from 'bits-ui';
  import {
    IconCheck,
    IconChevronDown,
    IconFolder,
    IconGitBranch,
    IconGitFork,
    IconSearch,
  } from '@tabler/icons-svelte';

  interface Props {
    cwd: string;
    currentBranch: string | null | undefined;
    branches: string[] | null | undefined;
    worktree: boolean;
    editable: boolean;
    lockReason?: string;
    busy: boolean;
    switchBranch: (branch: string) => Promise<void>;
    createWorktree: (baseBranch: string, branch: string) => Promise<void>;
  }

  const props: Props = $props();
  let workspaceOpen = $state(false);
  let pickerOpen = $state(false);
  let branchListOpen = $state(false);
  let branchSearch = $state('');
  let actionPending = $state(false);
  let actionError = $state('');
  let worktreeOpen = $state(false);
  let baseBranch = $state('');
  let newBranch = $state('');
  let newBranchInput = $state<HTMLInputElement>();
  let worktreeError = $state('');
  let contextCwd = '';
  const visibleBranches = $derived(matchingBranches(props.branches ?? []));
  const busy = $derived(props.busy || actionPending);

  $effect(() => {
    if (contextCwd === props.cwd) return;
    contextCwd = props.cwd;
    workspaceOpen = false;
    pickerOpen = false;
    worktreeOpen = false;
    actionError = '';
    worktreeError = '';
  });

  function matchingBranches(branches: string[]) {
    const query = branchSearch.trim().toLocaleLowerCase();
    return query
      ? branches.filter((branch) => branch.toLocaleLowerCase().includes(query))
      : branches;
  }

  function setWorkspaceOpen(open: boolean) {
    workspaceOpen = open;
    if (open) pickerOpen = false;
  }

  function setPickerOpen(open: boolean) {
    pickerOpen = open;
    if (!open) return;
    workspaceOpen = false;
    branchSearch = '';
    branchListOpen = true;
    actionError = '';
  }

  async function chooseBranch(branch: string) {
    if (!branch || branch === props.currentBranch || busy) return;
    actionPending = true;
    actionError = '';
    try {
      await props.switchBranch(branch);
      pickerOpen = false;
    } catch (error) {
      actionError = errorMessage(error);
    } finally {
      actionPending = false;
    }
  }

  function openWorktreeDialog() {
    baseBranch = props.branches?.includes(props.currentBranch ?? '')
      ? (props.currentBranch ?? '')
      : (props.branches?.[0] ?? '');
    newBranch = '';
    worktreeError = '';
    workspaceOpen = false;
    pickerOpen = false;
    worktreeOpen = true;
  }

  async function submitWorktree() {
    const branch = newBranch.trim();
    if (!baseBranch || !branch || busy) return;
    actionPending = true;
    worktreeError = '';
    try {
      await props.createWorktree(baseBranch, branch);
      worktreeOpen = false;
    } catch (error) {
      worktreeError = errorMessage(error);
    } finally {
      actionPending = false;
    }
  }

  function branchTitle() {
    if (props.lockReason) return props.lockReason;
    if (props.currentBranch === undefined) return 'Resolving Git branches…';
    if (props.currentBranch === null) return 'No Git repository';
    return `${props.currentBranch}${props.worktree ? ' · worktree' : ''}\n${props.cwd}`;
  }

  function workspaceTitle() {
    return `${props.worktree ? 'New worktree' : 'Current checkout'}\n${props.cwd}`;
  }

  function errorMessage(cause: unknown) {
    return cause instanceof Error ? cause.message : String(cause);
  }
</script>

<div class="git-controls">
  {#if props.editable && props.currentBranch !== null}
    <DropdownMenu.Root open={workspaceOpen} onOpenChange={setWorkspaceOpen}>
      <DropdownMenu.Trigger
        class="git-picker-trigger git-workspace-trigger"
        title={workspaceTitle()}
        disabled={props.worktree || props.currentBranch === undefined || !props.branches?.length || busy}
      >
        {#if props.worktree}<IconGitFork size={12} stroke={1.55} />{:else}<IconFolder size={12} stroke={1.55} />{/if}
        <span>{props.worktree ? 'New worktree' : 'Current checkout'}</span>
        {#if !props.worktree}<IconChevronDown size={10} stroke={1.55} />{/if}
      </DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          class="workspace-dropdown git-workspace-picker"
          side="top"
          align="end"
          sideOffset={6}
          collisionPadding={12}
          aria-label="Choose workspace"
        >
          <DropdownMenu.RadioGroup
            value={props.worktree ? 'worktree' : 'checkout'}
            onValueChange={(value) => { if (value === 'worktree') openWorktreeDialog(); }}
          >
            <DropdownMenu.RadioItem class="workspace-option" value="checkout">
              {#snippet children({ checked })}
                <IconFolder size={13} stroke={1.55} />
                <span class="workspace-option-main">
                  <strong>Current checkout</strong>
                  <small>Use this folder directly</small>
                </span>
                {#if checked}<IconCheck size={13} stroke={2} />{/if}
              {/snippet}
            </DropdownMenu.RadioItem>
            <DropdownMenu.RadioItem class="workspace-option" value="worktree">
              {#snippet children({ checked })}
                <IconGitFork size={13} stroke={1.55} />
                <span class="workspace-option-main">
                  <strong>New worktree</strong>
                  <small>Create a separate branch and folder</small>
                </span>
                {#if checked}<IconCheck size={13} stroke={2} />{/if}
              {/snippet}
            </DropdownMenu.RadioItem>
          </DropdownMenu.RadioGroup>
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  {/if}

  <Popover.Root open={pickerOpen} onOpenChange={setPickerOpen}>
  <Popover.Trigger
    class="git-picker-trigger"
    title={branchTitle()}
    disabled={!props.editable || props.currentBranch == null || !props.branches?.length || busy}
  >
    <IconGitBranch size={12} stroke={1.55} />
    <span>
      {props.currentBranch === undefined
        ? 'Resolving branch…'
        : props.currentBranch === null
          ? 'No Git branch'
          : `${props.currentBranch}${props.worktree ? ' · worktree' : ''}`}
    </span>
    {#if props.editable && props.currentBranch !== null}
      <IconChevronDown size={10} stroke={1.55} />
    {/if}
  </Popover.Trigger>
  <Popover.Portal>
    <Popover.Content
      class="git-picker"
      side="top"
      align="end"
      sideOffset={6}
      collisionPadding={12}
      aria-label="Choose Git branch"
    >
      <Combobox.Root
        type="single"
        value={props.currentBranch ?? ''}
        onValueChange={(branch) => { void chooseBranch(branch); }}
        open={branchListOpen}
        onOpenChange={(open) => { branchListOpen = open; }}
        inputValue={branchSearch}
        allowDeselect={false}
      >
        <label class="model-search">
          <IconSearch size={13} stroke={1.55} />
          <Combobox.Input
            oninput={(event) => { branchSearch = event.currentTarget.value; }}
            aria-label="Search branches"
            placeholder="Search branches…"
          />
        </label>
        <Combobox.ContentStatic class="model-options-scroll">
          {#if visibleBranches.length === 0}
            <div class="model-picker-state">No matching branches.</div>
          {:else}
            {#each visibleBranches as branch (branch)}
              <Combobox.Item class="model-option" value={branch} label={branch} disabled={busy}>
                {#snippet children({ selected })}
                  <IconGitBranch size={13} stroke={1.55} />
                  <span class="model-option-copy"><strong>{branch}</strong></span>
                  {#if selected}<IconCheck size={13} stroke={2} />{/if}
                {/snippet}
              </Combobox.Item>
            {/each}
          {/if}
        </Combobox.ContentStatic>
      </Combobox.Root>
      {#if actionError}<div class="git-error" role="alert">{actionError}</div>{/if}
    </Popover.Content>
  </Popover.Portal>
  </Popover.Root>
</div>

<Dialog.Root open={worktreeOpen} onOpenChange={(open) => { if (!busy) worktreeOpen = open; }}>
  <Dialog.Portal>
    <Dialog.Overlay class="modal-overlay" />
    <Dialog.Content
      class="permission-modal confirmation-modal git-worktree-modal"
      onOpenAutoFocus={(event) => {
        event.preventDefault();
        newBranchInput?.focus();
      }}
    >
      <Dialog.Title>Create worktree</Dialog.Title>
      <Dialog.Description>
        Create a managed worktree for this thread. The project stays the same.
      </Dialog.Description>
      <form onsubmit={(event) => { event.preventDefault(); void submitWorktree(); }}>
        <div class="git-worktree-fields">
          <label class="rename-thread-field">
            <span>Base branch</span>
            <select bind:value={baseBranch} disabled={busy}>
              {#each props.branches ?? [] as branch (branch)}
                <option value={branch}>{branch}</option>
              {/each}
            </select>
          </label>
          <label class="rename-thread-field">
            <span>New branch</span>
            <input
              bind:this={newBranchInput}
              bind:value={newBranch}
              maxlength="256"
              autocomplete="off"
              spellcheck="false"
              placeholder="feature/my-change"
              disabled={busy}
            />
          </label>
        </div>
        {#if worktreeError}<div class="git-error" role="alert">{worktreeError}</div>{/if}
        <footer class="confirmation-actions">
          <button type="button" disabled={busy} onclick={() => { worktreeOpen = false; }}>Cancel</button>
          <button type="submit" disabled={busy || !baseBranch || !newBranch.trim()}>
            {busy ? 'Creating…' : 'Create'}
          </button>
        </footer>
      </form>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
