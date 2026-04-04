"use client";

import { useState } from "react";
import { useParams, useRouter } from "next/navigation";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  Package,
  Users,
  FolderKanban,
  Inbox,
  ArrowRight,
  Play,
  Pause,
  Trash2,
  Activity,
} from "lucide-react";
import Link from "next/link";
import {
  getCompany,
  listProducts,
  listWorkspaces,
  listPeople,
  listAgentJobs,
  runCompany,
  stopCompany,
  terminateCompany,
  type RunState,
  type AgentJob,
} from "@/lib/api";
import { Spinner } from "@/components/ui/spinner";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

const STATUS_LABELS: Record<string, string> = {
  idea: "Idea",
  discovery: "Discovery",
  spec: "Spec",
  building: "Building",
  launched: "Launched",
};

const STATUS_COLORS: Record<string, string> = {
  idea: "text-zinc-400 bg-zinc-800",
  discovery: "text-blue-400 bg-blue-950",
  spec: "text-purple-400 bg-purple-950",
  building: "text-amber-400 bg-amber-950",
  launched: "text-green-400 bg-green-950",
};

const JOB_STATUS_COLORS: Record<string, string> = {
  pending: "text-zinc-400 bg-zinc-800",
  running: "text-blue-400 bg-blue-950",
  succeeded: "text-green-400 bg-green-950",
  failed: "text-red-400 bg-red-950",
};

