<script lang="ts" module>
  import { setShikiHighlighter } from '@humanspeak/svelte-markdown/extensions/shiki';
  import { syntaxHighlighter } from '../../utils/syntax-highlighter';

  setShikiHighlighter(syntaxHighlighter);

  const markdownTypography =
    '[&>:first-child]:mt-0 [&_h1:first-child]:mt-0 [&_h2:first-child]:mt-0 [&_h3:first-child]:mt-0 [&_h4:first-child]:mt-0 [&_h5:first-child]:mt-0 [&_h6:first-child]:mt-0 [&_p:first-child]:mt-0 [&_ul:first-child]:mt-0 [&_ol:first-child]:mt-0 [&_blockquote:first-child]:mt-0 [&_pre:first-child]:mt-0 [&_table:first-child]:mt-0 '
    + '[&>:last-child]:mb-0 [&_h1:last-child]:mb-0 [&_h2:last-child]:mb-0 [&_h3:last-child]:mb-0 [&_h4:last-child]:mb-0 [&_h5:last-child]:mb-0 [&_h6:last-child]:mb-0 [&_p:last-child]:mb-0 [&_ul:last-child]:mb-0 [&_ol:last-child]:mb-0 [&_blockquote:last-child]:mb-0 [&_pre:last-child]:mb-0 [&_table:last-child]:mb-0 '
    + '[&_p]:mb-[0.85em] [&_ul]:mb-[0.85em] [&_ol]:mb-[0.85em] [&_blockquote]:mb-[0.85em] [&_pre]:mb-[0.85em] [&_table]:mb-[0.85em] '
    + '[&_h1]:my-[1.2em_0.55em] [&_h2]:my-[1.2em_0.55em] [&_h3]:my-[1.2em_0.55em] [&_h4]:my-[1.2em_0.55em] [&_h5]:my-[1.2em_0.55em] [&_h6]:my-[1.2em_0.55em] [&_h1]:text-text [&_h2]:text-text [&_h3]:text-text [&_h4]:text-text [&_h5]:text-text [&_h6]:text-text [&_h1]:font-[650] [&_h2]:font-[650] [&_h3]:font-[650] [&_h4]:font-[650] [&_h5]:font-[650] [&_h6]:font-[650] [&_h1]:leading-tight [&_h2]:leading-tight [&_h3]:leading-tight [&_h4]:leading-tight [&_h5]:leading-tight [&_h6]:leading-tight '
    + '[&_h1]:text-[22px] [&_h2]:text-[19px] [&_h3]:text-base [&_h4]:text-base [&_h5]:text-base [&_h6]:text-base '
    + '[&_ul]:pl-[1.6em] [&_ol]:pl-[1.6em] [&_li+li]:mt-[0.2em] '
    + '[&_blockquote]:border-l-2 [&_blockquote]:border-line-strong [&_blockquote]:pl-[0.9em] [&_blockquote]:text-muted '
    + '[&_:not(pre)>code]:rounded [&_code]:border [&_code]:border-line [&_:not(pre)>code]:bg-recessed [&_:not(pre)>code]:px-[0.35em] [&_:not(pre)>code]:py-[0.12em] [&_:not(pre)>code]:font-mono [&_:not(pre)>code]:text-[13px] '
    + '[&_pre.shiki]:max-w-full [&_pre.shiki-fallback]:max-w-full [&_pre.shiki]:overflow-auto [&_pre.shiki-fallback]:overflow-auto [&_pre.shiki]:rounded-[10px] [&_pre.shiki-fallback]:rounded-[10px] [&_pre.shiki]:border [&_pre.shiki-fallback]:border [&_pre.shiki]:border-line [&_pre.shiki-fallback]:border-line [&_pre.shiki]:bg-recessed! [&_pre.shiki-fallback]:bg-recessed! [&_pre.shiki]:px-3.5 [&_pre.shiki-fallback]:px-3.5 [&_pre.shiki]:py-3 [&_pre.shiki-fallback]:py-3 [&_pre.shiki]:font-mono [&_pre.shiki-fallback]:font-mono [&_pre.shiki]:text-[12.5px] [&_pre.shiki-fallback]:text-[12.5px] [&_pre.shiki]:leading-[1.55] [&_pre.shiki-fallback]:leading-[1.55] '
    + '[&_pre_code]:font-[inherit] '
    + '[.wrap-message-code_&_pre.shiki]:break-anywhere [.wrap-message-code_&_pre.shiki-fallback]:break-anywhere [.wrap-message-code_&_pre.shiki]:whitespace-pre-wrap [.wrap-message-code_&_pre.shiki-fallback]:whitespace-pre-wrap [.wrap-message-code_&_pre_code]:break-anywhere [.wrap-message-code_&_pre_code]:whitespace-pre-wrap '
    + '[&_a]:text-text-soft [&_a]:underline-offset-2 [&_a]:decoration-faint hover:[&_a]:text-text hover:[&_a]:decoration-current '
    + '[&_table]:block [&_table]:max-w-full [&_table]:overflow-x-auto [&_table]:border-collapse [&_th]:border [&_td]:border [&_th]:border-line [&_td]:border-line [&_th]:px-[9px] [&_td]:px-[9px] [&_th]:py-1.5 [&_td]:py-1.5 [&_th]:text-left [&_td]:text-left [&_th]:bg-panel-hover [&_th]:text-text-soft';
</script>

<script lang="ts">
  import SvelteMarkdown from '@humanspeak/svelte-markdown';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { openMarkdownLinkFromClick } from '../../utils/external-links';
  import MotionBreathe from '../motion/MotionBreathe.svelte';
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
<div role="presentation" class={markdownTypography} onclick={handleClick}>
  <SvelteMarkdown {source} {streaming} streamId={id} {renderers} streamingText={streaming} />
  {#if streaming}
    <MotionBreathe
      min={0.28}
      max={0.9}
      class="ml-[3px] inline-block h-[0.95em] w-0.5 rounded-full bg-gradient-to-b from-text-soft to-faint align-[-0.12em] shadow-[0_0_7px_color-mix(in_srgb,var(--accent)_16%,transparent)]"
    />
  {/if}
</div>
