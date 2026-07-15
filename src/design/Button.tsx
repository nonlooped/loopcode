import type { ButtonHTMLAttributes } from "react";
import { Button as BaseButton } from "@base-ui/react/button";
import { cn } from "./cn";

export type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";

export const buttonBase =
  "inline-flex items-center justify-center gap-2 rounded-control text-sm font-semibold px-4 py-2.5 " +
  "cursor-pointer select-none " +
  "transition-[background-color,border-color,transform,box-shadow,filter] " +
  "duration-[var(--duration-press)] ease-[var(--ease-out)] " +
  "active:scale-[0.97] " +
  "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent " +
  "disabled:opacity-50 disabled:pointer-events-none disabled:active:scale-100";

const variants: Record<ButtonVariant, string> = {
  primary:
    "bg-accent text-accent-foreground shadow-soft-1 border border-transparent " +
    "fine-hover:bg-accent-strong",
  secondary:
    "bg-surface text-ink border border-line-2 " +
    "fine-hover:border-ink-3",
  ghost:
    "bg-transparent text-ink-2 border border-transparent " +
    "fine-hover:bg-surface fine-hover:text-ink",
  danger:
    "bg-clay-tint text-clay border border-transparent " +
    "fine-hover:brightness-95",
};

export const buttonPrimary = variants.primary;
export const buttonSecondary = variants.secondary;
export const buttonGhost = variants.ghost;
export const buttonDanger = variants.danger;

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
}

export function Button({ variant = "primary", className, type = "button", disabled, ...props }: ButtonProps) {
  return (
    <BaseButton
      type={type}
      disabled={disabled}
      className={cn(buttonBase, variants[variant], className)}
      {...props}
    />
  );
}