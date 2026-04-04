import { cn } from "@/lib/utils";
import React from "react";

interface TextareaProps
  extends React.TextareaHTMLAttributes<HTMLTextAreaElement> {
  label?: string;
  hint?: string;
  error?: string;
}

export function Textarea({
  label,
  hint,
  error,
  className,
  id,
  ...props
}: TextareaProps) {
  const inputId = id ?? label?.toLowerCase().replace(/\s+/g, "-");
  return (
    <div className="space-y-1.5">
      {label && (
        <label
          htmlFor={inputId}
          className="block text-sm font-medium text-zinc-300"
        >
          {label}
        </label>
      )}
      <textarea
        id={inputId}
        className={cn(
          "flex min-h-[80px] w-full rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2",
          "text-sm text-white placeholder:text-zinc-500 resize-none",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400",
          "disabled:cursor-not-allowed disabled:opacity-50",
          error && "border-red-500 focus-visible:ring-red-500",
          className
        )}
        {...props}
      />
      {hint && !error && <p className="text-xs text-zinc-500">{hint}</p>}
      {error && <p className="text-xs text-red-400">{error}</p>}
    </div>
  );
}
