<script lang="ts" module>
  import { setShikiHighlighter } from '@humanspeak/svelte-markdown/extensions/shiki';
  import { syntaxHighlighter } from '../../utils/syntax-highlighter';

  setShikiHighlighter(syntaxHighlighter);
</script>

<script lang="ts">
  import SvelteMarkdown from '@humanspeak/svelte-markdown';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { openMarkdownLinkFromClick } from '../../utils/external-links';
  import MarkdownCode from './MarkdownCode.svelte';
  import StreamingText from './StreamingText.svelte';

  interface Props {
    id: string;
    source: string;
    streaming?: boolean;
    fileLinks?: {
      projectRoot: string;
      open: (path: string) => void;
    };
  }

  const { id, source, streaming = false, fileLinks }: Props = $props();
  const renderers = { code: MarkdownCode, rawtext: StreamingText };

  function handleClick(event: MouseEvent): void {
    void openMarkdownLinkFromClick(event, { openUrl, fileLinks });
  }
</script>

<!-- The delegated handler only changes activation of the nested accessible anchors. -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div role="presentation" onclick={handleClick}>
  <SvelteMarkdown {source} {streaming} streamId={id} {renderers} streamingText={streaming} />
</div>
