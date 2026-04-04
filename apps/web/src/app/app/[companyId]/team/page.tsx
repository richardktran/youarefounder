import { ComingSoon } from "@/components/ui/coming-soon";

export default function TeamPage() {
  return (
    <div className="p-8">
      <h1 className="text-2xl font-bold text-white mb-2">Team</h1>
      <p className="text-zinc-400 mb-8">
        Your AI workforce — co-founders, executives, and specialists.
      </p>
      <ComingSoon phase="Phase 1" title="AI Co-founder & Team" />
    </div>
  );
}
