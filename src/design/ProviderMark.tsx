import { cn } from "./cn";
import anthropicMark from "@lobehub/icons-static-svg/icons/anthropic.svg?raw";
import openaiMark from "@lobehub/icons-static-svg/icons/openai.svg?raw";
import opencodeMark from "@lobehub/icons-static-svg/icons/opencode.svg?raw";
import openrouterMark from "@lobehub/icons-static-svg/icons/openrouter.svg?raw";

/**
 * Brand marks for the first-party providers. MIT-licensed static SVGs from
 * @lobehub/icons-static-svg; every mark uses `currentColor`, so they follow
 * the surrounding ink token in both themes.
 */
const MARKS: Record<string, string> = {
	openai: openaiMark,
	anthropic: anthropicMark,
	openrouter: openrouterMark,
	opencode: opencodeMark,
};

/**
 * Inline provider glyph sized in px; falls back to a monogram from `name`
 * for providers without a bundled mark.
 */
export function ProviderMark({
	providerId,
	name,
	size = 16,
	className,
}: {
	providerId: string;
	name?: string;
	size?: number;
	className?: string;
}) {
	const mark = MARKS[providerId];
	if (!mark) {
		return (
			<span
				aria-hidden="true"
				className={cn("font-semibold text-ink-2", className)}
				style={{ fontSize: Math.round(size * 0.75), lineHeight: 1 }}
			>
				{(name ?? providerId).slice(0, 1).toUpperCase()}
			</span>
		);
	}
	return (
		<span
			aria-hidden="true"
			className={cn("inline-block shrink-0", className)}
			style={{ fontSize: size, lineHeight: 0 }}
			dangerouslySetInnerHTML={{ __html: mark }}
		/>
	);
}
