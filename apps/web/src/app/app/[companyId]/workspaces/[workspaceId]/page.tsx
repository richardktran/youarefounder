"use client";

import { useState, useCallback } from "react";
import { useParams } from "next/navigation";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  DndContext,
  DragOverlay,
  PointerSensor,
  useSensor,
  useSensors,
  type DragStartEvent,
  type DragEndEvent,
  useDroppable,
} from "@dnd-kit/core";
import { useDraggable } from "@dnd-kit/core";
import {
  ArrowLeft,
  Plus,
  Users,
  Bot,
  User,
  X,
  CheckCheck,
  GripVertical,
} from "lucide-react";
import Link from "next/link";
import {
  getWorkspace,
  listTickets,
  createTicket,
  updateTicket,
  listPeople,
  listWorkspaceMembers,
  addWorkspaceMember,
  removeWorkspaceMember,
  type Ticket,
  type TicketStatus,
  type TicketPriority,
  type WorkspaceMember,
  type Person,
} from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/spinner";
import { cn } from "@/lib/utils";

const STATUS_CONFIG: Record<
  TicketStatus,
  { label: string; color: string; dot: string; header: string; count_bg: string }
> = {
  backlog: {
    label: "Backlog",
    color: "text-zinc-400",
    dot: "bg-zinc-500",
    header: "border-zinc-700",
    count_bg: "bg-zinc-800 text-zinc-400",
  },
  todo: {
    label: "Todo",
    color: "text-blue-400",
    dot: "bg-blue-500",
    header: "border-blue-700",
    count_bg: "bg-blue-950 text-blue-400",
  },
  in_progress: {
    label: "In Progress",
    color: "text-amber-400",
    dot: "bg-amber-500",
    header: "border-amber-600",
    count_bg: "bg-amber-950 text-amber-400",
  },
  blocked: {
    label: "Blocked",
    color: "text-red-400",
    dot: "bg-red-500",
    header: "border-red-700",
    count_bg: "bg-red-950 text-red-400",
  },
  done: {
    label: "Done",
    color: "text-green-400",
    dot: "bg-green-500",
    header: "border-green-700",
    count_bg: "bg-green-950 text-green-400",
  },
  cancelled: {
    label: "Cancelled",
    color: "text-zinc-500",
    dot: "bg-zinc-700",
    header: "border-zinc-800",
    count_bg: "bg-zinc-900 text-zinc-600",
  },
};

const PRIORITY_CONFIG: Record<TicketPriority, { label: string; color: string; bg: string }> = {
  low: { label: "Low", color: "text-zinc-500", bg: "bg-zinc-800" },
  medium: { label: "Med", color: "text-amber-500", bg: "bg-amber-950" },
  high: { label: "High", color: "text-red-500", bg: "bg-red-950" },
};

const STATUSES: TicketStatus[] = [
  "backlog",
  "todo",
  "in_progress",
  "blocked",
  "done",
  "cancelled",
];

// ─── Card visual (shared between board and DragOverlay) ──────────────────────

