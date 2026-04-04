"use client";

import { useState } from "react";
import { useParams } from "next/navigation";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  Inbox,
  Plus,
  CheckCircle,
  XCircle,
  Clock,
  ChevronDown,
  ChevronUp,
  User,
  Bot,
  Briefcase,
} from "lucide-react";
import {
  listHiringProposals,
  createHiringProposal,
  acceptHiringProposal,
  declineHiringProposal,
  listAiProfiles,
  type HiringProposal,
  type ProposalStatus,
  type RoleType,
} from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Card } from "@/components/ui/card";
import { Spinner } from "@/components/ui/spinner";
import { cn } from "@/lib/utils";

const ROLE_LABELS: Record<RoleType, string> = {
  co_founder: "Co-founder",
  ceo: "CEO",
  cto: "CTO",
  specialist: "Specialist",
};

const ROLE_COLORS: Record<RoleType, string> = {
  co_founder: "text-purple-400 bg-purple-950/60",
  ceo: "text-blue-400 bg-blue-950/60",
  cto: "text-cyan-400 bg-cyan-950/60",
  specialist: "text-amber-400 bg-amber-950/60",
};

const STATUS_LABELS: Record<ProposalStatus, string> = {
  pending_founder: "Pending review",
  accepted: "Accepted",
  declined: "Declined",
  withdrawn: "Withdrawn",
};

const STATUS_COLORS: Record<ProposalStatus, string> = {
  pending_founder: "text-yellow-400 bg-yellow-950/60",
  accepted: "text-green-400 bg-green-950/60",
  declined: "text-red-400 bg-red-950/60",
  withdrawn: "text-zinc-400 bg-zinc-800",
};

type TabKey = "pending" | "all" | "new";

