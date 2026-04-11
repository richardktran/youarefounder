"use client";

import { useEffect, useRef, useState } from "react";
import { useParams } from "next/navigation";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  ArrowLeft,
  MessageSquare,
  Send,
  User,
  Bot,
  UserCircle,
  Zap,
  ChevronDown,
  ChevronRight,
} from "lucide-react";
import Link from "next/link";
import {
  getTicket,
  getWorkspace,
  listTickets,
  listComments,
  createComment,
  updateTicket,
  listWorkspaceMembers,
  listTicketAgentRuns,
  listTicketReferences,
  createTicketReference,
  deleteTicketReference,
  type Ticket,
  type TicketStatus,
  type TicketPriority,
  type TicketType,
  type WorkspaceMember,
  type AgentRun,
} from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Card } from "@/components/ui/card";
import { Spinner } from "@/components/ui/spinner";
import { cn } from "@/lib/utils";

const STATUS_OPTIONS: { value: TicketStatus; label: string; dot: string; color: string }[] = [
  { value: "backlog", label: "Backlog", dot: "bg-zinc-500", color: "text-zinc-400" },
  { value: "todo", label: "Todo", dot: "bg-blue-500", color: "text-blue-400" },
  { value: "in_progress", label: "In Progress", dot: "bg-amber-500", color: "text-amber-400" },
  { value: "blocked", label: "Blocked", dot: "bg-red-500", color: "text-red-400" },
  { value: "done", label: "Done", dot: "bg-green-500", color: "text-green-400" },
  { value: "cancelled", label: "Cancelled", dot: "bg-zinc-700", color: "text-zinc-500" },
];

const PRIORITY_OPTIONS: { value: TicketPriority; label: string; color: string }[] = [
  { value: "low", label: "Low", color: "text-zinc-500" },
  { value: "medium", label: "Medium", color: "text-amber-500" },
  { value: "high", label: "High", color: "text-red-500" },
];

const TYPE_OPTIONS: { value: TicketType; label: string }[] = [
  { value: "task", label: "Task" },
  { value: "epic", label: "Epic" },
  { value: "research", label: "Research" },
];