export default function CompanyOverview() {
  const params = useParams<{ companyId: string }>();
  const companyId = params.companyId;
  const router = useRouter();
  const queryClient = useQueryClient();

  const [terminateInput, setTerminateInput] = useState("");
  const [showTerminate, setShowTerminate] = useState(false);

  const { data: company, isLoading: companyLoading } = useQuery({
    queryKey: ["company", companyId],
    queryFn: () => getCompany(companyId),
  });

  const { data: products, isLoading: productsLoading } = useQuery({
    queryKey: ["products", companyId],
    queryFn: () => listProducts(companyId),
  });

  const { data: workspaces } = useQuery({
    queryKey: ["workspaces", companyId],
    queryFn: () => listWorkspaces(companyId),
  });

  const { data: people } = useQuery({
    queryKey: ["people", companyId],
    queryFn: () => listPeople(companyId),
  });

  const { data: agentJobs } = useQuery({
    queryKey: ["agent-jobs", companyId],
    queryFn: () => listAgentJobs(companyId),
    refetchInterval: 5000,
  });

  const invalidateCompany = () => {
    queryClient.invalidateQueries({ queryKey: ["company", companyId] });
    queryClient.invalidateQueries({ queryKey: ["agent-jobs", companyId] });
  };

  const runMutation = useMutation({
    mutationFn: () => runCompany(companyId),
    onSuccess: invalidateCompany,
  });

  const stopMutation = useMutation({
    mutationFn: () => stopCompany(companyId),
    onSuccess: invalidateCompany,
  });

  const terminateMutation = useMutation({
    mutationFn: () => terminateCompany(companyId, terminateInput),
    onSuccess: () => router.push("/"),
  });

  if (companyLoading) {
    return (
      <div className="flex h-full items-center justify-center p-12">
        <Spinner />
      </div>
    );
  }

  if (!company) return null;

  const isRunning = company.run_state === "running";
  const isStopped = company.run_state === "stopped";

  return (
    <div className="p-8 space-y-8 max-w-4xl">
      {/* Page header */}
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-2xl font-bold text-white">{company.name}</h1>
          <p className="text-zinc-400 mt-1">Company overview</p>
        </div>
        <SimulationBadge state={company.run_state} />
      </div>

      {/* Simulation controls */}
      <section className="rounded-xl border border-zinc-800/60 bg-zinc-900/30 p-5 space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-sm font-semibold text-white">
              Simulation
            </h2>
            <p className="text-xs text-zinc-500 mt-0.5">
              {isRunning
                ? "The business is running — agents are processing tickets."
                : "The business is paused — agents are idle."}
            </p>
          </div>
          <div className="flex items-center gap-2">
            {isStopped && (
              <Button
                size="sm"
                isLoading={runMutation.isPending}
                onClick={() => runMutation.mutate()}
                className="bg-green-600 hover:bg-green-500 text-white"
              >
                <Play className="h-3.5 w-3.5" />
                Run
              </Button>
            )}
            {isRunning && (
              <Button
                size="sm"
                variant="outline"
                isLoading={stopMutation.isPending}
                onClick={() => stopMutation.mutate()}
              >
                <Pause className="h-3.5 w-3.5" />
                Stop
              </Button>
            )}
            <Button
              size="sm"
              variant="ghost"
              className="text-red-500 hover:text-red-400 hover:bg-red-950/50"
              onClick={() => setShowTerminate((v) => !v)}
            >
              <Trash2 className="h-3.5 w-3.5" />
              Terminate
            </Button>
          </div>
        </div>

        {/* Terminate confirmation */}
        {showTerminate && (
          <div className="rounded-lg border border-red-900/50 bg-red-950/20 p-4 space-y-3">
            <p className="text-sm text-red-400 font-medium">
              This is irreversible. All company data, people, tickets, and jobs
              will be permanently deleted.
            </p>
            <p className="text-xs text-red-500">
              Type <span className="font-mono font-bold">{company.name}</span>{" "}
              to confirm:
            </p>
            <div className="flex gap-2">
              <input
                value={terminateInput}
                onChange={(e) => setTerminateInput(e.target.value)}
                placeholder={company.name}
                className="flex-1 rounded-lg border border-red-900 bg-zinc-950 px-3 py-2 text-sm text-white placeholder-zinc-700 focus:outline-none focus:ring-1 focus:ring-red-700"
              />
              <Button
                size="sm"
                isLoading={terminateMutation.isPending}
                disabled={terminateInput !== company.name}
                onClick={() => terminateMutation.mutate()}
                className="bg-red-700 hover:bg-red-600 text-white disabled:opacity-40"
              >
                Terminate
              </Button>
              <Button
                size="sm"
                variant="ghost"
                onClick={() => {
                  setShowTerminate(false);
                  setTerminateInput("");
                }}
              >
                Cancel
              </Button>
            </div>
          </div>
        )}
      </section>

      {/* Quick stats */}
      <div className="grid grid-cols-2 gap-4 sm:grid-cols-4">
        <StatCard
          icon={<Package className="h-5 w-5" />}
          label="Products"
          value={productsLoading ? "…" : String(products?.length ?? 0)}
          href={`/app/${companyId}/settings`}
        />
        <StatCard
          icon={<FolderKanban className="h-5 w-5" />}
          label="Workspaces"
          value={workspaces ? String(workspaces.length) : "…"}
          href={`/app/${companyId}/workspaces`}
        />
        <StatCard
          icon={<Users className="h-5 w-5" />}
          label="Team members"
          value={people ? String(people.length) : "…"}
          href={`/app/${companyId}/team`}
        />
        <StatCard
          icon={<Inbox className="h-5 w-5" />}
          label="Open decisions"
          value="0"
          href={`/app/${companyId}/inbox`}
          dimmed
        />
      </div>

      {/* Products */}
      <section className="space-y-3">
        <h2 className="text-sm font-medium text-zinc-400 uppercase tracking-wider">
          Products
        </h2>
        {productsLoading ? (
          <Spinner />
        ) : !products?.length ? (
          <Card>
            <p className="text-zinc-500 text-sm">No products yet.</p>
          </Card>
        ) : (
          <div className="space-y-2">
            {products.map((p) => (
              <Card
                key={p.id}
                className="flex items-start justify-between gap-4"
              >
                <div className="space-y-1 min-w-0">
                  <p className="font-medium text-white">{p.name}</p>
                  {p.description && (
                    <p className="text-sm text-zinc-400 line-clamp-2">
                      {p.description}
                    </p>
                  )}
                </div>
                <span
                  className={cn(
                    "shrink-0 text-xs font-medium px-2 py-0.5 rounded-full",
                    STATUS_COLORS[p.status] ?? STATUS_COLORS.idea
                  )}
                >
                  {STATUS_LABELS[p.status] ?? p.status}
                </span>
              </Card>
            ))}
          </div>
        )}
      </section>

      {/* Agent jobs */}
      <section className="space-y-3">
        <h2 className="text-sm font-medium text-zinc-400 uppercase tracking-wider flex items-center gap-1.5">
          <Activity className="h-4 w-4" />
          Recent agent runs
        </h2>
        {!agentJobs?.length ? (
          <Card>
            <p className="text-zinc-500 text-sm">
              No agent jobs yet.{" "}
              {isStopped
                ? "Start the simulation to begin processing tickets."
                : "Jobs will appear as the co-founder processes tickets."}
            </p>
          </Card>
        ) : (
          <div className="space-y-2">
            {agentJobs.slice(0, 10).map((job) => (
              <AgentJobRow key={job.id} job={job} />
            ))}
          </div>
        )}
      </section>
    </div>
  );
}

