"use client";

import { useState } from "react";
import { useParams } from "next/navigation";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  Inbox,
  Plus,
  CheckCircle,
  XCircle,
  ChevronDown,
  ChevronUp,
  Bot,
  Briefcase,
  MessageSquare,
  HelpCircle,
  ExternalLink,
} from "lucide-react";
import {
  listHiringProposals,
  createHiringProposal,
  acceptHiringProposal,
  declineHiringProposal,
  listDecisionRequests,
  answerDecisionRequest,
  listAiProfiles,
  listWorkspaces,
  type HiringProposal,
  type DecisionRequest,
  type ProposalStatus,
  type DecisionStatus,
  type RoleType,
} from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Card } from "@/components/ui/card";
import { Spinner } from "@/components/ui/spinner";
import { cn } from "@/lib/utils";

// ─── Constants ────────────────────────────────────────────────────────────────

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

const PROPOSAL_STATUS_LABELS: Record<ProposalStatus, string> = {
  pending_founder: "Pending review",
  accepted: "Accepted",
  declined: "Declined",
  withdrawn: "Withdrawn",
};

const PROPOSAL_STATUS_COLORS: Record<ProposalStatus, string> = {
  pending_founder: "text-yellow-400 bg-yellow-950/60",
  accepted: "text-green-400 bg-green-950/60",
  declined: "text-red-400 bg-red-950/60",
  withdrawn: "text-zinc-400 bg-zinc-800",
};

const DECISION_STATUS_LABELS: Record<DecisionStatus, string> = {
  pending_founder: "Awaiting answer",
  answered: "Answered",
};

const DECISION_STATUS_COLORS: Record<DecisionStatus, string> = {
  pending_founder: "text-orange-400 bg-orange-950/60",
  answered: "text-green-400 bg-green-950/60",
};

type MainTab = "decisions" | "hiring";
type HiringSubTab = "pending" | "all" | "new";

// ─── Page ─────────────────────────────────────────────────────────────────────

