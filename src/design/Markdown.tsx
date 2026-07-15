import { Children, isValidElement, type ComponentPropsWithoutRef, type ReactNode } from "react";
import { Streamdown } from "streamdown";
import { code } from "@streamdown/code";
import { cn } from "./cn";
import { copyText } from "../lib/clipboard";
import { ContextMenu } from "./ContextMenu";

const plugins = { code } as const;

function MarkdownLink({
  href,
  children,
  ...rest
}: ComponentPropsWithoutRef<"a">) {
  return (
    <a
      {...rest}
      href={href}
      target="_blank"
      rel="noopener noreferrer"
    >
      {children}
    </a>
  );
}

function textFromNode(node: ReactNode): string {
  if (typeof node === "string" || typeof node === "number") return String(node);
  if (!isValidElement<{ children?: ReactNode }>(node)) return "";
  return Children.toArray(node.props.children).map(textFromNode).join("");
}

/** Code blocks keep useful text actions after the WebView menu is suppressed. */
function MarkdownCodeBlock({ children, ...rest }: ComponentPropsWithoutRef<"pre">) {
  const code = textFromNode(children);
  return (
    <ContextMenu
      items={[
        { label: "Copy code", onSelect: () => void copyText(code) },
        { label: "Copy with path", disabled: true },
        { label: "Insert into composer", disabled: true },
        { label: "Apply to file", disabled: true },
      ]}
    >
      <pre {...rest}>{children}</pre>
    </ContextMenu>
  );
}

const components = { a: MarkdownLink, pre: MarkdownCodeBlock };

export type MarkdownProps = {
  children: string;
  /** When true, shows streaming caret and disables copy controls. */
  isAnimating?: boolean;
  /** Prefer static for finished/history content; streaming for live assistant output. */
  mode?: "streaming" | "static";
  className?: string;
  /** Streaming caret glyph; only visible when isAnimating is true. */
  caret?: "block" | "circle";
};

/**
 * Streaming-aware markdown renderer with Shiki code highlighting.
 * Shared plugins object keeps Streamdown memoization stable across rows.
 */
export function Markdown({
  children,
  isAnimating = false,
  mode = "streaming",
  className,
  caret = "block",
}: MarkdownProps) {
  return (
    <Streamdown
      mode={mode}
      plugins={plugins}
      components={components}
      linkSafety={{ enabled: true }}
      isAnimating={isAnimating}
      caret={isAnimating ? caret : undefined}
      className={cn(
        "text-[15px] leading-relaxed break-words text-ink",
        "[&_a]:text-accent [&_a]:underline-offset-2 fine-hover:[&_a]:underline",
        "[&_pre]:text-[13px]",
        className,
      )}
    >
      {children}
    </Streamdown>
  );
}
