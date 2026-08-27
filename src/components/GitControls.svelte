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
  let worktreeDialogRequested = false;
  let baseBranch = $state('');
  let newBranch = $state('');
  let newBranchInput = $state<HTMLInputElement>();
  let worktreeError = $state('');
  let contextCwd = '';
  const visibleBranches = $derived(matchingBranches(props.branches ?? []));
  const busy = $derived(props.busy || actionPending);

  const trigger =
    'flex h-[22px] min-w-0 max-w-[220px] items-center gap-[5px] overflow-hidden rounded-[5px] border border-transparent bg-transparent px-1 text-[11px] text-muted hover:border-line hover:bg-panel-hover hover:text-text-soft disabled:text-faint [&>svg]:shrink-0 [&>span]:min-w-0 [&>span]:truncate';
  const workspaceTrigger = 'max-w-[180px]';
  const pickerShell =
    'z-40 flex max-h-[min(310px,calc(100vh-150px))] w-[min(300px,calc(100vw-42px))] flex-col overflow-hidden rounded-overlay border border-line bg-floating p-[7px] text-text shadow-overlay backdrop-blur-[20px] backdrop-saturate-[115%]';
  const workspaceShell = 'w-[min(260px,calc(100vw-42px))]';
  const searchField =
    'mx-0.5 mb-[7px] mt-px flex h-8 shrink-0 items-center gap-[7px] rounded-lg border border-line bg-panel px-[9px] text-faint focus-within:border-line-strong focus-within:bg-panel-hover focus-within:text-muted [&_input]:h-full [&_input]:min-w-0 [&_input]:flex-1 [&_input]:border-0 [&_input]:bg-transparent [&_input]:p-0 [&_input]:text-[11px] [&_input]:text-text-soft [&_input]:outline-0 [&_input::placeholder]:text-faint';
  const branchOption =
    'grid min-h-[39px] w-full grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-2.5 rounded-[9px] border border-transparent bg-transparent px-2.5 py-[7px] text-left text-muted hover:border-line hover:bg-panel-hover hover:text-text-soft data-[highlighted]:border-line data-[highlighted]:bg-panel-hover data-[highlighted]:text-text-soft data-[selected]:border-line data-[selected]:bg-panel-active data-[selected]:text-text-soft data-[disabled]:text-faint';
  const workspaceOption =
    'flex min-h-[38px] w-full items-center gap-2 rounded-[7px] border border-transparent bg-transparent px-2 py-1.5 text-left text-muted hover:bg-panel-hover hover:text-text-soft data-[state=checked]:border-line data-[state=checked]:bg-panel-active data-[state=checked]:text-text-soft';
  const modalShell =
    'fixed top-1/2 left-1/2 z-[91] w-[min(400px,calc(100vw-40px))] -translate-x-1/2 -translate-y-1/2 overflow-hidden rounded-overlay border border-line-strong bg-decision p-[18px] shadow-overlay outline-0 backdrop-blur-[20px] backdrop-saturate-[115%] [&_[role=heading]]:text-[15px] [&_[role=heading]]:leading-snug [&_[role=heading]]:font-semibold [&_[role=heading]]:tracking-tight [&_[role=heading]]:text-text [&_[data-dialog-description]]:mt-[5px] [&_[data-dialog-description]]:text-xs [&_[data-dialog-description]]:leading-snug [&_[data-dialog-description]]:text-muted';
  const actionFooter =
    'mt-[18px] flex flex-wrap justify-end gap-[7px] [&_button]:rounded-[7px] [&_button]:border [&_button]:border-line [&_button]:bg-transparent [&_button]:px-3 [&_button]:py-[7px] [&_button]:text-text-soft hover:[&_button]:border-line-strong hover:[&_button]:bg-panel-hover hover:[&_button]:text-text [&_button[type=submit]]:border-transparent [&_button[type=submit]]:bg-accent [&_button[type=submit]]:font-semibold [&_button[type=submit]]:text-accent-contrast hover:[&_button[type=submit]]:bg-accent-hover [&_button[type=submit]:disabled]:opacity-50';
  const fieldLabel =
    'grid gap-[5px] text-[11px] text-muted [&_input]:min-w-0 [&_input]:rounded-[7px] [&_input]:border [&_input]:border-line [&_input]:bg-recessed [&_input]:px-[9px] [&_input]:py-2 [&_input]:text-text [&_input:disabled]:text-faint [&_select]:h-[34px] [&_select]:min-w-0 [&_select]:rounded-[7px] [&_select]:border [&_select]:border-line [&_select]:bg-recessed [&_select]:px-[9px] [&_select]:text-text [&_select:disabled]:text-faint';
  const gitError =
    'mt-[7px] rounded-md border border-[color-mix(in_srgb,var(--danger)_24%,transparent)] bg-[color-mix(in_srgb,var(--danger)_7%,transparent)] px-2 py-[7px] text-[11px] leading-snug text-danger break-anywhere';

  $effect(() => {
    if (contextCwd === props.cwd) return;
    contextCwd = props.cwd;
    workspaceOpen = false;
    pickerOpen = false;
    worktreeOpen = false;
    worktreeDialogRequested = false;
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

  function requestWorktreeDialog() {
    baseBranch = props.branches?.includes(props.currentBranch ?? '')
      ? (props.currentBranch ?? '')
      : (props.branches?.[0] ?? '');
    newBranch = '';
    worktreeError = '';
    worktreeDialogRequested = true;
    workspaceOpen = false;
    pickerOpen = false;
  }

  function finishWorkspaceClose(open: boolean) {
    if (open || !worktreeDialogRequested) return;
    worktreeDialogRequested = false;
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

<div class="flex min-w-0 items-center gap-0.5">
  {#if props.editable && props.currentBranch !== null}
    <DropdownMenu.Root
      open={workspaceOpen}
      onOpenChange={setWorkspaceOpen}
      onOpenChangeComplete={finishWorkspaceClose}
    >
      <DropdownMenu.Trigger
        class="{trigger} {workspaceTrigger}"
        title={workspaceTitle()}
        disabled={props.worktree || props.currentBranch === undefined || !props.branches?.length || busy}
      >
        {#if props.worktree}<IconGitFork size={12} stroke={1.55} />{:else}<IconFolder size={12} stroke={1.55} />{/if}
        <span>{props.worktree ? 'New worktree' : 'Current checkout'}</span>
        {#if !props.worktree}<IconChevronDown size={10} stroke={1.55} />{/if}
      </DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          class="{pickerShell} {workspaceShell}"
          side="top"
          align="end"
          sideOffset={6}
          collisionPadding={12}
          aria-label="Choose workspace"
        >
          <DropdownMenu.RadioGroup
            value={props.worktree ? 'worktree' : 'checkout'}
            onValueChange={(value) => { if (value === 'worktree') requestWorktreeDialog(); }}
          >
            <DropdownMenu.RadioItem class={workspaceOption} value="checkout">
              {#snippet children({ checked })}
                <IconFolder size={13} stroke={1.55} />
                <span class="grid min-w-0 flex-1 gap-px [&_small]:truncate [&_small]:text-[11px] [&_small]:text-faint [&_strong]:truncate [&_strong]:text-xs [&_strong]:font-semibold [&_strong]:text-text-soft">
                  <strong>Current checkout</strong>
                  <small>Use this folder directly</small>
                </span>
                {#if checked}<IconCheck size={13} stroke={2} />{/if}
              {/snippet}
            </DropdownMenu.RadioItem>
            <DropdownMenu.RadioItem class={workspaceOption} value="worktree">
              {#snippet children({ checked })}
                <IconGitFork size={13} stroke={1.55} />
                <span class="grid min-w-0 flex-1 gap-px [&_small]:truncate [&_small]:text-[11px] [&_small]:text-faint [&_strong]:truncate [&_strong]:text-xs [&_strong]:font-semibold [&_strong]:text-text-soft">
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
    class={trigger}
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
      class={pickerShell}
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
        <label class={searchField}>
          <IconSearch size={13} stroke={1.55} />
          <Combobox.Input
            oninput={(event) => { branchSearch = event.currentTarget.value; }}
            aria-label="Search branches"
            placeholder="Search branches…"
          />
        </label>
        <Combobox.ContentStatic class="min-h-0 flex-1 overflow-auto pr-0.5">
          {#if visibleBranches.length === 0}
            <div class="flex min-h-[150px] items-center justify-center p-[22px] text-center text-[11px] leading-snug text-muted">No matching branches.</div>
          {:else}
            {#each visibleBranches as branch (branch)}
              <Combobox.Item class={branchOption} value={branch} label={branch} disabled={busy}>
                {#snippet children({ selected })}
                  <IconGitBranch size={13} stroke={1.55} />
                  <span class="min-w-0 [&_strong]:block [&_strong]:truncate [&_strong]:text-[11px] [&_strong]:font-semibold"><strong>{branch}</strong></span>
                  {#if selected}<IconCheck size={13} stroke={2} />{/if}
                {/snippet}
              </Combobox.Item>
            {/each}
          {/if}
        </Combobox.ContentStatic>
      </Combobox.Root>
      {#if actionError}<div class={gitError} role="alert">{actionError}</div>{/if}
    </Popover.Content>
  </Popover.Portal>
  </Popover.Root>
</div>

<Dialog.Root open={worktreeOpen} onOpenChange={(open) => { if (!busy) worktreeOpen = open; }}>
  <Dialog.Portal>
    <Dialog.Overlay class="fixed inset-0 z-[90] bg-overlay backdrop-blur-[3px]" />
    <Dialog.Content
      class={modalShell}
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
        <div class="mt-[14px] grid gap-3">
          <label class={fieldLabel}>
            <span>Base branch</span>
            <select bind:value={baseBranch} disabled={busy}>
              {#each props.branches ?? [] as branch (branch)}
                <option value={branch}>{branch}</option>
              {/each}
            </select>
          </label>
          <label class={fieldLabel}>
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
        {#if worktreeError}<div class={gitError} role="alert">{worktreeError}</div>{/if}
        <footer class={actionFooter}>
          <button type="button" disabled={busy} onclick={() => { worktreeOpen = false; }}>Cancel</button>
          <button type="submit" disabled={busy || !baseBranch || !newBranch.trim()}>
            {busy ? 'Creating…' : 'Create'}
          </button>
        </footer>
      </form>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
