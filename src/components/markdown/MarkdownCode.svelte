<script lang="ts">
  import { ShikiCode } from '@humanspeak/svelte-markdown/extensions/shiki';

  interface Props {
    lang: string;
    text: string;
  }

  const { lang, text }: Props = $props();
</script>

<ShikiCode {lang} {text} />
