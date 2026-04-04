"use client";

import { useState } from "react";
import { useParams } from "next/navigation";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { FolderKanban, Plus, Ticket, ChevronRight } from "lucide-react";
import Link from "next/link";
import {
  listWorkspaces,
  createWorkspace,
  listTickets,
  type Workspace,
} from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Card } from "@/components/ui/card";
import { Spinner } from "@/components/ui/spinner";

export default function WorkspacesPage() {
  const params = useParams<{ companyId: string }>();
  const companyId = params.companyId;
  const queryClient = useQueryClient();

  const [showForm, setShowForm] = useState(false);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");

  const { data: workspaces, isLoading } = useQuery({
    queryKey: ["workspaces", companyId],
    queryFn: () => listWorkspaces(companyId),
  });

  const createMutation = useMutation({
    mutationFn: () =>
      createWorkspace(companyId, { name: name.trim(), description: description.trim() || undefined }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["workspaces", companyId] });
      setShowForm(false);
      setName("");
      setDescription("");
    },
  });

  return (
    <div className="p-8 max-w-4xl space-y-6">
      {/* Header */}
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-2xl font-bold text-white">Workspaces</h1>
          <p className="text-zinc-400 mt-1">
            Project areas for your company and AI team.
          </p>
        </div>
        <Button
          size="sm"
          onClick={() => setShowForm((v) => !v)}
        >
          <Plus className="h-4 w-4" />
          New workspace
        </Button>
      </div>

      {/* Create form */}
      {showForm && (
        <Card className="space-y-4">
          <h2 className="text-sm font-semibold text-white">New workspace</h2>
          <div className="space-y-3">
            <Input
              placeholder="Workspace name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              autoFocus
            />
            <Textarea
              placeholder="Description (optional)"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              rows={2}
            />
          </div>
          <div className="flex gap-2 justify-end">
            <Button
              variant="ghost"
              size="sm"
              onClick={() => {
                setShowForm(false);
                setName("");
                setDescription("");
              }}
            >
              Cancel
            </Button>
            <Button
              size="sm"
              isLoading={createMutation.isPending}
              disabled={!name.trim()}
              onClick={() => createMutation.mutate()}
            >
              Create
            </Button>
          </div>
        </Card>
      )}

      {/* Workspace list */}
      {isLoading ? (
        <div className="flex justify-center py-12">
          <Spinner />
        </div>
      ) : !workspaces?.length ? (
        <Card className="text-center py-12">
          <FolderKanban className="h-10 w-10 text-zinc-700 mx-auto mb-3" />
          <p className="text-zinc-400 text-sm">No workspaces yet.</p>
          <p className="text-zinc-600 text-xs mt-1">
            Create one above to start organising your work.
          </p>
        </Card>
      ) : (
        <div className="space-y-2">
          {workspaces.map((ws) => (
            <WorkspaceRow
              key={ws.id}
              workspace={ws}
              companyId={companyId}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function WorkspaceRow({
  workspace,
  companyId,
}: {
  workspace: Workspace;
  companyId: string;
}) {
  const { data: tickets } = useQuery({
    queryKey: ["tickets", companyId, workspace.id],
    queryFn: () => listTickets(companyId, workspace.id),
  });

  const total = tickets?.length ?? 0;
  const open = tickets?.filter(
    (t) => t.status !== "done" && t.status !== "cancelled"
  ).length ?? 0;

  return (
    <Link href={`/app/${companyId}/workspaces/${workspace.id}`}>
      <Card className="group flex items-center justify-between gap-4 hover:border-zinc-700 cursor-pointer transition-colors">
        <div className="flex items-center gap-3 min-w-0">
          <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-zinc-800">
            <FolderKanban className="h-4 w-4 text-zinc-400" />
          </div>
          <div className="min-w-0">
            <p className="font-medium text-white truncate">{workspace.name}</p>
            {workspace.description && (
              <p className="text-xs text-zinc-500 truncate">
                {workspace.description}
              </p>
            )}
          </div>
        </div>
        <div className="flex items-center gap-4 shrink-0">
          <div className="flex items-center gap-1.5 text-xs text-zinc-500">
            <Ticket className="h-3.5 w-3.5" />
            <span>
              {open} open · {total} total
            </span>
          </div>
          <ChevronRight className="h-4 w-4 text-zinc-700 group-hover:text-zinc-500 transition-colors" />
        </div>
      </Card>
    </Link>
  );
}
