import { ComingSoon } from "@/components/ui/coming-soon";

export default function WorkspacesPage() {
  return (
    <div className="p-8">
      <h1 className="text-2xl font-bold text-white mb-2">Workspaces</h1>
      <p className="text-zinc-400 mb-8">Project areas for your AI team.</p>
      <ComingSoon phase="Phase 2" title="Workspaces & Tickets" />
    </div>
  );
}
