<script lang="ts">
  import { ShikiCode } from '@humanspeak/svelte-markdown/extensions/shiki';

  import ContextMenu from '../ContextMenu.svelte';
  import { copyText } from '../../utils/clipboard';
  import { menuFromEvent, type ContextMenuState } from '../../utils/context-menu';

  interface Props {
    lang: string;
    text: string;
  }

  const { lang, text }: Props = $props();
  let contextMenu = $state<ContextMenuState>();

  function openMenu(event: MouseEvent) {
    contextMenu = menuFromEvent(event, [
      { label: 'Copy code', action: () => copyText(text) },
      { label: 'Copy with Markdown fence', action: () => copyText(`\`\`\`${lang}\n${text}\n\`\`\``) },
    ]);
  }
</script>

<div class="markdown-code" role="presentation" oncontextmenu={openMenu}><ShikiCode {lang} {text} /></div>

{#if contextMenu}
  <ContextMenu menu={contextMenu} close={() => { contextMenu = undefined; }} />
{/if}
