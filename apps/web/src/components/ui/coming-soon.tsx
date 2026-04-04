import { Construction } from "lucide-react";

export function ComingSoon({ phase, title }: { phase: string; title: string }) {
  return (
    <div className="flex h-full min-h-80 items-center justify-center p-12">
      <div className="text-center space-y-4 max-w-sm">
        <div className="inline-flex h-12 w-12 items-center justify-center rounded-xl bg-zinc-800">
          <Construction className="h-6 w-6 text-zinc-400" />
        </div>
        <div>
          <h2 className="text-lg font-semibold text-white">{title}</h2>
          <p className="text-sm text-zinc-500 mt-1">
            Coming in <strong className="text-zinc-400">{phase}</strong>. The
            schema and API surface are already scaffolded — the UI just needs to
            be wired up.
          </p>
        </div>
      </div>
    </div>
  );
}
