import { useUi } from "../../store/ui";
import type { SettingsCategory } from "../../store/ui";
import { settingsCategoryMeta } from "./categories";
import { AboutSection } from "./sections/AboutSection";
import { DataSection } from "./sections/DataSection";
import { GeneralSection } from "./sections/GeneralSection";
import { McpSection } from "./sections/McpSection";
import { ProvidersSection } from "./sections/ProvidersSection";
import { SkillsSection } from "./sections/SkillsSection";

function SectionBody({ category }: { category: SettingsCategory }) {
	switch (category) {
		case "general":
			return <GeneralSection />;
		case "providers":
			return <ProvidersSection />;
		case "skills":
			return <SkillsSection />;
		case "mcp":
			return <McpSection />;
		case "data":
			return <DataSection />;
		case "about":
			return <AboutSection />;
	}
}

/**
 * Main/conversation region while settings mode is active.
 * Category switches are keyboard-frequent — no enter/exit animation (Emil).
 */
export function SettingsView() {
	const category = useUi((s) => s.settingsCategory);
	const meta = settingsCategoryMeta(category);

	return (
		<section
			id="region-settings"
			tabIndex={-1}
			aria-label="Settings"
			className="flex min-h-0 min-w-0 flex-1 flex-col bg-paper"
		>
			<div className="min-h-0 flex-1 overflow-y-auto">
				<div className="mx-auto w-full max-w-2xl px-6 py-8 sm:px-10">
					<header className="mb-8">
						<h1 className="font-serif text-[1.65rem] leading-tight tracking-[-0.02em] text-ink">
							{meta.label}
						</h1>
						<p className="mt-1.5 text-[14px] leading-snug text-ink-3">
							{meta.description}
						</p>
					</header>
					{/* Key remounts content so focus resets cleanly without motion. */}
					<div key={category}>
						<SectionBody category={category} />
					</div>
				</div>
			</div>
		</section>
	);
}