export default function InboxPage() {
  const params = useParams<{ companyId: string }>();
  const companyId = params.companyId;
  const queryClient = useQueryClient();

  const [tab, setTab] = useState<TabKey>("pending");

  // Decline modal state
  const [decliningId, setDecliningId] = useState<string | null>(null);
  const [declineReason, setDeclineReason] = useState("");

  // Accept note state
  const [acceptingId, setAcceptingId] = useState<string | null>(null);
  const [acceptNote, setAcceptNote] = useState("");

  // New proposal form
  const [showNewForm, setShowNewForm] = useState(false);
  const [newName, setNewName] = useState("");
  const [newRole, setNewRole] = useState<RoleType>("specialist");
  const [newSpecialty, setNewSpecialty] = useState("");
  const [newAiProfileId, setNewAiProfileId] = useState("");
  const [newRationale, setNewRationale] = useState("");
  const [newScope, setNewScope] = useState("");

  // Detail expansion
  const [expandedId, setExpandedId] = useState<string | null>(null);

  const { data: proposals, isLoading } = useQuery({
    queryKey: ["hiring-proposals", companyId],
    queryFn: () => listHiringProposals(companyId),
  });

  const { data: aiProfiles } = useQuery({
    queryKey: ["ai-profiles", companyId],
    queryFn: () => listAiProfiles(companyId),
  });

  const acceptMutation = useMutation({
    mutationFn: ({ proposalId, note }: { proposalId: string; note?: string }) =>
      acceptHiringProposal(companyId, proposalId, note),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["hiring-proposals", companyId] });
      queryClient.invalidateQueries({ queryKey: ["people", companyId] });
      setAcceptingId(null);
      setAcceptNote("");
    },
  });

  const declineMutation = useMutation({
    mutationFn: ({ proposalId, reason }: { proposalId: string; reason: string }) =>
      declineHiringProposal(companyId, proposalId, reason),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["hiring-proposals", companyId] });
      setDecliningId(null);
      setDeclineReason("");
    },
  });

  const createMutation = useMutation({
    mutationFn: () =>
      createHiringProposal(companyId, {
        employee_display_name: newName.trim(),
        role_type: newRole,
        specialty: newSpecialty.trim() || undefined,
        ai_profile_id: newAiProfileId || undefined,
        rationale: newRationale.trim() || undefined,
        scope_of_work: newScope.trim() || undefined,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["hiring-proposals", companyId] });
      setShowNewForm(false);
      setNewName("");
      setNewRole("specialist");
      setNewSpecialty("");
      setNewAiProfileId("");
      setNewRationale("");
      setNewScope("");
      setTab("pending");
    },
  });

  const displayedProposals =
    tab === "pending"
      ? (proposals ?? []).filter((p) => p.status === "pending_founder")
      : (proposals ?? []);

  const pendingCount = (proposals ?? []).filter(
    (p) => p.status === "pending_founder"
  ).length;

  function handleAccept(p: HiringProposal) {
    if (acceptingId === p.id) {
      acceptMutation.mutate({ proposalId: p.id, note: acceptNote || undefined });
    } else {
      setAcceptingId(p.id);
      setDecliningId(null);
    }
  }

  function handleDecline(p: HiringProposal) {
    if (decliningId === p.id) {
      if (!declineReason.trim()) return;
      declineMutation.mutate({ proposalId: p.id, reason: declineReason });
    } else {
      setDecliningId(p.id);
      setAcceptingId(null);
      setDeclineReason("");
    }
  }

  return (
    <div className="p-8 max-w-3xl mx-auto">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-white flex items-center gap-2">
            <Inbox className="h-6 w-6 text-zinc-400" />
            Inbox
          </h1>
          <p className="text-zinc-400 mt-1 text-sm">
            Hiring proposals waiting for your approval.
          </p>
        </div>
        <Button
          size="sm"
          onClick={() => {
            setShowNewForm((v) => !v);
            setTab("new");
          }}
          className="gap-1.5"
        >
          <Plus className="h-4 w-4" />
          New proposal
        </Button>
      </div>

      {/* Tabs */}
      <div className="flex gap-1 mb-6 border-b border-zinc-800">
        {(["pending", "all"] as TabKey[]).map((t) => (
          <button
            key={t}
            onClick={() => {
              setTab(t);
              setShowNewForm(false);
            }}
            className={cn(
              "px-4 py-2 text-sm font-medium border-b-2 -mb-px transition-colors",
              tab === t
                ? "border-white text-white"
                : "border-transparent text-zinc-500 hover:text-zinc-300"
            )}
          >
            {t === "pending" ? (
              <span className="flex items-center gap-1.5">
                Pending
                {pendingCount > 0 && (
                  <span className="bg-yellow-500 text-black text-xs font-semibold rounded-full px-1.5 py-0.5 leading-none">
                    {pendingCount}
                  </span>
                )}
              </span>
            ) : (
              "All proposals"
            )}
          </button>
        ))}
      </div>

      {/* New proposal form */}
      {showNewForm && (
        <Card className="mb-6 p-5 border-zinc-700 bg-zinc-900/60">
          <h2 className="text-sm font-semibold text-white mb-4 flex items-center gap-2">
            <Briefcase className="h-4 w-4 text-zinc-400" />
            Create hiring proposal
          </h2>
          <div className="space-y-3">
            <div>
              <label className="text-xs text-zinc-400 mb-1 block">
                Candidate name *
              </label>
              <Input
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                placeholder='e.g. "Alex — Growth Marketing"'
              />
            </div>

            <div className="grid grid-cols-2 gap-3">
              <div>
                <label className="text-xs text-zinc-400 mb-1 block">Role *</label>
                <select
                  value={newRole}
                  onChange={(e) => setNewRole(e.target.value as RoleType)}
                  className="w-full rounded-md border border-zinc-700 bg-zinc-800 px-3 py-2 text-sm text-white focus:outline-none focus:ring-1 focus:ring-zinc-500"
                >
                  {(Object.keys(ROLE_LABELS) as RoleType[]).map((r) => (
                    <option key={r} value={r}>
                      {ROLE_LABELS[r]}
                    </option>
                  ))}
                </select>
              </div>
              <div>
                <label className="text-xs text-zinc-400 mb-1 block">
                  Specialty (optional)
                </label>
                <Input
                  value={newSpecialty}
                  onChange={(e) => setNewSpecialty(e.target.value)}
                  placeholder='e.g. "market research"'
                />
              </div>
            </div>

            {(aiProfiles ?? []).length > 0 && (
              <div>
                <label className="text-xs text-zinc-400 mb-1 block">
                  AI profile (optional)
                </label>
                <select
                  value={newAiProfileId}
                  onChange={(e) => setNewAiProfileId(e.target.value)}
                  className="w-full rounded-md border border-zinc-700 bg-zinc-800 px-3 py-2 text-sm text-white focus:outline-none focus:ring-1 focus:ring-zinc-500"
                >
                  <option value="">— none selected —</option>
                  {(aiProfiles ?? []).map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.display_name ?? p.model_id} ({p.provider_kind})
                    </option>
                  ))}
                </select>
              </div>
            )}

            <div>
              <label className="text-xs text-zinc-400 mb-1 block">
                Rationale (optional)
              </label>
              <Textarea
                value={newRationale}
                onChange={(e) => setNewRationale(e.target.value)}
                placeholder="Why is this hire needed?"
                rows={2}
              />
            </div>

            <div>
              <label className="text-xs text-zinc-400 mb-1 block">
                Scope of work (optional)
              </label>
              <Textarea
                value={newScope}
                onChange={(e) => setNewScope(e.target.value)}
                placeholder="What will this person be responsible for?"
                rows={2}
              />
            </div>

            <div className="flex gap-2 pt-1">
              <Button
                size="sm"
                onClick={() => createMutation.mutate()}
                disabled={!newName.trim() || createMutation.isPending}
              >
                {createMutation.isPending ? (
                  <Spinner className="h-4 w-4" />
                ) : (
                  "Submit proposal"
                )}
              </Button>
              <Button
                size="sm"
                variant="ghost"
                onClick={() => {
                  setShowNewForm(false);
                  setTab("pending");
                }}
              >
                Cancel
              </Button>
            </div>
            {createMutation.isError && (
              <p className="text-xs text-red-400">
                {String((createMutation.error as Error)?.message ?? "Error")}
              </p>
            )}
          </div>
        </Card>
      )}

      {/* Proposals list */}
      {isLoading ? (
        <div className="flex justify-center py-12">
          <Spinner className="h-6 w-6 text-zinc-500" />
        </div>
      ) : displayedProposals.length === 0 ? (
        <div className="text-center py-16 text-zinc-500">
          <Inbox className="h-8 w-8 mx-auto mb-3 opacity-40" />
          <p className="text-sm">
            {tab === "pending"
              ? "No pending proposals. All clear!"
              : "No proposals yet. Create one to get started."}
          </p>
        </div>
      ) : (
        <div className="space-y-3">
          {displayedProposals.map((proposal) => (
            <ProposalCard
              key={proposal.id}
              proposal={proposal}
              expanded={expandedId === proposal.id}
              onToggleExpand={() =>
                setExpandedId(expandedId === proposal.id ? null : proposal.id)
              }
              acceptingThis={acceptingId === proposal.id}
              decliningThis={decliningId === proposal.id}
              acceptNote={acceptNote}
              declineReason={declineReason}
              onAccept={() => handleAccept(proposal)}
              onDecline={() => handleDecline(proposal)}
              onAcceptNoteChange={setAcceptNote}
              onDeclineReasonChange={setDeclineReason}
              onCancelAction={() => {
                setAcceptingId(null);
                setDecliningId(null);
                setDeclineReason("");
                setAcceptNote("");
              }}
              isAccepting={acceptMutation.isPending && acceptingId === proposal.id}
              isDeclining={declineMutation.isPending && decliningId === proposal.id}
            />
          ))}
        </div>
      )}
    </div>
  );
}

