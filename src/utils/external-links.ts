interface LinkTarget {
  closest(selector: string): { href: string } | null;
}

interface LinkClick {
  button: number;
  target: EventTarget | null;
  preventDefault(): void;
}

type OpenUrl = (url: string) => Promise<void>;

export async function openExternalLinkFromClick(event: LinkClick, openUrl: OpenUrl): Promise<void> {
  if (event.button !== 0 || !isLinkTarget(event.target)) return;

  const href = event.target.closest("a[href]")?.href;
  if (!href || !isWebUrl(href)) return;

  event.preventDefault();
  await openUrl(href);
}

function isLinkTarget(target: EventTarget | null): target is EventTarget & LinkTarget {
  return target !== null && "closest" in target && typeof target.closest === "function";
}

function isWebUrl(href: string): boolean {
  try {
    const url = new URL(href);
    return url.protocol === "http:" || url.protocol === "https:";
  } catch {
    return false;
  }
}