export default function TicketPage() {
  const params = useParams<{
    companyId: string;
    workspaceId: string;
    ticketId: string;
  }>();
  const { companyId, workspaceId, ticketId } = params;
  const queryClient = useQueryClient();

  const [commentBody, setCommentBody] = useState("");
  const [editingDesc, setEditingDesc] = useState(false);
  const [description, setDescription] = useState("");
  const [editingDod, setEditingDod] = useState(false);
  const [definitionOfDone, setDefinitionOfDone] = useState("");
  const [editingFounderMem, setEditingFounderMem] = useState(false);
  const [founderMemory, setFounderMemory] = useState("");
  const [editingOutcome, setEditingOutcome] = useState(false);
  const [outcomeSummary, setOutcomeSummary] = useState("");
  const [refTargetId, setRefTargetId] = useState("");

  const { data: workspace } = useQuery({
    queryKey: ["workspace", workspaceId],
    queryFn: () => getWorkspace(companyId, workspaceId),
  });

  const { data: ticket, isLoading: ticketLoading } = useQuery({
    queryKey: ["ticket", ticketId],
    queryFn: () => getTicket(companyId, workspaceId, ticketId),
  });

  const { data: parentTicket } = useQuery({
    queryKey: ["ticket", ticket?.parent_ticket_id],
    queryFn: () =>
      getTicket(companyId, workspaceId, ticket!.parent_ticket_id!),
    enabled: !!ticket?.parent_ticket_id,
  });

  const { data: subtasks = [] } = useQuery({
    queryKey: ["ticket-subtasks", workspaceId, ticketId],
    queryFn: () =>
      listTickets(companyId, workspaceId, { parentTicketId: ticketId }),
    enabled: !!ticket,
    refetchInterval: 4000,
  });

  const { data: comments, isLoading: commentsLoading } = useQuery({
    queryKey: ["comments", ticketId],
    queryFn: () => listComments(companyId, workspaceId, ticketId),
    // AI comments are written during agent runs; keep in sync with run history without a full reload.
    refetchInterval: 4000,
  });

  const { data: wsMembers = [] } = useQuery({
    queryKey: ["workspace-members", workspaceId],
    queryFn: () => listWorkspaceMembers(companyId, workspaceId),
  });

  const { data: agentRuns } = useQuery({
    queryKey: ["agent-runs", ticketId],
    queryFn: () => listTicketAgentRuns(companyId, workspaceId, ticketId),
    refetchInterval: 4000,
  });

  const { data: ticketRefs = [] } = useQuery({
    queryKey: ["ticket-refs", ticketId],
    queryFn: () => listTicketReferences(companyId, workspaceId, ticketId),
    enabled: !!ticket,
  });

  const prevAgentRunIds = useRef<string>("");
  useEffect(() => {
    const ids = agentRuns?.map((r) => r.id).join(",") ?? "";
    if (!ids) return;
    if (prevAgentRunIds.current && ids !== prevAgentRunIds.current) {
      queryClient.invalidateQueries({ queryKey: ["comments", ticketId] });
    }
    prevAgentRunIds.current = ids;
  }, [agentRuns, queryClient, ticketId]);

  const updateMutation = useMutation({
    mutationFn: (patch: Parameters<typeof updateTicket>[3]) =>
      updateTicket(companyId, workspaceId, ticketId, patch),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["ticket", ticketId] });
      queryClient.invalidateQueries({ queryKey: ["tickets", companyId, workspaceId] });
      queryClient.invalidateQueries({ queryKey: ["ticket-subtasks", workspaceId] });
      const t = queryClient.getQueryData<Ticket>(["ticket", ticketId]);
      if (t?.parent_ticket_id) {
        queryClient.invalidateQueries({ queryKey: ["ticket", t.parent_ticket_id] });
      }
    },
  });

  const addRefMutation = useMutation({
    mutationFn: (toId: string) =>
      createTicketReference(companyId, workspaceId, ticketId, { to_ticket_id: toId.trim() }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["ticket-refs", ticketId] });
      setRefTargetId("");
    },
  });

  const removeRefMutation = useMutation({
    mutationFn: (toId: string) =>
      deleteTicketReference(companyId, workspaceId, ticketId, toId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["ticket-refs", ticketId] });
    },
  });

  useEffect(() => {
    if (!ticket) return;
    setOutcomeSummary(ticket.outcome_summary ?? "");
  }, [ticket?.id, ticket?.outcome_summary]);

  const commentMutation = useMutation({
    mutationFn: () =>
      createComment(companyId, workspaceId, ticketId, {
        body: commentBody.trim(),
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["comments", ticketId] });
      setCommentBody("");
    },
  });

  if (ticketLoading) {
    return (
      <div className="flex h-full items-center justify-center p-12">
        <Spinner />
      </div>
    );
  }

  if (!ticket) return null;

  const statusCfg = STATUS_OPTIONS.find((s) => s.value === ticket.status);
  const assigneeMember = wsMembers.find((m: WorkspaceMember) => m.person_id === ticket.assignee_person_id);

  return (
    <div className="p-8 max-w-4xl mx-auto">
      {/* Breadcrumb */}
      <Link
        href={`/app/${companyId}/workspaces/${workspaceId}`}
        className="inline-flex items-center gap-1.5 text-sm text-zinc-500 hover:text-zinc-300 transition-colors mb-6"
      >
        <ArrowLeft className="h-3.5 w-3.5" />
        {workspace?.name ?? "Workspace"}
      </Link>

      {ticket.parent_ticket_id ? (
        <div className="mb-4 rounded-lg border border-zinc-800 bg-zinc-900/40 px-3 py-2 text-sm">
          <span className="text-zinc-500">Subtask of </span>
          <Link
            href={`/app/${companyId}/workspaces/${workspaceId}/tickets/${ticket.parent_ticket_id}`}
            className="text-amber-400/90 hover:text-amber-300 font-medium"
          >
            {parentTicket?.title ?? "Parent task"}
          </Link>
        </div>
      ) : null}

      <div className="grid grid-cols-1 gap-8 lg:grid-cols-[1fr_260px]">
        {/* Main column */}
        <div className="space-y-6">
          {/* Title + status badge */}
          <div>
            <div className="flex items-center gap-2 mb-2">
              {statusCfg && (
                <span
                  className={cn(
                    "inline-flex items-center gap-1.5 rounded-full border border-zinc-800 bg-zinc-900 px-2.5 py-1 text-xs font-medium",
                    statusCfg.color
                  )}
                >
                  <span className={cn("h-1.5 w-1.5 rounded-full", statusCfg.dot)} />
                  {statusCfg.label}
                </span>
              )}
            </div>
            <h1 className="text-2xl font-bold text-white leading-snug">
              {ticket.title}
            </h1>
          </div>

          {/* Description */}
          <section className="space-y-2">
            <h2 className="text-xs font-medium text-zinc-500 uppercase tracking-wider">
              Description
            </h2>
            {editingDesc ? (
              <div className="space-y-2">
                <Textarea
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  rows={5}
                  autoFocus
                  placeholder="Describe this ticket…"
                />
                <div className="flex gap-2 justify-end">
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => setEditingDesc(false)}
                  >
                    Cancel
                  </Button>
                  <Button
                    size="sm"
                    isLoading={updateMutation.isPending}
                    onClick={() => {
                      updateMutation.mutate(
                        { description: description.trim() || undefined },
                        { onSuccess: () => setEditingDesc(false) }
                      );
                    }}
                  >
                    Save
                  </Button>
                </div>
              </div>
            ) : (
              <div
                className="rounded-lg border border-zinc-800 bg-zinc-900/30 px-4 py-3 cursor-pointer hover:border-zinc-700 transition-colors min-h-[80px]"
                onClick={() => {
                  setDescription(ticket.description ?? "");
                  setEditingDesc(true);
                }}
              >
                {ticket.description ? (
                  <p className="text-sm text-zinc-300 whitespace-pre-wrap">
                    {ticket.description}
                  </p>
                ) : (
                  <p className="text-sm text-zinc-600">
                    Click to add a description…
                  </p>
                )}
              </div>
            )}
          </section>

          {/* Definition of done */}
          <section className="space-y-2">
            <h2 className="text-xs font-medium text-zinc-500 uppercase tracking-wider">
              Definition of done
            </h2>
            <p className="text-[11px] text-zinc-600 -mt-1 mb-1">
              Concrete checks before this ticket should be marked complete. Agents use this to know when to set status to done.
            </p>
            {editingDod ? (
              <div className="space-y-2">
                <Textarea
                  value={definitionOfDone}
                  onChange={(e) => setDefinitionOfDone(e.target.value)}
                  rows={4}
                  autoFocus
                  placeholder="- Criterion one&#10;- Criterion two"
                />
                <div className="flex gap-2 justify-end">
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => setEditingDod(false)}
                  >
                    Cancel
                  </Button>
                  <Button
                    size="sm"
                    isLoading={updateMutation.isPending}
                    onClick={() => {
                      updateMutation.mutate(
                        { definition_of_done: definitionOfDone.trim() },
                        { onSuccess: () => setEditingDod(false) }
                      );
                    }}
                  >
                    Save
                  </Button>
                </div>
              </div>
            ) : (
              <div
                className="rounded-lg border border-amber-900/40 bg-amber-950/20 px-4 py-3 cursor-pointer hover:border-amber-800/50 transition-colors min-h-[64px]"
                onClick={() => {
                  setDefinitionOfDone(ticket.definition_of_done ?? "");
                  setEditingDod(true);
                }}
              >
                {ticket.definition_of_done ? (
                  <p className="text-sm text-zinc-300 whitespace-pre-wrap">
                    {ticket.definition_of_done}
                  </p>
                ) : (
                  <p className="text-sm text-zinc-600">
                    Click to set definition of done (checklist for completion)…
                  </p>
                )}
              </div>
            )}
          </section>

          {/* Outcome summary (optional; helps cross-ticket snapshots) */}
          <section className="space-y-2">
            <h2 className="text-xs font-medium text-zinc-500 uppercase tracking-wider">
              Outcome summary
            </h2>
            <p className="text-[11px] text-zinc-600 -mt-1 mb-1">
              Short note on what shipped or was decided. Included when other tickets reference this one.
            </p>
            {editingOutcome ? (
              <div className="space-y-2">
                <Textarea
                  value={outcomeSummary}
                  onChange={(e) => setOutcomeSummary(e.target.value)}
                  rows={3}
                  autoFocus
                  placeholder="e.g. Chose Postgres + Drizzle; migration path documented in thread."
                />
                <div className="flex gap-2 justify-end">
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => setEditingOutcome(false)}
                  >
                    Cancel
                  </Button>
                  <Button
                    size="sm"
                    isLoading={updateMutation.isPending}
                    onClick={() => {
                      updateMutation.mutate(
                        { outcome_summary: outcomeSummary.trim() || null },
                        { onSuccess: () => setEditingOutcome(false) }
                      );
                    }}
                  >
                    Save
                  </Button>
                </div>
              </div>
            ) : (
              <div
                className="rounded-lg border border-zinc-800 bg-zinc-900/30 px-4 py-3 cursor-pointer hover:border-zinc-700 transition-colors min-h-[48px]"
                onClick={() => {
                  setOutcomeSummary(ticket.outcome_summary ?? "");
                  setEditingOutcome(true);
                }}
              >
                {ticket.outcome_summary ? (
                  <p className="text-sm text-zinc-300 whitespace-pre-wrap">
                    {ticket.outcome_summary}
                  </p>
                ) : (
                  <p className="text-sm text-zinc-600">
                    Click to add an outcome summary…
                  </p>
                )}
              </div>
            )}
          </section>

          {/* Founder memory (this ticket) */}
          <section className="space-y-2">
            <h2 className="text-xs font-medium text-zinc-500 uppercase tracking-wider">
              Founder memory
            </h2>
            <p className="text-[11px] text-zinc-600 -mt-1 mb-1">
              Sticky instructions for agents on this ticket only. Shown in every agent run together with company memory in Settings.
            </p>
            {editingFounderMem ? (
              <div className="space-y-2">
                <Textarea
                  value={founderMemory}
                  onChange={(e) => setFounderMemory(e.target.value)}
                  rows={4}
                  autoFocus
                  placeholder="- Must align with Q4 roadmap doc&#10;- Do not escalate pricing without a draft&#10;- ..."
                />
                <div className="flex gap-2 justify-end">
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => setEditingFounderMem(false)}
                  >
                    Cancel
                  </Button>
                  <Button
                    size="sm"
                    isLoading={updateMutation.isPending}
                    onClick={() => {
                      updateMutation.mutate(
                        { founder_memory: founderMemory.trim() },
                        { onSuccess: () => setEditingFounderMem(false) }
                      );
                    }}
                  >
                    Save
                  </Button>
                </div>
              </div>
            ) : (
              <div
                className="rounded-lg border border-violet-900/40 bg-violet-950/20 px-4 py-3 cursor-pointer hover:border-violet-800/50 transition-colors min-h-[64px]"
                onClick={() => {
                  setFounderMemory(ticket.founder_memory ?? "");
                  setEditingFounderMem(true);
                }}
              >
                {ticket.founder_memory ? (
                  <p className="text-sm text-zinc-300 whitespace-pre-wrap">
                    {ticket.founder_memory}
                  </p>
                ) : (
                  <p className="text-sm text-zinc-600">
                    Click to add founder memory for this ticket…
                  </p>
                )}
              </div>
            )}
          </section>

          {/* Referenced tickets (cross-ticket memory) */}
          <section className="space-y-2">
            <h2 className="text-xs font-medium text-zinc-500 uppercase tracking-wider">
              Referenced tickets
            </h2>
            <p className="text-[11px] text-zinc-600 -mt-1 mb-1">
              Link other tickets by id so agents load their outcome into context on the next run.
              Same company only.
            </p>
            <div className="flex flex-col sm:flex-row gap-2 sm:items-end">
              <Input
                label="Ticket UUID to link"
                value={refTargetId}
                onChange={(e) => setRefTargetId(e.target.value)}
                placeholder="xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
                className="font-mono text-xs"
              />
              <Button
                size="sm"
                variant="outline"
                disabled={!refTargetId.trim()}
                isLoading={addRefMutation.isPending}
                onClick={() => addRefMutation.mutate(refTargetId)}
              >
                Add reference
              </Button>
            </div>
            {ticketRefs.length === 0 ? (
              <p className="text-sm text-zinc-600 italic">No references yet.</p>
            ) : (
              <ul className="space-y-2">
                {ticketRefs.map((r) => (
                  <li
                    key={r.to_ticket_id}
                    className="flex items-start justify-between gap-3 rounded-lg border border-zinc-800 bg-zinc-900/40 px-3 py-2"
                  >
                    <div className="min-w-0">
                      <Link
                        href={`/app/${companyId}/workspaces/${workspaceId}/tickets/${r.to_ticket_id}`}
                        className="text-sm text-sky-400/90 hover:text-sky-300 font-mono break-all"
                      >
                        {r.to_ticket_id}
                      </Link>
                      {r.note ? (
                        <p className="text-xs text-zinc-500 mt-1">{r.note}</p>
                      ) : null}
                    </div>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="shrink-0 text-zinc-500"
                      isLoading={removeRefMutation.isPending}
                      onClick={() => removeRefMutation.mutate(r.to_ticket_id)}
                    >
                      Remove
                    </Button>
                  </li>
                ))}
              </ul>
            )}
          </section>

          {/* Subtasks (direct children only — one level under top-level tickets) */}
          <section className="space-y-3">
            <h2 className="text-xs font-medium text-zinc-500 uppercase tracking-wider">
              Subtasks
            </h2>
            {subtasks.length === 0 ? (
              <p className="text-sm text-zinc-600 italic">
                No subtasks yet. Agents attach work here with{" "}
                <code className="text-zinc-500">create_subtask</code> so it stays under this ticket.
              </p>
            ) : (
              <ul className="space-y-2">
                {subtasks.map((st) => {
                  const stStatus = STATUS_OPTIONS.find((s) => s.value === st.status);
                  return (
                    <li key={st.id}>
                      <Link
                        href={`/app/${companyId}/workspaces/${workspaceId}/tickets/${st.id}`}
                        className="flex items-start gap-3 rounded-lg border border-zinc-800 bg-zinc-900/40 px-3 py-2.5 hover:border-zinc-700 transition-colors"
                      >
                        {stStatus && (
                          <span
                            className={cn(
                              "mt-0.5 inline-flex shrink-0 items-center gap-1 rounded-full border border-zinc-800 bg-zinc-900 px-2 py-0.5 text-[10px] font-medium",
                              stStatus.color
                            )}
                          >
                            <span className={cn("h-1 w-1 rounded-full", stStatus.dot)} />
                            {stStatus.label}
                          </span>
                        )}
                        <span className="text-sm text-zinc-200 leading-snug">{st.title}</span>
                      </Link>
                    </li>
                  );
                })}
              </ul>
            )}
          </section>

          {/* Comments */}
          <section className="space-y-4">
            <h2 className="text-xs font-medium text-zinc-500 uppercase tracking-wider flex items-center gap-1.5">
              <MessageSquare className="h-3.5 w-3.5" />
              Comments
              {comments?.length ? (
                <span className="rounded-full bg-zinc-800 px-1.5 py-0.5 text-[10px] font-bold text-zinc-400">
                  {comments.length}
                </span>
              ) : null}
            </h2>

            {commentsLoading ? (
              <Spinner />
            ) : !comments?.length ? (
              <p className="text-sm text-zinc-600 italic">No comments yet. Be the first to comment.</p>
            ) : (
              <div className="space-y-3">
                {comments.map((c) => {
                  const author = wsMembers.find((m: WorkspaceMember) => m.person_id === c.author_person_id);
                  return (
                    <div key={c.id} className="flex gap-3">
                      {/* Avatar */}
                      <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-zinc-800 ring-1 ring-zinc-700 mt-0.5">
                        {author ? (
                          author.person_kind === "ai_agent" ? (
                            <Bot className="h-3.5 w-3.5 text-blue-400" />
                          ) : (
                            <span className="text-[10px] font-bold text-zinc-300 uppercase">
                              {author.display_name.slice(0, 2)}
                            </span>
                          )
                        ) : (
                          <UserCircle className="h-3.5 w-3.5 text-zinc-600" />
                        )}
                      </div>
                      {/* Bubble */}
                      <div className="flex-1 min-w-0">
                        <div className="flex items-baseline gap-2 mb-1">
                          <span className="text-xs font-semibold text-zinc-300">
                            {author?.display_name ?? "Unknown"}
                          </span>
                          <span className="text-[10px] text-zinc-600">
                            {new Date(c.created_at).toLocaleString()}
                          </span>
                        </div>
                        <div className="rounded-lg border border-zinc-800 bg-zinc-900/50 px-3 py-2">
                          <p className="text-sm text-zinc-300 whitespace-pre-wrap">{c.body}</p>
                        </div>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}

            {/* Add comment */}
            <div className="flex gap-3">
              <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-zinc-800 ring-1 ring-zinc-700 mt-1">
                <User className="h-3.5 w-3.5 text-zinc-500" />
              </div>
              <div className="flex-1 space-y-2">
                <Textarea
                  placeholder="Leave a comment…"
                  value={commentBody}
                  onChange={(e) => setCommentBody(e.target.value)}
                  rows={3}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && (e.metaKey || e.ctrlKey) && commentBody.trim()) {
                      commentMutation.mutate();
                    }
                  }}
                />
                <div className="flex items-center justify-between">
                  <p className="text-[10px] text-zinc-700">⌘ + Enter to submit</p>
                  <Button
                    size="sm"
                    isLoading={commentMutation.isPending}
                    disabled={!commentBody.trim()}
                    onClick={() => commentMutation.mutate()}
                  >
                    <Send className="h-3.5 w-3.5" />
                    Comment
                  </Button>
                </div>
              </div>
            </div>
          </section>
          {/* Agent run history */}
          {agentRuns && agentRuns.length > 0 && (
            <AgentRunHistory runs={agentRuns} />
          )}
        </div>

        {/* Sidebar */}
        <aside className="space-y-1">
          <p className="text-[10px] font-semibold text-zinc-600 uppercase tracking-widest mb-3">
            Properties
          </p>

          {/* Status */}
          <SidebarField label="Status">
            <select
              value={ticket.status}
              onChange={(e) =>
                updateMutation.mutate({ status: e.target.value as TicketStatus })
              }
              className="w-full rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-white focus:outline-none focus:ring-1 focus:ring-zinc-500"
            >
              {STATUS_OPTIONS.map((s) => (
                <option key={s.value} value={s.value}>
                  {s.label}
                </option>
              ))}
            </select>
          </SidebarField>

          {/* Assignee */}
          <SidebarField label="Assignee">
            <div className="flex items-center gap-2">
              {assigneeMember ? (
                <div className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-zinc-700">
                  {assigneeMember.person_kind === "ai_agent" ? (
                    <Bot className="h-3 w-3 text-blue-400" />
                  ) : (
                    <span className="text-[9px] font-bold text-zinc-300 uppercase">
                      {assigneeMember.display_name.slice(0, 2)}
                    </span>
                  )}
                </div>
              ) : null}
              <select
                value={ticket.assignee_person_id ?? ""}
                onChange={(e) =>
                  updateMutation.mutate({
                    assignee_person_id: e.target.value || null,
                  })
                }
                className="flex-1 rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-white focus:outline-none focus:ring-1 focus:ring-zinc-500"
              >
                <option value="">Unassigned</option>
                {wsMembers.map((m: WorkspaceMember) => (
                  <option key={m.person_id} value={m.person_id}>
                    {m.display_name}
                  </option>
                ))}
              </select>
            </div>
          </SidebarField>

          {/* Priority */}
          <SidebarField label="Priority">
            <select
              value={ticket.priority}
              onChange={(e) =>
                updateMutation.mutate({ priority: e.target.value as TicketPriority })
              }
              className="w-full rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-white focus:outline-none focus:ring-1 focus:ring-zinc-500"
            >
              {PRIORITY_OPTIONS.map((p) => (
                <option key={p.value} value={p.value}>
                  {p.label}
                </option>
              ))}
            </select>
          </SidebarField>

          {/* Type */}
          <SidebarField label="Type">
            <select
              value={ticket.ticket_type}
              onChange={(e) =>
                updateMutation.mutate({ ticket_type: e.target.value as TicketType })
              }
              className="w-full rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-white focus:outline-none focus:ring-1 focus:ring-zinc-500"
            >
              {TYPE_OPTIONS.map((t) => (
                <option key={t.value} value={t.value}>
                  {t.label}
                </option>
              ))}
            </select>
          </SidebarField>

          {/* Agent runs summary */}
          {agentRuns && agentRuns.length > 0 && (
            <div className="pt-2">
              <p className="text-[10px] font-semibold text-zinc-600 uppercase tracking-widest mb-1">
                Agent
              </p>
              <p className="text-[10px] text-zinc-600">
                {agentRuns.length} run{agentRuns.length !== 1 ? "s" : ""} total
              </p>
            </div>
          )}

          {/* Metadata */}
          <div className="pt-4 border-t border-zinc-800 space-y-1.5">
            <p className="text-[10px] text-zinc-700">
              Created {new Date(ticket.created_at).toLocaleDateString("en-US", { month: "short", day: "numeric", year: "numeric" })}
            </p>
            <p className="text-[10px] text-zinc-700">
              Updated {new Date(ticket.updated_at).toLocaleDateString("en-US", { month: "short", day: "numeric", year: "numeric" })}
            </p>
          </div>
        </aside>
      </div>
    </div>
  );
}