// ─── Proposal card ────────────────────────────────────────────────────────────

interface ProposalCardProps {
  proposal: HiringProposal;
  expanded: boolean;
  onToggleExpand: () => void;
  acceptingThis: boolean;
  decliningThis: boolean;
  acceptNote: string;
  declineReason: string;
  onAccept: () => void;
  onDecline: () => void;
  onAcceptNoteChange: (v: string) => void;
  onDeclineReasonChange: (v: string) => void;
  onCancelAction: () => void;
  isAccepting: boolean;
  isDeclining: boolean;
}

function ProposalCard({
  proposal,
  expanded,
  onToggleExpand,
  acceptingThis,
  decliningThis,
  acceptNote,
  declineReason,
  onAccept,
  onDecline,
  onAcceptNoteChange,
  onDeclineReasonChange,
  onCancelAction,
  isAccepting,
  isDeclining,
}: ProposalCardProps) {
  const isPending = proposal.status === "pending_founder";

  return (
    <Card
      className={cn(
        "border transition-colors",
        isPending ? "border-zinc-700 bg-zinc-900/70" : "border-zinc-800 bg-zinc-950/60"
      )}
    >
      {/* Summary row */}
      <div className="p-4">
        <div className="flex items-start justify-between gap-3">
          <div className="flex items-center gap-3 min-w-0">
            <div className="h-9 w-9 rounded-full bg-zinc-800 flex items-center justify-center shrink-0">
              <Bot className="h-4 w-4 text-zinc-400" />
            </div>
            <div className="min-w-0">
              <p className="text-sm font-semibold text-white truncate">
                {proposal.employee_display_name}
              </p>
              <div className="flex items-center gap-2 mt-0.5 flex-wrap">
                <span
                  className={cn(
                    "text-xs font-medium px-2 py-0.5 rounded-full",
                    ROLE_COLORS[proposal.role_type as RoleType] ??
                      "text-zinc-400 bg-zinc-800"
                  )}
                >
                  {ROLE_LABELS[proposal.role_type as RoleType] ?? proposal.role_type}
                </span>
                {proposal.specialty && (
                  <span className="text-xs text-zinc-500">{proposal.specialty}</span>
                )}
              </div>
            </div>
          </div>

          <div className="flex items-center gap-2 shrink-0">
            <span
              className={cn(
                "text-xs font-medium px-2 py-0.5 rounded-full",
                STATUS_COLORS[proposal.status]
              )}
            >
              {STATUS_LABELS[proposal.status]}
            </span>
            <button
              onClick={onToggleExpand}
              className="text-zinc-500 hover:text-zinc-300 transition-colors"
            >
              {expanded ? (
                <ChevronUp className="h-4 w-4" />
              ) : (
                <ChevronDown className="h-4 w-4" />
              )}
            </button>
          </div>
        </div>

        {/* Inline rationale preview */}
        {!expanded && proposal.rationale && (
          <p className="mt-2 text-xs text-zinc-500 line-clamp-1 pl-12">
            {proposal.rationale}
          </p>
        )}
      </div>

      {/* Expanded detail */}
      {expanded && (
        <div className="px-4 pb-4 border-t border-zinc-800 pt-3 space-y-3">
          {proposal.rationale && (
            <div>
              <p className="text-xs font-medium text-zinc-400 mb-1">Rationale</p>
              <p className="text-sm text-zinc-300 whitespace-pre-wrap">
                {proposal.rationale}
              </p>
            </div>
          )}
          {proposal.scope_of_work && (
            <div>
              <p className="text-xs font-medium text-zinc-400 mb-1">Scope of work</p>
              <p className="text-sm text-zinc-300 whitespace-pre-wrap">
                {proposal.scope_of_work}
              </p>
            </div>
          )}
          {proposal.founder_response_text && !isPending && (
            <div>
              <p className="text-xs font-medium text-zinc-400 mb-1">
                {proposal.status === "accepted" ? "Founder note" : "Decline reason"}
              </p>
              <p className="text-sm text-zinc-300 italic">
                &ldquo;{proposal.founder_response_text}&rdquo;
              </p>
            </div>
          )}
          <p className="text-xs text-zinc-600">
            Submitted {new Date(proposal.created_at).toLocaleDateString()}
          </p>
        </div>
      )}

      {/* Action area — only for pending */}
      {isPending && (
        <div className="px-4 pb-4">
          {!acceptingThis && !decliningThis && (
            <div className="flex gap-2">
              <Button
                size="sm"
                onClick={onAccept}
                className="gap-1.5 bg-green-700 hover:bg-green-600 text-white"
              >
                <CheckCircle className="h-4 w-4" />
                Accept
              </Button>
              <Button
                size="sm"
                variant="ghost"
                onClick={onDecline}
                className="gap-1.5 text-red-400 hover:text-red-300 hover:bg-red-950/40"
              >
                <XCircle className="h-4 w-4" />
                Decline
              </Button>
            </div>
          )}

          {acceptingThis && (
            <div className="space-y-2">
              <Textarea
                value={acceptNote}
                onChange={(e) => onAcceptNoteChange(e.target.value)}
                placeholder="Optional note (e.g. welcome message)…"
                rows={2}
              />
              <div className="flex gap-2">
                <Button
                  size="sm"
                  onClick={onAccept}
                  disabled={isAccepting}
                  className="bg-green-700 hover:bg-green-600 text-white gap-1.5"
                >
                  {isAccepting ? (
                    <Spinner className="h-4 w-4" />
                  ) : (
                    <>
                      <CheckCircle className="h-4 w-4" />
                      Confirm hire
                    </>
                  )}
                </Button>
                <Button size="sm" variant="ghost" onClick={onCancelAction}>
                  Cancel
                </Button>
              </div>
            </div>
          )}

          {decliningThis && (
            <div className="space-y-2">
              <Textarea
                value={declineReason}
                onChange={(e) => onDeclineReasonChange(e.target.value)}
                placeholder="Reason for declining (required)…"
                rows={2}
              />
              <div className="flex gap-2">
                <Button
                  size="sm"
                  onClick={onDecline}
                  disabled={!declineReason.trim() || isDeclining}
                  className="bg-red-800 hover:bg-red-700 text-white gap-1.5"
                >
                  {isDeclining ? (
                    <Spinner className="h-4 w-4" />
                  ) : (
                    <>
                      <XCircle className="h-4 w-4" />
                      Confirm decline
                    </>
                  )}
                </Button>
                <Button size="sm" variant="ghost" onClick={onCancelAction}>
                  Cancel
                </Button>
              </div>
            </div>
          )}
        </div>
      )}
    </Card>
  );
}