function SimulationBadge({ state }: { state: RunState }) {
  const cfg = {
    running: {
      dot: "bg-green-400 animate-pulse",
      text: "text-green-400",
      label: "Running",
    },
    stopped: {
      dot: "bg-zinc-500",
      text: "text-zinc-400",
      label: "Stopped",
    },
    terminated: {
      dot: "bg-red-500",
      text: "text-red-400",
      label: "Terminated",
    },
  }[state] ?? { dot: "bg-zinc-500", text: "text-zinc-400", label: state };

  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-full border border-zinc-800 bg-zinc-900 px-3 py-1 text-xs font-medium",
        cfg.text
      )}
    >
      <span className={cn("h-1.5 w-1.5 rounded-full", cfg.dot)} />
      {cfg.label}
    </span>
  );
}

function AgentJobRow({ job }: { job: AgentJob }) {
  const cfg = JOB_STATUS_COLORS[job.status] ?? "text-zinc-400 bg-zinc-800";
  const kindLabel =
    job.kind === "agent_ticket_run" ? "Ticket run" : job.kind;
  return (
    <div className="flex items-center gap-3 rounded-xl border border-zinc-800/60 bg-zinc-900/30 px-4 py-3">
      <span
        className={cn(
          "shrink-0 rounded-full px-2 py-0.5 text-[10px] font-semibold",
          cfg
        )}
      >
        {job.status}
      </span>
      <span className="text-sm text-zinc-300 font-medium">{kindLabel}</span>
      {job.error && (
        <span className="text-xs text-red-500 truncate ml-2 max-w-xs">
          {job.error}
        </span>
      )}
      <span className="ml-auto text-[10px] text-zinc-600 shrink-0">
        {new Date(job.created_at).toLocaleTimeString()}
      </span>
    </div>
  );
}

function StatCard({
  icon,
  label,
  value,
  href,
  dimmed,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
  href: string;
  dimmed?: boolean;
}) {
  return (
    <Link href={href}>
      <Card
        className={cn(
          "group cursor-pointer hover:border-zinc-700 transition-colors",
          dimmed && "opacity-50"
        )}
      >
        <div className="flex items-center justify-between mb-3">
          <span className="text-zinc-500">{icon}</span>
          <ArrowRight className="h-3.5 w-3.5 text-zinc-700 group-hover:text-zinc-500 transition-colors" />
        </div>
        <p className="text-2xl font-bold text-white">{value}</p>
        <p className="text-xs text-zinc-500 mt-0.5">{label}</p>
      </Card>
    </Link>
  );
}