function SidebarField({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="rounded-lg border border-zinc-800/60 bg-zinc-900/30 p-3 space-y-1.5">
      <p className="text-[10px] font-medium text-zinc-500 uppercase tracking-wider">{label}</p>
      {children}
    </div>
  );
}

function AgentRunHistory({ runs }: { runs: AgentRun[] }) {
  const [expanded, setExpanded] = useState<string | null>(null);

  return (
    <section className="space-y-3">
      <h2 className="text-xs font-medium text-zinc-500 uppercase tracking-wider flex items-center gap-1.5">
        <Zap className="h-3.5 w-3.5 text-amber-400" />
        Agent run history
        <span className="rounded-full bg-zinc-800 px-1.5 py-0.5 text-[10px] font-bold text-zinc-400">
          {runs.length}
        </span>
      </h2>
      <div className="space-y-2">
        {runs.map((run) => (
          <div
            key={run.id}
            className="rounded-lg border border-zinc-800 bg-zinc-900/30 overflow-hidden"
          >
            <button
              className="w-full flex items-center gap-3 px-4 py-3 text-left hover:bg-zinc-900/60 transition-colors"
              onClick={() => setExpanded(expanded === run.id ? null : run.id)}
            >
              {run.error ? (
                <span className="text-[10px] font-semibold rounded-full px-2 py-0.5 bg-red-950 text-red-400">
                  error
                </span>
              ) : (
                <span className="text-[10px] font-semibold rounded-full px-2 py-0.5 bg-green-950 text-green-400">
                  ok
                </span>
              )}
              <span className="text-xs text-zinc-400">
                {new Date(run.created_at).toLocaleString()}
              </span>
              {expanded === run.id ? (
                <ChevronDown className="h-3.5 w-3.5 text-zinc-600 ml-auto" />
              ) : (
                <ChevronRight className="h-3.5 w-3.5 text-zinc-600 ml-auto" />
              )}
            </button>
            {expanded === run.id && (
              <div className="border-t border-zinc-800 px-4 py-3 space-y-3">
                {run.error && (
                  <div className="rounded-lg border border-red-900 bg-red-950/30 px-3 py-2">
                    <p className="text-xs text-red-400 font-mono whitespace-pre-wrap">
                      {run.error}
                    </p>
                  </div>
                )}
                {run.raw_response && (
                  <div>
                    <p className="text-[10px] font-medium text-zinc-600 uppercase tracking-wider mb-1.5">
                      Raw LLM response
                    </p>
                    <pre className="text-xs text-zinc-400 font-mono bg-zinc-950 rounded-lg p-3 overflow-auto max-h-64 whitespace-pre-wrap">
                      {run.raw_response}
                    </pre>
                  </div>
                )}
                {Array.isArray(run.actions_applied) &&
                  run.actions_applied.length > 0 && (
                    <div>
                      <p className="text-[10px] font-medium text-zinc-600 uppercase tracking-wider mb-1.5">
                        Actions applied
                      </p>
                      <pre className="text-xs text-zinc-400 font-mono bg-zinc-950 rounded-lg p-3 overflow-auto max-h-40 whitespace-pre-wrap">
                        {JSON.stringify(run.actions_applied, null, 2)}
                      </pre>
                    </div>
                  )}
              </div>
            )}
          </div>
        ))}
      </div>
    </section>
  );
}