export default function InboxPage() {
  const params = useParams<{ companyId: string }>();
  const companyId = params.companyId;
  const queryClient = useQueryClient();

  const [mainTab, setMainTab] = useState<MainTab>("decisions");
  const [hiringSubTab, setHiringSubTab] = useState<HiringSubTab>("pending");

  // ── Decisions state ──────────────────────────────────────────────────────
  const [answeringId, setAnsweringId] = useState<string | null>(null);
  const [answerText, setAnswerText] = useState("");
  const [expandedDecisionId, setExpandedDecisionId] = useState<string | null>(null);

  // ── Hiring state ─────────────────────────────────────────────────────────
  const [decliningId, setDecliningId] = useState<string | null>(null);
  const [declineReason, setDeclineReason] = useState("");
  const [acceptingId, setAcceptingId] = useState<string | null>(null);
  const [acceptNote, setAcceptNote] = useState("");
  const [showNewForm, setShowNewForm] = useState(false);
  const [newName, setNewName] = useState("");
  const [newRole, setNewRole] = useState<RoleType>("specialist");
  const [newSpecialty, setNewSpecialty] = useState("");
  const [newAiProfileId, setNewAiProfileId] = useState("");
  const [newRationale, setNewRationale] = useState("");
  const [newScope, setNewScope] = useState("");
  const [expandedProposalId, setExpandedProposalId] = useState<string | null>(null);

  // ── Queries ───────────────────────────────────────────────────────────────
  const { data: decisions, isLoading: decisionsLoading } = useQuery({
    queryKey: ["decision-requests", companyId],
    queryFn: () => listDecisionRequests(companyId),
  });

  const { data: proposals, isLoading: proposalsLoading } = useQuery({
    queryKey: ["hiring-proposals", companyId],
    queryFn: () => listHiringProposals(companyId),
  });

  const { data: aiProfiles } = useQuery({
    queryKey: ["ai-profiles", companyId],
    queryFn: () => listAiProfiles(companyId),
  });

  // Load workspaces for linking to tickets
  const { data: workspaces } = useQuery({
    queryKey: ["workspaces", companyId],
    queryFn: () => listWorkspaces(companyId),
  });

  // ── Mutations ─────────────────────────────────────────────────────────────
  const answerMutation = useMutation({
    mutationFn: ({ decisionId, answer }: { decisionId: string; answer: string }) =>
      answerDecisionRequest(companyId, decisionId, answer),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["decision-requests", companyId] });
      setAnsweringId(null);
      setAnswerText("");
    },
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
      setHiringSubTab("pending");
    },
  });

  // ── Derived counts ────────────────────────────────────────────────────────
  const pendingDecisions = (decisions ?? []).filter((d) => d.status === "pending_founder");
  const pendingProposals = (proposals ?? []).filter((p) => p.status === "pending_founder");
  const totalPendingCount = pendingDecisions.length + pendingProposals.length;

  const displayedProposals =
    hiringSubTab === "pending"
      ? (proposals ?? []).filter((p) => p.status === "pending_founder")
      : (proposals ?? []);

  // ── Handlers ──────────────────────────────────────────────────────────────
  function handleAnswer(d: DecisionRequest) {
    if (answeringId === d.id) {
      if (!answerText.trim()) return;
      answerMutation.mutate({ decisionId: d.id, answer: answerText });
    } else {
      setAnsweringId(d.id);
      setAnswerText("");
    }
  }

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

  // ── Render ────────────────────────────────────────────────────────────────
  return (
    <div className="p-8 max-w-3xl mx-auto">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-white flex items-center gap-2">
            <Inbox className="h-6 w-6 text-zinc-400" />
            Inbox
            {totalPendingCount > 0 && (
              <span className="ml-1 bg-orange-500 text-black text-sm font-semibold rounded-full px-2 py-0.5 leading-none">
                {totalPendingCount}
              </span>
            )}
          </h1>
          <p className="text-zinc-400 mt-1 text-sm">
            Decisions and hiring proposals waiting for your review.
          </p>
        </div>
        {mainTab === "hiring" && (
          <Button
            size="sm"
            onClick={() => {
              setShowNewForm((v) => !v);
              setHiringSubTab("new");
            }}
            className="gap-1.5"
          >
            <Plus className="h-4 w-4" />
            New proposal
          </Button>
        )}
      </div>

      {/* Main tabs */}
      <div className="flex gap-1 mb-6 border-b border-zinc-800">
        <button
          onClick={() => setMainTab("decisions")}
          className={cn(
            "px-4 py-2 text-sm font-medium border-b-2 -mb-px transition-colors flex items-center gap-1.5",
            mainTab === "decisions"
              ? "border-white text-white"
              : "border-transparent text-zinc-500 hover:text-zinc-300"
          )}
        >
          <HelpCircle className="h-3.5 w-3.5" />
          Decisions
          {pendingDecisions.length > 0 && (
            <span className="bg-orange-500 text-black text-xs font-semibold rounded-full px-1.5 py-0.5 leading-none">
              {pendingDecisions.length}
            </span>
          )}
        </button>
        <button
          onClick={() => setMainTab("hiring")}
          className={cn(
            "px-4 py-2 text-sm font-medium border-b-2 -mb-px transition-colors flex items-center gap-1.5",
            mainTab === "hiring"
              ? "border-white text-white"
              : "border-transparent text-zinc-500 hover:text-zinc-300"
          )}
        >
          <Briefcase className="h-3.5 w-3.5" />
          Hiring
          {pendingProposals.length > 0 && (
            <span className="bg-yellow-500 text-black text-xs font-semibold rounded-full px-1.5 py-0.5 leading-none">
              {pendingProposals.length}
            </span>
          )}
        </button>
      </div>

      {/* ── Decisions tab ─────────────────────────────────────────────────── */}
      {mainTab === "decisions" && (
        <>
          {decisionsLoading ? (
            <div className="flex justify-center py-12">
              <Spinner className="h-6 w-6 text-zinc-500" />
            </div>
          ) : (decisions ?? []).length === 0 ? (
            <div className="text-center py-16 text-zinc-500">
              <HelpCircle className="h-8 w-8 mx-auto mb-3 opacity-40" />
              <p className="text-sm">No decisions yet. When agents need your input, they&apos;ll appear here.</p>
            </div>
          ) : (
            <div className="space-y-3">
              {(decisions ?? []).map((decision) => (
                <DecisionCard
                  key={decision.id}
                  decision={decision}
                  companyId={companyId}
                  workspaces={workspaces ?? []}
                  expanded={expandedDecisionId === decision.id}
                  onToggleExpand={() =>
                    setExpandedDecisionId(expandedDecisionId === decision.id ? null : decision.id)
                  }
                  answeringThis={answeringId === decision.id}
                  answerText={answerText}
                  onAnswer={() => handleAnswer(decision)}
                  onAnswerTextChange={setAnswerText}
                  onCancelAnswer={() => {
                    setAnsweringId(null);
                    setAnswerText("");
                  }}
                  isAnswering={answerMutation.isPending && answeringId === decision.id}
                />
              ))}
            </div>
          )}
        </>
      )}

      {/* ── Hiring tab ────────────────────────────────────────────────────── */}
      {mainTab === "hiring" && (
        <>
          {/* Hiring sub-tabs */}
          <div className="flex gap-1 mb-4 border-b border-zinc-800/60">
            {(["pending", "all"] as HiringSubTab[]).map((t) => (
              <button
                key={t}
                onClick={() => {
                  setHiringSubTab(t);
                  setShowNewForm(false);
                }}
                className={cn(
                  "px-3 py-1.5 text-sm font-medium border-b-2 -mb-px transition-colors",
                  hiringSubTab === t
                    ? "border-white text-white"
                    : "border-transparent text-zinc-500 hover:text-zinc-300"
                )}
              >
                {t === "pending" ? (
                  <span className="flex items-center gap-1.5">
                    Pending
                    {pendingProposals.length > 0 && (
                      <span className="bg-yellow-500 text-black text-xs font-semibold rounded-full px-1.5 py-0.5 leading-none">
                        {pendingProposals.length}
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
                  <label className="text-xs text-zinc-400 mb-1 block">Candidate name *</label>
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
                    <label className="text-xs text-zinc-400 mb-1 block">Specialty (optional)</label>
                    <Input
                      value={newSpecialty}
                      onChange={(e) => setNewSpecialty(e.target.value)}
                      placeholder='e.g. "market research"'
                    />
                  </div>
                </div>
                {(aiProfiles ?? []).length > 0 && (
                  <div>
                    <label className="text-xs text-zinc-400 mb-1 block">AI profile (optional)</label>
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
                  <label className="text-xs text-zinc-400 mb-1 block">Rationale (optional)</label>
                  <Textarea
                    value={newRationale}
                    onChange={(e) => setNewRationale(e.target.value)}
                    placeholder="Why is this hire needed?"
                    rows={2}
                  />
                </div>
                <div>
                  <label className="text-xs text-zinc-400 mb-1 block">Scope of work (optional)</label>
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
                    {createMutation.isPending ? <Spinner className="h-4 w-4" /> : "Submit proposal"}
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => {
                      setShowNewForm(false);
                      setHiringSubTab("pending");
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

          {proposalsLoading ? (
            <div className="flex justify-center py-12">
              <Spinner className="h-6 w-6 text-zinc-500" />
            </div>
          ) : displayedProposals.length === 0 ? (
            <div className="text-center py-16 text-zinc-500">
              <Inbox className="h-8 w-8 mx-auto mb-3 opacity-40" />
              <p className="text-sm">
                {hiringSubTab === "pending"
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
                  expanded={expandedProposalId === proposal.id}
                  onToggleExpand={() =>
                    setExpandedProposalId(expandedProposalId === proposal.id ? null : proposal.id)
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
        </>
      )}
    </div>
  );
}

// ─── Decision card ────────────────────────────────────────────────────────────

interface DecisionCardProps {
  decision: DecisionRequest;
  companyId: string;
  workspaces: { id: string; name: string }[];
  expanded: boolean;
  onToggleExpand: () => void;
  answeringThis: boolean;
  answerText: string;
  onAnswer: () => void;
  onAnswerTextChange: (v: string) => void;
  onCancelAnswer: () => void;
  isAnswering: boolean;
}

function DecisionCard({
  decision,
  companyId,
  workspaces: _workspaces,
  expanded,
  onToggleExpand,
  answeringThis,
  answerText,
  onAnswer,
  onAnswerTextChange,
  onCancelAnswer,
  isAnswering,
}: DecisionCardProps) {
  const isPending = decision.status === "pending_founder";

  return (
    <Card
      className={cn(
        "border transition-colors",
        isPending ? "border-orange-800/60 bg-zinc-900/70" : "border-zinc-800 bg-zinc-950/60"
      )}
    >
      <div className="p-4">
        <div className="flex items-start justify-between gap-3">
          <div className="flex items-start gap-3 min-w-0">
            <div
              className={cn(
                "h-9 w-9 rounded-full flex items-center justify-center shrink-0 mt-0.5",
                isPending ? "bg-orange-950/60" : "bg-zinc-800"
              )}
            >
              <HelpCircle
                className={cn("h-4 w-4", isPending ? "text-orange-400" : "text-zinc-500")}
              />
            </div>
            <div className="min-w-0">
              <p className="text-sm font-semibold text-white leading-snug">
                {decision.question}
              </p>
              <div className="flex items-center gap-2 mt-1 flex-wrap">
                <span
                  className={cn(
                    "text-xs font-medium px-2 py-0.5 rounded-full",
                    DECISION_STATUS_COLORS[decision.status]
                  )}
                >
                  {DECISION_STATUS_LABELS[decision.status]}
                </span>
                <span className="text-xs text-zinc-600">
                  {new Date(decision.created_at).toLocaleDateString()}
                </span>
              </div>
            </div>
          </div>

          <div className="flex items-center gap-2 shrink-0">
            <a
              href={`/app/${companyId}/workspaces`}
              className="text-zinc-600 hover:text-zinc-400 transition-colors"
              title="View ticket"
            >
              <ExternalLink className="h-3.5 w-3.5" />
            </a>
            <button
              onClick={onToggleExpand}
              className="text-zinc-500 hover:text-zinc-300 transition-colors"
            >
              {expanded ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
            </button>
          </div>
        </div>
      </div>

      {expanded && (
        <div className="px-4 pb-3 border-t border-zinc-800 pt-3 space-y-2">
          {decision.context_note && (
            <div>
              <p className="text-xs font-medium text-zinc-400 mb-1">Context</p>
              <p className="text-sm text-zinc-300 whitespace-pre-wrap">{decision.context_note}</p>
            </div>
          )}
          {decision.founder_answer && (
            <div>
              <p className="text-xs font-medium text-zinc-400 mb-1">Your answer</p>
              <p className="text-sm text-zinc-300 italic">&ldquo;{decision.founder_answer}&rdquo;</p>
            </div>
          )}
        </div>
      )}

      {isPending && (
        <div className="px-4 pb-4">
          {!answeringThis ? (
            <Button
              size="sm"
              onClick={onAnswer}
              className="gap-1.5 bg-orange-700 hover:bg-orange-600 text-white"
            >
              <MessageSquare className="h-4 w-4" />
              Answer
            </Button>
          ) : (
            <div className="space-y-2">
              <Textarea
                value={answerText}
                onChange={(e) => onAnswerTextChange(e.target.value)}
                placeholder="Your answer (will be added as a comment and unblock the ticket)…"
                rows={3}
                autoFocus
              />
              <div className="flex gap-2">
                <Button
                  size="sm"
                  onClick={onAnswer}
                  disabled={!answerText.trim() || isAnswering}
                  className="bg-orange-700 hover:bg-orange-600 text-white gap-1.5"
                >
                  {isAnswering ? (
                    <Spinner className="h-4 w-4" />
                  ) : (
                    <>
                      <CheckCircle className="h-4 w-4" />
                      Submit answer
                    </>
                  )}
                </Button>
                <Button size="sm" variant="ghost" onClick={onCancelAnswer}>
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
                    ROLE_COLORS[proposal.role_type as RoleType] ?? "text-zinc-400 bg-zinc-800"
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
                PROPOSAL_STATUS_COLORS[proposal.status]
              )}
            >
              {PROPOSAL_STATUS_LABELS[proposal.status]}
            </span>
            <button
              onClick={onToggleExpand}
              className="text-zinc-500 hover:text-zinc-300 transition-colors"
            >
              {expanded ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
            </button>
          </div>
        </div>

        {!expanded && proposal.rationale && (
          <p className="mt-2 text-xs text-zinc-500 line-clamp-1 pl-12">{proposal.rationale}</p>
        )}
      </div>

      {expanded && (
        <div className="px-4 pb-4 border-t border-zinc-800 pt-3 space-y-3">
          {proposal.rationale && (
            <div>
              <p className="text-xs font-medium text-zinc-400 mb-1">Rationale</p>
              <p className="text-sm text-zinc-300 whitespace-pre-wrap">{proposal.rationale}</p>
            </div>
          )}
          {proposal.scope_of_work && (
            <div>
              <p className="text-xs font-medium text-zinc-400 mb-1">Scope of work</p>
              <p className="text-sm text-zinc-300 whitespace-pre-wrap">{proposal.scope_of_work}</p>
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
