import { useEffect, useState } from "react";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { RadioGroup } from "@base-ui/react/radio-group";
import { Radio } from "@base-ui/react/radio";
import {
  Button,
  Card,
  TextField,
  Wordmark,
  cn,
  choiceCard,
  choiceCardIdle,
  choiceCardSelected,
} from "../../design";
import { useConnectProvider, useHeroes, useOpenProject } from "../../ipc/hooks";
import { pickFolder } from "../../ipc/dialog";
import { useSession } from "../../store/session";
import { WindowControls } from "../cockpit/WindowControls";

type Step = "welcome" | "provider";

export function OnboardingFlow() {
  const [step, setStep] = useState<Step>("welcome");
  const reduce = useReducedMotion();
  const setMode = useSession((s) => s.setMode);
  const enterCockpit = useSession((s) => s.enterCockpit);

  // Asymmetric timing: enter deliberate, exit snappy (system response).
  const enterTransition = { duration: reduce ? 0.01 : 0.26, ease: [0.23, 1, 0.32, 1] as const };
  const exitTransition = { duration: reduce ? 0.01 : 0.16, ease: [0.23, 1, 0.32, 1] as const };

  const variants = {
    initial: reduce ? { opacity: 0 } : { opacity: 0, transform: "translateY(10px)" },
    animate: {
      opacity: 1,
      transform: "translateY(0px)",
      transition: enterTransition,
    },
    exit: {
      opacity: 0,
      ...(reduce ? {} : { transform: "translateY(-8px)" }),
      transition: exitTransition,
    },
  };

  function finishOnboarding() {
    setMode("build");
    enterCockpit();
  }

  return (
    <div className="flex min-h-screen flex-col bg-paper">
      <header className="material-chrome flex h-11 shrink-0 select-none items-stretch border-b">
        <div
          data-tauri-drag-region
          className="flex min-w-0 flex-1 items-center px-4"
        >
          <Wordmark className="pointer-events-none text-[15px]" />
        </div>
        <WindowControls />
      </header>

      <div className="grid min-h-0 flex-1 place-items-center px-6 py-10">
        <div className="w-full max-w-lg">
          <AnimatePresence mode="wait">
            <motion.div
              key={step}
              variants={variants}
              initial="initial"
              animate="animate"
              exit="exit"
            >
              {step === "welcome" && <WelcomeStep onNext={() => setStep("provider")} />}
              {step === "provider" && (
                <ProviderStep onBack={() => setStep("welcome")} onNext={finishOnboarding} />
              )}
            </motion.div>
          </AnimatePresence>

          <Stepper step={step} />
        </div>
      </div>
    </div>
  );
}

function Stepper({ step }: { step: Step }) {
  const order: Step[] = ["welcome", "provider"];
  const idx = order.indexOf(step);
  return (
    <div className="mt-7 flex items-center justify-center gap-2" aria-hidden="true">
      {order.map((s, i) => (
        <span
          key={s}
          className={cn(
            "h-1.5 rounded-full",
            "transition-[width,background-color] duration-[var(--duration-fast)] ease-[var(--ease-out)]",
            i === idx ? "w-6 bg-accent" : i < idx ? "w-1.5 bg-accent-line" : "w-1.5 bg-line-2",
          )}
        />
      ))}
    </div>
  );
}

function WelcomeStep({ onNext }: { onNext: () => void }) {
  const openProject = useOpenProject();
  const setActiveProject = useSession((s) => s.setActiveProject);
  const [busy, setBusy] = useState(false);

  async function handleOpen() {
    setBusy(true);
    try {
      const path = await pickFolder();
      if (path) {
        const project = await openProject.mutateAsync(path);
        setActiveProject(project.id);
      }
    } catch {
      /* cancelled or unavailable in preview — fall through to provider step */
    } finally {
      setBusy(false);
      onNext();
    }
  }

  return (
    <Card className="px-10 py-12 text-center shadow-soft-2">
      <Wordmark className="text-[15px]" />
      <h1 className="mt-8 font-serif text-[30px] leading-[1.12] tracking-tight text-balance">
        Build software by describing it.
      </h1>
      <p className="mx-auto mt-3 max-w-[18rem] text-[14.5px] leading-relaxed text-ink-3">
        Local by default. Your keys, your machine.
      </p>
      <div className="mt-9 flex flex-col items-center gap-3">
        <Button onClick={handleOpen} disabled={busy} className="min-w-[12.5rem]">
          {busy ? "Opening…" : "Open a folder"}
        </Button>
        <Button
          variant="ghost"
          onClick={handleOpen}
          disabled={busy}
          className={cn(
            "text-[13px] font-medium text-ink-3 outline-none",
            "transition-colors duration-[var(--duration-press)] ease-[var(--ease-out)]",
            "fine-hover:text-ink focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent",
            "disabled:opacity-50",
          )}
        >
          Or start empty
        </Button>
      </div>
    </Card>
  );
}

/** Mask shown when a provider already has a key on file (real secret never leaves Core). */
const SAVED_KEY_MASK = "•".repeat(28);

function keyForProvider(hasSavedKey: boolean): string {
  return hasSavedKey ? SAVED_KEY_MASK : "";
}

