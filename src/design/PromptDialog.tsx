import { useEffect, useId, useState } from "react";
import { Dialog } from "@base-ui/react/dialog";
import { Button } from "./Button";
import { cn } from "./cn";
import { inputChrome } from "./interactive";

/**
 * Themed replacement for `window.prompt`: a small Base UI dialog matching
 * RestoreCheckpointWizard's surface language (material scrim, panel radius,
 * hairline header/footer).
 */
export function PromptDialog({
  open,
  onOpenChange,
  title,
  description,
  defaultValue,
  confirmLabel = "Save",
  onConfirm,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description?: string;
  defaultValue: string;
  confirmLabel?: string;
  onConfirm: (value: string) => void;
}) {
  const [value, setValue] = useState(defaultValue);
  const inputId = useId();

  useEffect(() => {
    if (open) setValue(defaultValue);
  }, [open, defaultValue]);

  function submit() {
    const trimmed = value.trim();
    if (!trimmed) return;
    onConfirm(trimmed);
    onOpenChange(false);
  }

  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Backdrop className="material-scrim fixed inset-0 z-40" />
        <Dialog.Popup
          className={
            "material-chrome fixed left-1/2 top-[20%] z-50 w-[min(24rem,92vw)] -translate-x-1/2 " +
            "overflow-hidden rounded-panel border shadow-soft-3 outline-none"
          }
        >
          <div className="border-b border-line px-4 py-3">
            <Dialog.Title className="text-[15px] font-semibold text-ink">{title}</Dialog.Title>
            {description && (
              <Dialog.Description className="mt-0.5 text-[12.5px] text-ink-3">
                {description}
              </Dialog.Description>
            )}
          </div>

          <div className="px-4 py-3">
            <label htmlFor={inputId} className="sr-only">
              {title}
            </label>
            <input
              id={inputId}
              autoFocus
              value={value}
              onChange={(e) => setValue(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  submit();
                }
              }}
              className={cn(inputChrome, "w-full")}
            />
          </div>

          <div className="flex items-center justify-end gap-2 border-t border-line px-4 py-3">
            <Button variant="ghost" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button variant="primary" disabled={!value.trim()} onClick={submit}>
              {confirmLabel}
            </Button>
          </div>
        </Dialog.Popup>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

/** Themed replacement for `window.confirm` for destructive actions. */
export function ConfirmDialog({
  open,
  onOpenChange,
  title,
  description,
  confirmLabel = "Confirm",
  onConfirm,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description?: string;
  confirmLabel?: string;
  onConfirm: () => void;
}) {
  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Backdrop className="material-scrim fixed inset-0 z-40" />
        <Dialog.Popup
          className={
            "material-chrome fixed left-1/2 top-[20%] z-50 w-[min(24rem,92vw)] -translate-x-1/2 " +
            "overflow-hidden rounded-panel border shadow-soft-3 outline-none"
          }
        >
          <div className="border-b border-line px-4 py-3">
            <Dialog.Title className="text-[15px] font-semibold text-ink">{title}</Dialog.Title>
            {description && (
              <Dialog.Description className="mt-0.5 text-[12.5px] text-ink-3">
                {description}
              </Dialog.Description>
            )}
          </div>

          <div className="flex items-center justify-end gap-2 border-t border-line px-4 py-3">
            <Button variant="ghost" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button
              variant="danger"
              onClick={() => {
                onConfirm();
                onOpenChange(false);
              }}
            >
              {confirmLabel}
            </Button>
          </div>
        </Dialog.Popup>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
