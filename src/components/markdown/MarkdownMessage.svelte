<script lang="ts" module>
  import { setShikiHighlighter } from '@humanspeak/svelte-markdown/extensions/shiki';
  import { syntaxHighlighter } from '../../utils/syntax-highlighter';

  setShikiHighlighter(syntaxHighlighter);
</script>

<script lang="ts">
  import SvelteMarkdown from '@humanspeak/svelte-markdown';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { openExternalLinkFromClick } from '../../utils/external-links';
  import MarkdownCode from './MarkdownCode.svelte';

  interface Props {
    id: string;
    source: string;
    streaming?: boolean;
  }

  const { id, source, streaming = false }: Props = $props();
  const renderers = { code: MarkdownCode };

  function handleClick(event: MouseEvent): void {
    void openExternalLinkFromClick(event, openUrl);
  }
</script>

<!-- The delegated handler only changes activation of the nested accessible anchors. -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div role="presentation" onclick={handleClick}>
  <SvelteMarkdown {source} {streaming} streamId={id} {renderers} />
</div>
