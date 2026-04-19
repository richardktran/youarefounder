"use client";

import { useParams, useRouter } from "next/navigation";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useState, useEffect } from "react";
import {
  getCompany,
  listProducts,
  listProductBrainEntries,
  listProductBrainPending,
  approveProductBrainPending,
  rejectProductBrainPending,
  resetInstall,
  RESET_INSTALL_CONFIRM_PHRASE,
  updateCompany,
  updateProduct,
} from "@/lib/api";
import { AlertTriangle, Brain, Library, Zap } from "lucide-react";
import { Spinner } from "@/components/ui/spinner";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Button } from "@/components/ui/button";
import { Card, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import type { ProductStatus } from "@/lib/api";

const STATUS_OPTIONS: { value: ProductStatus; label: string }[] = [
  { value: "idea", label: "Idea" },
  { value: "discovery", label: "Discovery" },
  { value: "spec", label: "Spec" },
  { value: "building", label: "Building" },
  { value: "launched", label: "Launched" },
];

export default function SettingsPage() {
  const params = useParams<{ companyId: string }>();
  const companyId = params.companyId;
  const router = useRouter();
  const qc = useQueryClient();

  const [showWarningZone, setShowWarningZone] = useState(false);
  const [resetConfirm, setResetConfirm] = useState("");

  const resetInstallMutation = useMutation({
    mutationFn: () => resetInstall(resetConfirm),
    onSuccess: () => {
      qc.clear();
      router.replace("/");
    },
  });

  const { data: company, isLoading: companyLoading } = useQuery({
    queryKey: ["company", companyId],
    queryFn: () => getCompany(companyId),
  });

  const { data: products, isLoading: productsLoading } = useQuery({
    queryKey: ["products", companyId],
    queryFn: () => listProducts(companyId),
  });

  const { data: brainPending = [], isLoading: brainPendingLoading } = useQuery({
    queryKey: ["product-brain-pending", companyId],
    queryFn: () => listProductBrainPending(companyId),
  });

  const { data: brainEntries = [], isLoading: brainEntriesLoading } = useQuery({
    queryKey: ["product-brain-entries", companyId],
    queryFn: () => listProductBrainEntries(companyId),
  });

  const approveBrainMutation = useMutation({
    mutationFn: (pendingId: string) => approveProductBrainPending(companyId, pendingId),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["product-brain-pending", companyId] });
      qc.invalidateQueries({ queryKey: ["product-brain-entries", companyId] });
    },
  });

  const rejectBrainMutation = useMutation({
    mutationFn: (pendingId: string) => rejectProductBrainPending(companyId, pendingId),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["product-brain-pending", companyId] });
    },
  });

  const [companyName, setCompanyName] = useState<string>("");
  const [savedCompany, setSavedCompany] = useState(false);
  const [maxConcurrent, setMaxConcurrent] = useState<number>(1);
  const [savedConcurrency, setSavedConcurrency] = useState(false);
  const [ticketMemory, setTicketMemory] = useState("");
  const [decisionMemory, setDecisionMemory] = useState("");
  const [savedMemory, setSavedMemory] = useState(false);

  useEffect(() => {
    if (!company) return;
    setTicketMemory(company.agent_ticket_memory ?? "");
    setDecisionMemory(company.agent_decision_memory ?? "");
  }, [company?.id, company?.agent_ticket_memory, company?.agent_decision_memory]);

  const updateCompanyMutation = useMutation({
    mutationFn: () => updateCompany(companyId, { name: companyName }),
    onSuccess: (updated) => {
      qc.setQueryData(["company", companyId], updated);
      setSavedCompany(true);
      setTimeout(() => setSavedCompany(false), 2000);
    },
  });

  const updateConcurrencyMutation = useMutation({
    mutationFn: () => updateCompany(companyId, { max_concurrent_agents: maxConcurrent }),
    onSuccess: (updated) => {
      qc.setQueryData(["company", companyId], updated);
      setSavedConcurrency(true);
      setTimeout(() => setSavedConcurrency(false), 2000);
    },
  });

  const updateMemoryMutation = useMutation({
    mutationFn: () =>
      updateCompany(companyId, {
        agent_ticket_memory: ticketMemory,
        agent_decision_memory: decisionMemory,
      }),
    onSuccess: (updated) => {
      qc.setQueryData(["company", companyId], updated);
      setSavedMemory(true);
      setTimeout(() => setSavedMemory(false), 2000);
    },
  });

  // Sync local state with fetched company
  if (company && companyName === "") setCompanyName(company.name);
  if (company && maxConcurrent === 1 && company.max_concurrent_agents !== 1) {
    setMaxConcurrent(company.max_concurrent_agents);
  }

  const firstProduct = products?.[0];

  return (
    <div className="p-8 space-y-8 max-w-2xl">
      <div>
        <h1 className="text-2xl font-bold text-white">Settings</h1>
        <p className="text-zinc-400 mt-1">Manage your company and product.</p>
      </div>

      {/* Company settings */}
      <Card>
        <CardHeader>
          <CardTitle>Company</CardTitle>
          <CardDescription>Basic details about your company.</CardDescription>
        </CardHeader>
        {companyLoading ? (
          <Spinner />
        ) : (
          <div className="space-y-4">
            <Input
              label="Company name"
              value={companyName}
              onChange={(e) => setCompanyName(e.target.value)}
            />
            <div className="flex items-center gap-3">
              <Button
                onClick={() => updateCompanyMutation.mutate()}
                isLoading={updateCompanyMutation.isPending}
                size="sm"
              >
                {savedCompany ? "Saved!" : "Save changes"}
              </Button>
            </div>
          </div>
        )}
      </Card>

      {/* Founder memory for agents */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Brain className="h-4 w-4 text-violet-400" />
            Agent memory
          </CardTitle>
          <CardDescription>
            Persistent instructions injected into every agent run (priorities, tone, how the
            team should execute). The team runs autonomously after onboarding — there is no
            founder decision inbox.
          </CardDescription>
        </CardHeader>
        {companyLoading ? (
          <Spinner />
        ) : (
          <div className="space-y-4">
            <Textarea
              label="Tickets & execution"
              value={ticketMemory}
              onChange={(e) => setTicketMemory(e.target.value)}
              rows={5}
              placeholder="- Prefer subtasks over new top-level tickets&#10;- Always delegate X to the CTO agent&#10;- ..."
            />
            <Textarea
              label="Team judgment and style"
              value={decisionMemory}
              onChange={(e) => setDecisionMemory(e.target.value)}
              rows={5}
              placeholder="- Prefer shipping small iterations&#10;- When unsure, pick a default and document it in comments&#10;- ..."
            />
            <Button
              size="sm"
              onClick={() => updateMemoryMutation.mutate()}
              isLoading={updateMemoryMutation.isPending}
            >
              {savedMemory ? "Saved!" : "Save memory"}
            </Button>
          </div>
        )}
      </Card>

      {/* Product brain (auto-promoted + optional manual review) */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Library className="h-4 w-4 text-sky-400" />
            Product brain
          </CardTitle>
          <CardDescription>
            Completed tickets and agent insights are promoted into persistent knowledge
            automatically. You can still approve or reject any rare pending drafts below.
          </CardDescription>
        </CardHeader>
        {brainPendingLoading || brainEntriesLoading ? (
          <Spinner />
        ) : (
          <div className="space-y-6">
            <div>
              <h3 className="text-sm font-medium text-zinc-300 mb-2">Pending review</h3>
              {brainPending.filter((p) => p.status === "pending").length === 0 ? (
                <p className="text-sm text-zinc-600">No pending items.</p>
              ) : (
                <ul className="space-y-3">
                  {brainPending
                    .filter((p) => p.status === "pending")
                    .map((p) => (
                      <li
                        key={p.id}
                        className="rounded-lg border border-zinc-800 bg-zinc-900/40 p-3 space-y-2"
                      >
                        <p className="text-xs text-zinc-500">
                          {p.source_ticket_id
                            ? `From ticket ${p.source_ticket_id}`
                            : "Proposed insight"}
                          {" · "}
                          {new Date(p.proposed_at).toLocaleString()}
                        </p>
                        <pre className="text-xs text-zinc-300 whitespace-pre-wrap max-h-40 overflow-y-auto font-mono">
                          {p.body}
                        </pre>
                        <div className="flex gap-2">
                          <Button
                            size="sm"
                            isLoading={approveBrainMutation.isPending}
                            onClick={() => approveBrainMutation.mutate(p.id)}
                          >
                            Approve
                          </Button>
                          <Button
                            size="sm"
                            variant="ghost"
                            isLoading={rejectBrainMutation.isPending}
                            onClick={() => rejectBrainMutation.mutate(p.id)}
                          >
                            Reject
                          </Button>
                        </div>
                      </li>
                    ))}
                </ul>
              )}
            </div>
            <div>
              <h3 className="text-sm font-medium text-zinc-300 mb-2">Approved (recent)</h3>
              {brainEntries.length === 0 ? (
                <p className="text-sm text-zinc-600">Nothing approved yet.</p>
              ) : (
                <ul className="space-y-2 max-h-64 overflow-y-auto">
                  {brainEntries.slice(0, 25).map((e) => (
                    <li
                      key={e.id}
                      className="rounded border border-zinc-800/80 bg-zinc-950/50 px-3 py-2 text-xs text-zinc-400"
                    >
                      <pre className="whitespace-pre-wrap font-mono text-zinc-300">{e.body}</pre>
                      <p className="mt-1 text-[10px] text-zinc-600">
                        {new Date(e.created_at).toLocaleString()}
                        {e.workspace_id ? ` · workspace ${e.workspace_id}` : " · company-wide"}
                      </p>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </div>
        )}
      </Card>

      {/* Agent concurrency */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Zap className="h-4 w-4 text-amber-400" />
            Agent concurrency
          </CardTitle>
          <CardDescription>
            How many agent jobs can run at the same time. Increase this to let
            multiple tickets be worked on in parallel. Changes take effect
            immediately — no restart required.
          </CardDescription>
        </CardHeader>
        {companyLoading ? (
          <Spinner />
        ) : (
          <div className="space-y-4">
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <label className="text-sm font-medium text-zinc-300">
                  Max concurrent agents
                </label>
                <span className="text-sm font-semibold text-white w-6 text-center">
                  {maxConcurrent}
                </span>
              </div>
              <input
                type="range"
                min={1}
                max={10}
                step={1}
                value={maxConcurrent}
                onChange={(e) => setMaxConcurrent(Number(e.target.value))}
                className="w-full accent-amber-400 cursor-pointer"
              />
              <div className="flex justify-between text-[10px] text-zinc-600">
                <span>1 (sequential)</span>
                <span>10 (max)</span>
              </div>
            </div>
            <Button
              size="sm"
              onClick={() => updateConcurrencyMutation.mutate()}
              isLoading={updateConcurrencyMutation.isPending}
            >
              {savedConcurrency ? "Saved!" : "Save"}
            </Button>
          </div>
        )}
      </Card>

      {/* Product settings */}
      {firstProduct && (
        <ProductSettingsCard
          companyId={companyId}
          product={firstProduct}
          onUpdated={(updated) => {
            qc.setQueryData(["products", companyId], [updated]);
          }}
        />
      )}

      {/* Data directory info */}
      <Card>
        <CardHeader>
          <CardTitle>Data storage</CardTitle>
          <CardDescription>
            Your company data is stored locally by the embedded PostgreSQL
            database managed by this app.
          </CardDescription>
        </CardHeader>
        <div className="text-sm text-zinc-500 space-y-1">
          <p>macOS: ~/Library/Application Support/youarefounder/</p>
          <p>Linux: ~/.local/share/youarefounder/</p>
          <p className="text-zinc-600 text-xs pt-2">
            Backup: copy this directory or use pg_dump on the embedded instance.
          </p>
        </div>
      </Card>

      {/* Full reset — all companies, back to onboarding */}
      <div className="rounded-xl border border-red-900/40 bg-red-950/10 overflow-hidden">
        <div className="p-6 space-y-4">
          <div className="flex items-start gap-3">
            <AlertTriangle className="h-5 w-5 text-red-400 shrink-0 mt-0.5" />
            <div>
              <h2 className="text-lg font-semibold text-red-300">Warning zone</h2>
              <p className="text-sm text-red-200/80 mt-1">
                Reset this install to a fresh state: every company and all local
                data (products, team, workspaces, tickets, jobs) is permanently
                removed. You will return to the onboarding wizard. This is not
                the same as pausing the simulation — there is no undo.
              </p>
            </div>
          </div>
          {!showWarningZone ? (
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="border-red-800 text-red-300 hover:bg-red-950/50 hover:text-red-200"
              onClick={() => setShowWarningZone(true)}
            >
              Show reset controls
            </Button>
          ) : (
            <div className="space-y-3 rounded-lg border border-red-900/50 bg-zinc-950/80 p-4">
              <p className="text-xs text-red-400">
                Type{" "}
                <span className="font-mono font-semibold text-red-200">
                  {RESET_INSTALL_CONFIRM_PHRASE}
                </span>{" "}
                exactly to confirm.
              </p>
              <div className="flex flex-col gap-2 sm:flex-row sm:items-center">
                <input
                  type="text"
                  value={resetConfirm}
                  onChange={(e) => setResetConfirm(e.target.value)}
                  placeholder={RESET_INSTALL_CONFIRM_PHRASE}
                  autoComplete="off"
                  className="flex-1 rounded-lg border border-red-900 bg-zinc-950 px-3 py-2 text-sm text-white placeholder-zinc-600 focus:outline-none focus:ring-1 focus:ring-red-700"
                />
                <div className="flex gap-2 shrink-0">
                  <Button
                    type="button"
                    size="sm"
                    isLoading={resetInstallMutation.isPending}
                    disabled={resetConfirm !== RESET_INSTALL_CONFIRM_PHRASE}
                    onClick={() => resetInstallMutation.mutate()}
                    className="bg-red-700 hover:bg-red-600 text-white disabled:opacity-40"
                  >
                    Reset install
                  </Button>
                  <Button
                    type="button"
                    size="sm"
                    variant="ghost"
                    onClick={() => {
                      setShowWarningZone(false);
                      setResetConfirm("");
                    }}
                  >
                    Cancel
                  </Button>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function ProductSettingsCard({
  companyId,
  product,
  onUpdated,
}: {
  companyId: string;
  product: { id: string; name: string; description: string | null; status: ProductStatus };
  onUpdated: (p: ReturnType<typeof Object.assign>) => void;
}) {
  const [name, setName] = useState(product.name);
  const [description, setDescription] = useState(product.description ?? "");
  const [status, setStatus] = useState<ProductStatus>(product.status);
  const [saved, setSaved] = useState(false);

  const mutation = useMutation({
    mutationFn: () =>
      updateProduct(companyId, product.id, {
        name,
        description: description || undefined,
        status,
      }),
    onSuccess: (updated) => {
      onUpdated(updated);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    },
  });

  return (
    <Card>
      <CardHeader>
        <CardTitle>Product</CardTitle>
        <CardDescription>
          Your flagship product — the mission your AI team executes against.
        </CardDescription>
      </CardHeader>
      <div className="space-y-4">
        <Input
          label="Product name"
          value={name}
          onChange={(e) => setName(e.target.value)}
        />
        <Textarea
          label="Description"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          rows={4}
        />
        <div className="space-y-1.5">
          <label className="block text-sm font-medium text-zinc-300">
            Status
          </label>
          <select
            value={status}
            onChange={(e) => setStatus(e.target.value as ProductStatus)}
            className="flex h-10 w-full rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-white focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400"
          >
            {STATUS_OPTIONS.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>
        </div>
        <Button
          onClick={() => mutation.mutate()}
          isLoading={mutation.isPending}
          size="sm"
        >
          {saved ? "Saved!" : "Save changes"}
        </Button>
      </div>
    </Card>
  );
}