function TicketCardContent({
  ticket,
  companyId,
  workspaceId,
  people,
  dragHandleProps,
  className,
}: {
  ticket: Ticket;
  companyId: string;
  workspaceId: string;
  people: Person[];
  dragHandleProps?: React.HTMLAttributes<HTMLButtonElement>;
  className?: string;
}) {
  const priorityCfg = PRIORITY_CONFIG[ticket.priority];
  const assignee = people.find((p) => p.id === ticket.assignee_person_id);
  const isDone = ticket.status === "done";
  const isCancelled = ticket.status === "cancelled";

  return (
    <div className={cn("rounded-lg border bg-zinc-900 p-3", className)}>
      <div className="flex items-start gap-2">
        {/* Grip handle — drag starts here only */}
        <button
          {...dragHandleProps}
          className="mt-0.5 shrink-0 cursor-grab active:cursor-grabbing text-zinc-700 hover:text-zinc-500 transition-colors touch-none"
        >
          <GripVertical className="h-3.5 w-3.5" />
        </button>
        <Link
          href={`/app/${companyId}/workspaces/${workspaceId}/tickets/${ticket.id}`}
          className="flex-1 min-w-0"
        >
          <p
            className={cn(
              "text-sm font-medium leading-snug",
              isDone || isCancelled
                ? "text-zinc-600 line-through"
                : "text-white hover:text-zinc-200"
            )}
          >
            {ticket.title}
          </p>
        </Link>
      </div>
      <div className="mt-2.5 flex items-center justify-between gap-2 pl-5">
        <span
          className={cn(
            "inline-flex items-center rounded px-1.5 py-0.5 text-[11px] font-medium",
            priorityCfg.bg,
            priorityCfg.color
          )}
        >
          {priorityCfg.label}
        </span>
        {assignee ? (
          <div
            className="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-zinc-700 ring-1 ring-zinc-600"
            title={assignee.display_name}
          >
            {assignee.kind === "ai_agent" ? (
              <Bot className="h-3 w-3 text-blue-400" />
            ) : (
              <span className="text-[9px] font-bold text-zinc-300 uppercase">
                {assignee.display_name.slice(0, 2)}
              </span>
            )}
          </div>
        ) : (
          <div className="h-5 w-5 rounded-full border border-dashed border-zinc-700" title="Unassigned" />
        )}
      </div>
    </div>
  );
}

// ─── Draggable Ticket Card ────────────────────────────────────────────────────

function TicketCard({
  ticket,
  companyId,
  workspaceId,
  people,
}: {
  ticket: Ticket;
  companyId: string;
  workspaceId: string;
  people: Person[];
}) {
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({
    id: ticket.id,
    data: { ticket },
  });

  return (
    <div ref={setNodeRef} className={cn("transition-opacity", isDragging && "opacity-30")}>
      <TicketCardContent
        ticket={ticket}
        companyId={companyId}
        workspaceId={workspaceId}
        people={people}
        dragHandleProps={{ ...listeners, ...attributes } as React.HTMLAttributes<HTMLButtonElement>}
        className="border-zinc-800 hover:border-zinc-600 transition-colors"
      />
    </div>
  );
}

// ─── Droppable Column ─────────────────────────────────────────────────────────

