<script lang="ts" module>
  import bash from 'shiki/langs/bash.mjs';
  import css from 'shiki/langs/css.mjs';
  import diff from 'shiki/langs/diff.mjs';
  import html from 'shiki/langs/html.mjs';
  import javascript from 'shiki/langs/javascript.mjs';
  import json from 'shiki/langs/json.mjs';
  import markdown from 'shiki/langs/markdown.mjs';
  import powershell from 'shiki/langs/powershell.mjs';
  import python from 'shiki/langs/python.mjs';
  import rust from 'shiki/langs/rust.mjs';
  import svelte from 'shiki/langs/svelte.mjs';
  import toml from 'shiki/langs/toml.mjs';
  import tsx from 'shiki/langs/tsx.mjs';
  import typescript from 'shiki/langs/typescript.mjs';
  import yaml from 'shiki/langs/yaml.mjs';
  import githubDark from 'shiki/themes/github-dark.mjs';
  import {
    createShikiHighlighter,
    setShikiHighlighter,
  } from '@humanspeak/svelte-markdown/extensions/shiki';

  setShikiHighlighter(
    createShikiHighlighter({
      langs: [
        bash,
        css,
        diff,
        html,
        javascript,
        json,
        markdown,
        powershell,
        python,
        rust,
        svelte,
        toml,
        tsx,
        typescript,
        yaml,
      ],
      themes: [githubDark],
    }),
  );
</script>

<script lang="ts">
  import SvelteMarkdown from '@humanspeak/svelte-markdown';
  import MarkdownCode from './MarkdownCode.svelte';

  interface Props {
    id: string;
    source: string;
    streaming?: boolean;
  }

  const { id, source, streaming = false }: Props = $props();
  const renderers = { code: MarkdownCode };
</script>

<SvelteMarkdown {source} {streaming} streamId={id} {renderers} />