function ProviderStep({ onBack, onNext }: { onBack: () => void; onNext: () => void }) {
  const { data: heroes = [] } = useHeroes();
  const connect = useConnectProvider();
  const [selected, setSelected] = useState<string | null>(null);
  const [apiKey, setApiKey] = useState("");
  const [didAutoSelect, setDidAutoSelect] = useState(false);

  // Prefer a provider that already has a key.
  useEffect(() => {
    if (didAutoSelect || heroes.length === 0) return;
    const saved = heroes.find((h) => h.hasSavedKey);
    if (saved) {
      setSelected(saved.id);
      setApiKey(SAVED_KEY_MASK);
    }
    setDidAutoSelect(true);
  }, [heroes, didAutoSelect]);

  const selectedHero = heroes.find((h) => h.id === selected);
  const selectedName = selectedHero?.name ?? "";
  const hasSavedKey = Boolean(selectedHero?.hasSavedKey);
  const usingSavedKey = hasSavedKey && apiKey === SAVED_KEY_MASK;
  const result = connect.data;
  const ready = result?.probe.ready ?? false;
  const canTest = Boolean(selected) && (usingSavedKey || apiKey.trim().length > 0);

  function selectProvider(id: string) {
    const hero = heroes.find((h) => h.id === id);
    setSelected(id);
    setApiKey(keyForProvider(Boolean(hero?.hasSavedKey)));
    connect.reset();
  }

  function onApiKeyChange(value: string) {
    setApiKey(value);
    // Key changed — require a fresh test before Start.
    if (connect.data || connect.isError) connect.reset();
  }

  function testConnection() {
    if (!selected || !canTest) return;
    // Empty string tells Core to reuse the stored secret.
    connect.mutate({
      providerId: selected,
      apiKey: usingSavedKey ? "" : apiKey.trim(),
    });
  }

  function handlePrimary() {
    if (ready) {
      onNext();
      return;
    }
    testConnection();
  }

  const statusMessage = (() => {
    if (connect.isPending) return null;
    if (ready) {
      return {
        tone: "ok" as const,
        text: `Connected — ${result?.connection.defaultModel ?? "ready"}`,
      };
    }
    if (connect.isError) {
      return {
        tone: "err" as const,
        text:
          connect.error instanceof Error
            ? connect.error.message
            : "Couldn’t reach the provider.",
      };
    }
    if (result && !ready) {
      return {
        tone: "err" as const,
        text: result.probe.error?.message ?? "Not ready — check the key.",
      };
    }
    if (!selected) {
      return { tone: "hint" as const, text: "Choose a provider to continue." };
    }
    if (!canTest) {
      return { tone: "hint" as const, text: "Paste an API key, then test the connection." };
    }
    return { tone: "hint" as const, text: "Test the connection to continue." };
  })();

  const primaryLabel = connect.isPending ? "Testing…" : ready ? "Start" : "Test connection";
  const primaryDisabled = connect.isPending || (!ready && !canTest);

  return (
    <Card className="p-8 shadow-soft-2">
      <h2 className="font-serif text-2xl tracking-tight">Connect a provider</h2>
      <p className="mt-2 text-[14.5px] leading-relaxed text-ink-2">
        Keys stay on this computer. You pay the provider directly.
      </p>

      <RadioGroup
        value={selected}
        onValueChange={(v) => selectProvider(v as string)}
        className="mt-6 grid grid-cols-2 gap-3"
      >
        {heroes.map((h) => {
          const isSelected = selected === h.id;
          const hasKey = Boolean(h.hasSavedKey);
          return (
            <Radio.Root
              key={h.id}
              value={h.id}
              className={cn(
                choiceCard,
                "px-4 py-4 text-[15px] font-semibold outline-none",
                isSelected
                  ? choiceCardSelected
                  : hasKey
                    ? "border-accent-line bg-accent-tint/50 text-ink fine-hover:border-accent"
                    : choiceCardIdle,
              )}
            >
              <span className="block">{h.name}</span>
              {hasKey && (
                <span className="mt-1 block text-[12px] font-medium text-accent">Key on file</span>
              )}
              <Radio.Indicator keepMounted className="sr-only" />
            </Radio.Root>
          );
        })}
      </RadioGroup>

      {selected && (
        <div className="mt-6 border-t border-line pt-5">
          <TextField
            label={`${selectedName} API key`}
            type="password"
            autoComplete="off"
            placeholder="Paste your API key"
            value={apiKey}
            onChange={(e) => onApiKeyChange(e.currentTarget.value)}
            onFocus={(e) => {
              // Selecting the mask makes it easy to replace the whole value.
              if (usingSavedKey) e.currentTarget.select();
            }}
          />
        </div>
      )}

      <div className="mt-8 flex items-center justify-between gap-4">
        <Button variant="ghost" onClick={onBack}>
          Back
        </Button>
        <div className="flex min-w-0 flex-col items-end gap-1.5">
          {statusMessage && (
            <p
              className={cn(
                "max-w-[16rem] text-right text-[12.5px] leading-snug",
                statusMessage.tone === "ok" && "font-medium text-accent",
                statusMessage.tone === "err" && "font-medium text-clay",
                statusMessage.tone === "hint" && "text-ink-3",
              )}
              role="status"
              aria-live="polite"
            >
              {statusMessage.text}
            </p>
          )}
          <Button onClick={handlePrimary} disabled={primaryDisabled}>
            {primaryLabel}
          </Button>
        </div>
      </div>
    </Card>
  );
}
