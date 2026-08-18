<script lang="ts">
  import ContextMenu from '../ContextMenu.svelte';
  import { copyText } from '../../utils/clipboard';
  import { menuFromEvent, type ContextMenuState } from '../../utils/context-menu';
  import { fallbackHighlight, highlightCode, normalizeLanguage } from '../../utils/syntax-highlighter';

  interface Props {
    lang: string;
    text: string;
  }

  const { lang, text }: Props = $props();
  let contextMenu = $state<ContextMenuState>();
  let highlighted = $state('');
  let highlightToken = 0;

  $effect(() => {
    const requestedLanguage = normalizeLanguage(lang);
    const requestedText = text;
    const token = ++highlightToken;
    highlighted = fallbackHighlight(requestedText);
    if (requestedLanguage) {
      void highlightCode(requestedText, requestedLanguage).then((html) => {
        if (token === highlightToken) highlighted = html;
      });
    }
  });

  function openMenu(event: MouseEvent) {
    contextMenu = menuFromEvent(event, [
      { label: 'Copy code', action: () => copyText(text) },
      { label: 'Copy with Markdown fence', action: () => copyText(`\`\`\`${lang}\n${text}\n\`\`\``) },
    ]);
  }
</script>

<div class="markdown-code" role="presentation" oncontextmenu={openMenu}>{@html highlighted}</div>

{#if contextMenu}
  <ContextMenu menu={contextMenu} close={() => { contextMenu = undefined; }} />
{/if}