function KanbanColumn({
  status,
  tickets,
  companyId,
  workspaceId,
  people,
  onCreateTicket,
  isCreating,
}: {
  status: TicketStatus;
  tickets: Ticket[];
  companyId: string;
  workspaceId: string;
  people: Person[];
  onCreateTicket: (status: TicketStatus, title: string) => void;
  isCreating: boolean;
}) {
  const { setNodeRef, isOver } = useDroppable({ id: status });
  const cfg = STATUS_CONFIG[status];
  const [showAdd, setShowAdd] = useState(false);
  const [newTitle, setNewTitle] = useState("");

  function handleCreate() {
    if (!newTitle.trim()) return;
    onCreateTicket(status, newTitle.trim());
    setNewTitle("");
    setShowAdd(false);
  }

  return (
    <div className="flex w-72 shrink-0 flex-col rounded-xl border border-zinc-800 bg-zinc-950/60">
      {/* Column header */}
      <div className={cn("flex items-center gap-2 border-b px-3 py-2.5", cfg.header)}>
        <div className={cn("h-2 w-2 rounded-full", cfg.dot)} />
        <span className={cn("flex-1 text-xs font-semibold uppercase tracking-wider", cfg.color)}>
          {cfg.label}
        </span>
        <span
          className={cn(
            "rounded-full px-1.5 py-0.5 text-[10px] font-bold tabular-nums",
            cfg.count_bg
          )}
        >
          {tickets.length}
        </span>
        <button
          onClick={() => { setShowAdd((v) => !v); setNewTitle(""); }}
          className="ml-1 rounded p-0.5 text-zinc-600 hover:text-zinc-300 hover:bg-zinc-800 transition-colors"
          title="Add ticket"
        >
          <Plus className="h-3.5 w-3.5" />
        </button>
      </div>

      {/* Cards */}
      <div
        ref={setNodeRef}
        className={cn(
          "flex flex-1 flex-col gap-2 p-2 min-h-[120px] transition-colors rounded-b-xl",
          isOver && "bg-zinc-800/30"
        )}
      >
        {tickets.map((ticket) => (
          <TicketCard
            key={ticket.id}
            ticket={ticket}
            companyId={companyId}
            workspaceId={workspaceId}
            people={people}
          />
        ))}

        {/* Inline create form */}
        {showAdd && (
          <div className="rounded-lg border border-zinc-700 bg-zinc-900 p-2 space-y-2">
            <Input
              placeholder="Ticket title"
              value={newTitle}
              onChange={(e) => setNewTitle(e.target.value)}
              autoFocus
              className="text-sm"
              onKeyDown={(e) => {
                if (e.key === "Enter") handleCreate();
                if (e.key === "Escape") { setShowAdd(false); setNewTitle(""); }
              }}
            />
            <div className="flex gap-1.5 justify-end">
              <Button
                variant="ghost"
                size="sm"
                onClick={() => { setShowAdd(false); setNewTitle(""); }}
              >
                Cancel
              </Button>
              <Button
                size="sm"
                disabled={!newTitle.trim() || isCreating}
                isLoading={isCreating}
                onClick={handleCreate}
              >
                Add
              </Button>
            </div>
          </div>
        )}

        {tickets.length === 0 && !showAdd && (
          <div className="flex flex-1 items-center justify-center py-8">
            <p className="text-xs text-zinc-700">Drop cards here</p>
          </div>
        )}
      </div>
    </div>
  );
}

// ─── Main Page ────────────────────────────────────────────────────────────────

