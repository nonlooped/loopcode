interface LinkAnchor {
  href: string;
  getAttribute(name: string): string | null;
}

interface LinkTarget {
  closest(selector: string): LinkAnchor | null;
}

interface LinkClick {
  button: number;
  target: EventTarget | null;
  composedPath?(): EventTarget[];
  preventDefault(): void;
}

type OpenUrl = (url: string) => Promise<void>;

interface MarkdownLinkActions {
  openUrl: OpenUrl;
  fileLinks?: {
    projectRoot: string;
    open(path: string): void;
  };
}

export async function openMarkdownLinkFromClick(
  event: LinkClick,
  { openUrl, fileLinks }: MarkdownLinkActions,
): Promise<void> {
  if (event.button !== 0) return;
  const target = event.composedPath?.().find(isLinkTarget) ?? event.target;
  if (!isLinkTarget(target)) return;

  const anchor = target.closest("a[href]");
  if (!anchor) return;
  const rawHref = anchor.getAttribute("href");
  if (rawHref?.startsWith("#") || rawHref?.startsWith("?")) return;

  if (fileLinks) {
    const filePath = projectFilePath(rawHref, fileLinks.projectRoot);
    if (filePath) {
      event.preventDefault();
      fileLinks.open(filePath);
      return;
    }
  }

  if (!isWebUrl(anchor.href)) return;
  event.preventDefault();
  await openUrl(anchor.href);
}

function projectFilePath(href: string | null, projectRoot: string): string | null {
  if (
    !href ||
    !projectRoot ||
    href.startsWith("#") ||
    href.startsWith("?") ||
    href.startsWith("//")
  ) {
    return null;
  }

  const windows = projectRoot.includes("\\");
  let candidate = href.replaceAll("\\", "/");
  if (/^[a-z]:\//i.test(candidate)) candidate = `file:///${candidate}`;
  const root = projectRoot.replaceAll("\\", "/");
  const directory = root.endsWith("/") ? root : `${root}/`;
  const base = directory.startsWith("//")
    ? `file:${directory}`
    : `file:///${directory.replace(/^\/+/, "")}`;

  try {
    const url = new URL(candidate, base);
    if (url.protocol !== "file:") return null;
    let path = decodeURIComponent(url.pathname);
    if (!windows) return path;
    if (url.hostname && url.hostname !== "localhost") {
      return `\\\\${url.hostname}${path.replaceAll("/", "\\")}`;
    }
    if (/^\/[a-z]:\//i.test(path)) path = path.slice(1);
    return path.replaceAll("/", "\\");
  } catch {
    return null;
  }
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
