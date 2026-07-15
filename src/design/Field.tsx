import type { ComponentPropsWithoutRef, ReactNode } from "react";
import { Field } from "@base-ui/react/field";
import { Input } from "@base-ui/react/input";
import { cn } from "./cn";
import { inputChrome } from "./interactive";

interface TextFieldProps extends Omit<ComponentPropsWithoutRef<typeof Input>, "className"> {
  label: ReactNode;
  hint?: ReactNode;
  className?: string;
}

/** Labelled text input built on Base UI Field (label association + error slot). */
export function TextField({ label, hint, className, ...inputProps }: TextFieldProps) {
  return (
    <Field.Root className="flex flex-col gap-1.5">
      <Field.Label className="text-[13px] font-semibold tracking-wide text-ink-2">{label}</Field.Label>
      <Input className={cn(inputChrome, className)} {...inputProps} />
      {hint && <Field.Description className="text-[13px] text-ink-3">{hint}</Field.Description>}
      <Field.Error className="text-[13px] text-clay" />
    </Field.Root>
  );
}
