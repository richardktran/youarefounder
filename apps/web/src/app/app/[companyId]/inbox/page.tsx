import { ComingSoon } from "@/components/ui/coming-soon";

export default function InboxPage() {
  return (
    <div className="p-8">
      <h1 className="text-2xl font-bold text-white mb-2">Inbox</h1>
      <p className="text-zinc-400 mb-8">
        Decisions and hiring proposals that need your approval.
      </p>
      <ComingSoon phase="Phase 5" title="Founder Inbox" />
    </div>
  );
}