export default function WorkspacePage() {
  const params = useParams<{ companyId: string; workspaceId: string }>();
  const { companyId, workspaceId } = params;
  const queryClient = useQueryClient();

  const [showMembersPanel, setShowMembersPanel] = useState(false);
  const [addingPersonId, setAddingPersonId] = useState("");
  const [activeTicket, setActiveTicket] = useState<Ticket | null>(null);
  const [creatingStatus, setCreatingStatus] = useState<TicketStatus | null>(null);

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } })
  );

  const { data: workspace, isLoading: wsLoading } = useQuery({
    queryKey: ["workspace", workspaceId],
    queryFn: () => getWorkspace(companyId, workspaceId),
  });

  const { data: tickets = [], isLoading: ticketsLoading } = useQuery({
    queryKey: ["tickets", companyId, workspaceId],
    queryFn: () => listTickets(companyId, workspaceId),
  });

  const { data: wsMembers = [] } = useQuery({
    queryKey: ["workspace-members", workspaceId],
    queryFn: () => listWorkspaceMembers(companyId, workspaceId),
  });

  const { data: allPeople = [] } = useQuery({
    queryKey: ["people", companyId],
    queryFn: () => listPeople(companyId),
  });

  const addMemberMutation = useMutation({
    mutationFn: (personId: string) =>
      addWorkspaceMember(companyId, workspaceId, { person_id: personId }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["workspace-members", workspaceId] });
      setAddingPersonId("");
    },
  });

  const removeMemberMutation = useMutation({
    mutationFn: (personId: string) =>
      removeWorkspaceMember(companyId, workspaceId, personId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["workspace-members", workspaceId] });
    },
  });

  const createMutation = useMutation({
    mutationFn: ({ title, status }: { title: string; status: TicketStatus }) =>
      createTicket(companyId, workspaceId, { title, status }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["tickets", companyId, workspaceId] });
      setCreatingStatus(null);
    },
  });

  const statusMutation = useMutation({
    mutationFn: ({ ticketId, status }: { ticketId: string; status: TicketStatus }) =>
      updateTicket(companyId, workspaceId, ticketId, { status }),
    onMutate: async ({ ticketId, status }) => {
      await queryClient.cancelQueries({ queryKey: ["tickets", companyId, workspaceId] });
      await queryClient.cancelQueries({ queryKey: ["ticket", ticketId] });
      const prevList = queryClient.getQueryData<Ticket[]>(["tickets", companyId, workspaceId]);
      const prevSingle = queryClient.getQueryData<Ticket>(["ticket", ticketId]);
      // Optimistically update both caches so the detail page sees the new status immediately
      queryClient.setQueryData<Ticket[]>(
        ["tickets", companyId, workspaceId],
        (old) => old?.map((t) => t.id === ticketId ? { ...t, status } : t) ?? []
      );
      queryClient.setQueryData<Ticket>(
        ["ticket", ticketId],
        (old) => old ? { ...old, status } : old
      );
      return { prevList, prevSingle };
    },
    onError: (_err, { ticketId }, context) => {
      if (context?.prevList) {
        queryClient.setQueryData(["tickets", companyId, workspaceId], context.prevList);
      }
      if (context?.prevSingle) {
        queryClient.setQueryData(["ticket", ticketId], context.prevSingle);
      }
    },
    onSettled: (_data, _err, { ticketId }) => {
      queryClient.invalidateQueries({ queryKey: ["tickets", companyId, workspaceId] });
      queryClient.invalidateQueries({ queryKey: ["ticket", ticketId] });
    },
  });

  const grouped = STATUSES.reduce<Record<TicketStatus, Ticket[]>>(
    (acc, s) => { acc[s] = tickets.filter((t) => t.status === s); return acc; },
    {} as Record<TicketStatus, Ticket[]>
  );

  const handleDragStart = useCallback((event: DragStartEvent) => {
    const ticket = event.active.data.current?.ticket as Ticket;
    setActiveTicket(ticket ?? null);
  }, []);

  const handleDragEnd = useCallback((event: DragEndEvent) => {
    setActiveTicket(null);
    const { active, over } = event;
    if (!over) return;
    const ticketId = active.id as string;
    const newStatus = over.id as TicketStatus;
    const ticket = tickets.find((t) => t.id === ticketId);
    if (ticket && ticket.status !== newStatus && STATUSES.includes(newStatus)) {
      statusMutation.mutate({ ticketId, status: newStatus });
    }
  }, [tickets, statusMutation]);

  const handleCreateTicket = useCallback((status: TicketStatus, title: string) => {
    setCreatingStatus(status);
    createMutation.mutate({ title, status });
  }, [createMutation]);

  if (wsLoading) {
    return (
      <div className="flex h-full items-center justify-center p-12">
        <Spinner />
      </div>
    );
  }

  const doneCount = grouped.done.length;
  const totalCount = tickets.length;

  return (
    <div className="flex h-full flex-col overflow-hidden">
      {/* Header */}
      <div className="shrink-0 border-b border-zinc-800 px-6 py-4">
        <Link
          href={`/app/${companyId}/workspaces`}
          className="inline-flex items-center gap-1.5 text-xs text-zinc-500 hover:text-zinc-300 transition-colors mb-2"
        >
          <ArrowLeft className="h-3 w-3" />
          Workspaces
        </Link>
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div>
              <h1 className="text-lg font-bold text-white">{workspace?.name}</h1>
              {workspace?.description && (
                <p className="text-xs text-zinc-500 mt-0.5">{workspace.description}</p>
              )}
            </div>
            {totalCount > 0 && (
              <div className="flex items-center gap-1.5 rounded-full border border-zinc-800 bg-zinc-900 px-2.5 py-1 text-xs text-zinc-500">
                <CheckCheck className="h-3 w-3 text-green-500" />
                {doneCount} / {totalCount} done
              </div>
            )}
          </div>
          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => setShowMembersPanel((v) => !v)}
            >
              <Users className="h-3.5 w-3.5" />
              Members
              {wsMembers.length > 0 && (
                <span className="ml-0.5 text-xs text-zinc-500">{wsMembers.length}</span>
              )}
            </Button>
          </div>
        </div>

        {/* Members panel (inline below header) */}
        {showMembersPanel && (
          <div className="mt-3 rounded-xl border border-zinc-800 bg-zinc-900/60 p-4">
            <div className="flex items-center justify-between mb-3">
              <p className="text-xs font-semibold text-zinc-400 uppercase tracking-wider">Team Access</p>
              <button onClick={() => setShowMembersPanel(false)} className="text-zinc-600 hover:text-zinc-400">
                <X className="h-3.5 w-3.5" />
              </button>
            </div>
            <div className="flex flex-wrap gap-2 mb-3">
              {wsMembers.map((m: WorkspaceMember) => (
                <div
                  key={m.id}
                  className="flex items-center gap-1.5 rounded-full border border-zinc-700 bg-zinc-800 pl-1 pr-2 py-1"
                >
                  <div className="flex h-5 w-5 items-center justify-center rounded-full bg-zinc-700">
                    {m.person_kind === "ai_agent" ? (
                      <Bot className="h-2.5 w-2.5 text-blue-400" />
                    ) : (
                      <User className="h-2.5 w-2.5 text-zinc-400" />
                    )}
                  </div>
                  <span className="text-xs text-zinc-300">{m.display_name}</span>
                  <button
                    onClick={() => removeMemberMutation.mutate(m.person_id)}
                    className="text-zinc-600 hover:text-red-400 transition-colors"
                  >
                    <X className="h-3 w-3" />
                  </button>
                </div>
              ))}
              {wsMembers.length === 0 && (
                <p className="text-xs text-zinc-600">No members yet.</p>
              )}
            </div>
            {allPeople.filter((p) => !wsMembers.some((m: WorkspaceMember) => m.person_id === p.id)).length > 0 && (
              <div className="flex gap-2">
                <select
                  value={addingPersonId}
                  onChange={(e) => setAddingPersonId(e.target.value)}
                  className="flex-1 rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-1.5 text-xs text-white focus:outline-none focus:ring-1 focus:ring-zinc-500"
                >
                  <option value="">Add a person…</option>
                  {allPeople
                    .filter((p) => !wsMembers.some((m: WorkspaceMember) => m.person_id === p.id))
                    .map((p) => (
                      <option key={p.id} value={p.id}>
                        {p.display_name} ({p.role_type.replace("_", " ")})
                      </option>
                    ))}
                </select>
                <Button
                  size="sm"
                  disabled={!addingPersonId || addMemberMutation.isPending}
                  isLoading={addMemberMutation.isPending}
                  onClick={() => addingPersonId && addMemberMutation.mutate(addingPersonId)}
                >
                  Add
                </Button>
              </div>
            )}
          </div>
        )}
      </div>

      {/* Kanban board */}
      {ticketsLoading ? (
        <div className="flex flex-1 items-center justify-center">
          <Spinner />
        </div>
      ) : (
        <DndContext
          sensors={sensors}
          onDragStart={handleDragStart}
          onDragEnd={handleDragEnd}
        >
          <div className="flex flex-1 gap-3 overflow-x-auto p-4 pb-6">
            {STATUSES.map((status) => (
              <KanbanColumn
                key={status}
                status={status}
                tickets={grouped[status]}
                companyId={companyId}
                workspaceId={workspaceId}
                people={allPeople}
                onCreateTicket={handleCreateTicket}
                isCreating={creatingStatus === status && createMutation.isPending}
              />
            ))}
          </div>

          <DragOverlay>
            {activeTicket && (
              <TicketCardContent
                ticket={activeTicket}
                companyId={companyId}
                workspaceId={workspaceId}
                people={allPeople}
                className="shadow-2xl shadow-black/60 border-zinc-600 rotate-1 scale-105"
              />
            )}
          </DragOverlay>
        </DndContext>
      )}
    </div>
  );
}
