<script lang="ts">
  import { ShikiCode } from '@humanspeak/svelte-markdown/extensions/shiki';

  import ContextMenu from '../ContextMenu.svelte';
  import { copyText } from '../../utils/clipboard';

  interface Props {
    lang: string;
    text: string;
  }

  const { lang, text }: Props = $props();
</script>

<ContextMenu
  items={[
    { label: 'Copy code', action: () => copyText(text) },
    { label: 'Copy with Markdown fence', action: () => copyText(`\`\`\`${lang}\n${text}\n\`\`\``) },
  ]}
>
  {#snippet children({ props })}
    <div {...props} class="contents" role="presentation"><ShikiCode {lang} {text} /></div>
  {/snippet}
</ContextMenu>
