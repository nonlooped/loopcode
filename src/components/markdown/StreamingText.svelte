<script lang="ts">
  interface Props {
    text: string;
    streamingText?: boolean;
  }

  const { text, streamingText = false }: Props = $props();
  let initialized = false;
  let previousText = '';
  let stableText = $state('');
  let revealedText = $state('');

  $effect.pre(() => {
    if (!initialized) {
      initialized = true;
      previousText = text;
      stableText = text;
      return;
    }
    if (!streamingText || !text.startsWith(previousText)) {
      previousText = text;
      stableText = text;
      revealedText = '';
      return;
    }
    if (text === previousText) return;
    stableText = previousText;
    revealedText = text.slice(previousText.length);
    previousText = text;
  });
</script>

{stableText}{#if revealedText}{#key text}<span class="stream-chunk">{revealedText}</span>{/key}{/if}
