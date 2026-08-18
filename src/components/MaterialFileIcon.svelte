<script lang="ts">
  interface Props {
    name: string;
    directory: boolean;
    expanded: boolean;
  }

  const { name, directory, expanded }: Props = $props();
  let src = $state('');
  let loadToken = 0;

  $effect(() => {
    const requestedName = name;
    const requestedDirectory = directory;
    const requestedExpanded = expanded;
    const token = ++loadToken;
    void import('../utils/material-file-icons').then((icons) => {
      if (token !== loadToken) return;
      src = requestedDirectory
        ? icons.materialFolderIcon(requestedName, requestedExpanded)
        : icons.materialFileIcon(requestedName);
    });
  });
</script>

{#if src}<img class="project-file-icon" {src} alt="" />{/if}
