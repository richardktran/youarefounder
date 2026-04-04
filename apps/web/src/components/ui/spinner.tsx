import { cn } from "@/lib/utils";

export function Spinner({ className }: { className?: string }) {
  return (
    <div
      className={cn(
        "h-6 w-6 animate-spin rounded-full border-2 border-zinc-700 border-t-zinc-300",
        className
      )}
      role="status"
      aria-label="Loading"
    />
  );
}
