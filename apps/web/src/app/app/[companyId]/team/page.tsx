"use client";

import { useState } from "react";
import { useParams } from "next/navigation";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  Users,
  Plus,
  Bot,
  User,
  Trash2,
  ChevronDown,
  ChevronUp,
  Cpu,
  CheckCircle,
  XCircle,
  Loader2,
  GitBranch,
  ArrowRight,
  Pencil,
} from "lucide-react";
import {
  listPeople,
  createPerson,
  deletePerson,
  updatePerson,
  listAiProfiles,
  listAiProviders,
  createAiProfile,
  updateAiProfile,
  testConnection,
  getOrgChart,
  updateReportingLine,
  type Person,
  type PersonKind,
  type RoleType,
  type OrgNode,
  type AiProfile,
  type ProviderInfo,
} from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
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

const KIND_LABELS: Record<PersonKind, string> = {
  human_founder: "Human",
  ai_agent: "AI Agent",
};

type TeamTab = "members" | "org-chart";

export default function TeamPage() {
  const params = useParams<{ companyId: string }>();
  const companyId = params.companyId;
  const queryClient = useQueryClient();

  const [activeTab, setActiveTab] = useState<TeamTab>("members");

  const [showForm, setShowForm] = useState(false);
  const [kind, setKind] = useState<PersonKind>("human_founder");
  const [name, setName] = useState("");
  const [roleType, setRoleType] = useState<RoleType>("specialist");
  const [specialty, setSpecialty] = useState("");

  // AI agent — existing profile
  const [aiSetupMode, setAiSetupMode] = useState<"new" | "existing">("new");
  const [aiProfileId, setAiProfileId] = useState("");

  // AI agent — new profile setup
  const [selectedProviderKind, setSelectedProviderKind] = useState("ollama");
  const [providerConfig, setProviderConfig] = useState<Record<string, string>>({
    base_url: "http://127.0.0.1:11434",
  });
  const [newModelId, setNewModelId] = useState("gemma4:e2b");
  const [connStatus, setConnStatus] = useState<"idle" | "testing" | "ok" | "error">("idle");
  const [connError, setConnError] = useState<string | null>(null);

  const { data: people, isLoading } = useQuery({
    queryKey: ["people", companyId],
    queryFn: () => listPeople(companyId),
  });

  const { data: aiProfiles } = useQuery({
    queryKey: ["ai-profiles", companyId],
    queryFn: () => listAiProfiles(companyId),
  });

  const { data: providersData } = useQuery({
    queryKey: ["ai-providers"],
    queryFn: listAiProviders,
  });
  const providers = providersData?.providers ?? [];

  function handleSelectProvider(providerKind: string) {
    setSelectedProviderKind(providerKind);
    const provider = providers.find((p) => p.kind === providerKind);
    if (provider) {
      const defaults: Record<string, string> = {};
      provider.config_fields.forEach((f) => {
        defaults[f.key] = f.default_value ?? "";
      });
      setProviderConfig(defaults);
    } else {
      setProviderConfig({});
    }
    setConnStatus("idle");
    setConnError(null);
  }

  async function handleTestConnection() {
    setConnStatus("testing");
    setConnError(null);
    try {
      const result = await testConnection({
        provider_kind: selectedProviderKind,
        provider_config: {
          schema_version: 1,
          ...Object.fromEntries(
            Object.entries(providerConfig).map(([k, v]) => [k, v.trim()])
          ),
        },
        model_id: newModelId.trim() || undefined,
      });
      setConnStatus(result.ok ? "ok" : "error");
      if (!result.ok) setConnError(result.error ?? "Connection failed");
    } catch {
      setConnStatus("error");
      setConnError("Network error — is the API running?");
    }
  }

  const createMutation = useMutation({
    mutationFn: async () => {
      let profileId: string | undefined =
        kind === "ai_agent" && aiSetupMode === "existing" && aiProfileId
          ? aiProfileId
          : undefined;

      if (kind === "ai_agent" && aiSetupMode === "new" && newModelId.trim()) {
        const profile = await createAiProfile(companyId, {
          provider_kind: selectedProviderKind,
          model_id: newModelId.trim(),
          provider_config: {
            schema_version: 1,
            ...Object.fromEntries(
              Object.entries(providerConfig).map(([k, v]) => [k, v.trim()])
            ),
          },
          display_name: name.trim() || undefined,
        });
        profileId = profile.id;
      }

      return createPerson(companyId, {
        kind,
        display_name: name.trim(),
        role_type: roleType,
        specialty: specialty.trim() || undefined,
        ai_profile_id: profileId,
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["people", companyId] });
      queryClient.invalidateQueries({ queryKey: ["ai-profiles", companyId] });
      resetForm();
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (personId: string) => deletePerson(companyId, personId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["people", companyId] });
    },
  });

  function resetForm() {
    setShowForm(false);
    setKind("human_founder");
    setName("");
    setRoleType("specialist");
    setSpecialty("");
    setAiProfileId("");
    setAiSetupMode("new");
    setSelectedProviderKind("ollama");
    setProviderConfig({ base_url: "http://127.0.0.1:11434" });
    setNewModelId("gemma4:e2b");
    setConnStatus("idle");
    setConnError(null);
  }

  const humanMembers = people?.filter((p) => p.kind === "human_founder") ?? [];
  const aiAgents = people?.filter((p) => p.kind === "ai_agent") ?? [];

  return (
    <div className="p-8 max-w-4xl space-y-6">
      {/* Header */}
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-2xl font-bold text-white">Team</h1>
          <p className="text-zinc-400 mt-1">
            Your AI workforce — co-founders, executives, and specialists.
          </p>
        </div>
        {activeTab === "members" && (
          <Button size="sm" onClick={() => setShowForm((v) => !v)}>
            <Plus className="h-4 w-4" />
            Add member
          </Button>
        )}
      </div>

      {/* Tabs */}
      <div className="flex gap-1 border-b border-zinc-800">
        {([
          { key: "members", label: "Members", icon: <Users className="h-3.5 w-3.5" /> },
          { key: "org-chart", label: "Org chart", icon: <GitBranch className="h-3.5 w-3.5" /> },
        ] as { key: TeamTab; label: string; icon: React.ReactNode }[]).map((t) => (
          <button
            key={t.key}
            onClick={() => setActiveTab(t.key)}
            className={cn(
              "flex items-center gap-1.5 px-4 py-2 text-sm font-medium border-b-2 -mb-px transition-colors",
              activeTab === t.key
                ? "border-white text-white"
                : "border-transparent text-zinc-500 hover:text-zinc-300"
            )}
          >
            {t.icon}
            {t.label}
          </button>
        ))}
      </div>

      {/* Org chart tab */}
      {activeTab === "org-chart" && (
        <OrgChartView companyId={companyId} people={people ?? []} />
      )}

      {/* Members tab content */}
      {activeTab === "members" && <>

      {/* Add member form */}
      {showForm && (
        <Card className="space-y-4">
          <h2 className="text-sm font-semibold text-white">Add team member</h2>

          {/* Kind toggle */}
          <div className="flex gap-2">
            {(["human_founder", "ai_agent"] as PersonKind[]).map((k) => (
              <button
                key={k}
                onClick={() => setKind(k)}
                className={cn(
                  "flex items-center gap-2 rounded-lg px-3 py-1.5 text-sm font-medium transition-colors border",
                  kind === k
                    ? "bg-zinc-800 border-zinc-600 text-white"
                    : "border-zinc-800 text-zinc-500 hover:text-zinc-300 hover:border-zinc-700"
                )}
              >
                {k === "human_founder" ? (
                  <User className="h-3.5 w-3.5" />
                ) : (
                  <Bot className="h-3.5 w-3.5" />
                )}
                {KIND_LABELS[k]}
              </button>
            ))}
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div className="col-span-2">
              <Input
                placeholder="Display name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                autoFocus
              />
            </div>

            {/* Role type */}
            <div>
              <label className="text-xs text-zinc-500 mb-1.5 block">Role</label>
              <select
                value={roleType}
                onChange={(e) => setRoleType(e.target.value as RoleType)}
                className="w-full rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-white focus:outline-none focus:ring-2 focus:ring-zinc-400"
              >
                <option value="co_founder">Co-founder</option>
                <option value="ceo">CEO</option>
                <option value="cto">CTO</option>
                <option value="specialist">Specialist</option>
              </select>
            </div>

            <div>
              <label className="text-xs text-zinc-500 mb-1.5 block">
                Specialty (optional)
              </label>
              <Input
                placeholder="e.g. Frontend, Design, Growth"
                value={specialty}
                onChange={(e) => setSpecialty(e.target.value)}
              />
            </div>

            {/* AI configuration — only for AI agents */}
            {kind === "ai_agent" && (
              <div className="col-span-2 space-y-4 border-t border-zinc-800 pt-4">
                {/* Section header + mode toggle */}
                <div className="flex items-center justify-between">
                  <span className="text-xs font-semibold text-zinc-400 uppercase tracking-wider flex items-center gap-1.5">
                    <Cpu className="h-3.5 w-3.5" />
                    AI Configuration
                  </span>
                  {aiProfiles && aiProfiles.length > 0 && (
                    <div className="flex rounded-lg border border-zinc-700 overflow-hidden">
                      {(["new", "existing"] as const).map((mode) => (
                        <button
                          key={mode}
                          type="button"
                          onClick={() => setAiSetupMode(mode)}
                          className={cn(
                            "px-3 py-1 text-xs font-medium transition-colors",
                            aiSetupMode === mode
                              ? "bg-zinc-700 text-white"
                              : "text-zinc-500 hover:text-zinc-300"
                          )}
                        >
                          {mode === "new" ? "Set up new" : "Use existing"}
                        </button>
                      ))}
                    </div>
                  )}
                </div>

                {aiSetupMode === "existing" ? (
                  /* Existing profile selector */
                  <select
                    value={aiProfileId}
                    onChange={(e) => setAiProfileId(e.target.value)}
                    className="w-full rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-white focus:outline-none focus:ring-2 focus:ring-zinc-400"
                  >
                    <option value="">None</option>
                    {aiProfiles?.map((p) => (
                      <option key={p.id} value={p.id}>
                        {p.display_name ?? p.model_id} ({p.provider_kind})
                      </option>
                    ))}
                  </select>
                ) : (
                  /* New profile setup */
                  <div className="space-y-3">
                    {/* Provider selection — shown only when more than one provider */}
                    {providers.length > 1 && (
                      <div className="space-y-1.5">
                        <label className="text-xs text-zinc-500">Provider</label>
                        <div className="flex gap-2 flex-wrap">
                          {providers.map((p) => (
                            <button
                              key={p.kind}
                              type="button"
                              onClick={() => handleSelectProvider(p.kind)}
                              className={cn(
                                "flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-medium transition-colors border",
                                selectedProviderKind === p.kind
                                  ? "bg-zinc-800 border-zinc-600 text-white"
                                  : "border-zinc-800 text-zinc-500 hover:text-zinc-300 hover:border-zinc-700"
                              )}
                            >
                              <Cpu className="h-3 w-3" />
                              {p.display_name}
                            </button>
                          ))}
                        </div>
                      </div>
                    )}

                    {/* Dynamic provider config fields */}
                    {providers
                      .find((p) => p.kind === selectedProviderKind)
                      ?.config_fields.map((field) => (
                        <Input
                          key={field.key}
                          label={field.label}
                          placeholder={field.placeholder}
                          type={field.field_type === "password" ? "password" : "text"}
                          value={providerConfig[field.key] ?? field.default_value ?? ""}
                          onChange={(e) =>
                            setProviderConfig((prev) => ({
                              ...prev,
                              [field.key]: e.target.value,
                            }))
                          }
                        />
                      ))}

                    {/* Model ID */}
                    <Input
                      label="Model"
                      placeholder="e.g. llama3.2, mistral, codellama"
                      value={newModelId}
                      onChange={(e) => setNewModelId(e.target.value)}
                      hint={
                        selectedProviderKind === "ollama"
                          ? "Run `ollama list` to see models available on your machine."
                          : undefined
                      }
                    />

                    {/* Test connection */}
                    <div className="space-y-2">
                      <Button
                        variant="outline"
                        size="sm"
                        type="button"
                        onClick={handleTestConnection}
                        disabled={connStatus === "testing"}
                        className="w-full"
                      >
                        {connStatus === "testing" ? (
                          <>
                            <Loader2 className="h-3.5 w-3.5 animate-spin" />
                            Testing connection…
                          </>
                        ) : (
                          "Test connection"
                        )}
                      </Button>

                      {connStatus === "ok" && (
                        <div className="flex items-center gap-2 text-xs text-emerald-400">
                          <CheckCircle className="h-3.5 w-3.5 shrink-0" />
                          {newModelId.trim()
                            ? `Connected — ${newModelId.trim()} responded successfully.`
                            : "Connected successfully."}
                        </div>
                      )}
                      {connStatus === "error" && (
                        <div className="flex items-start gap-2 text-xs text-red-400">
                          <XCircle className="h-3.5 w-3.5 shrink-0 mt-0.5" />
                          <span>{connError ?? "Connection failed."}</span>
                        </div>
                      )}
                      {connStatus === "idle" && (
                        <p className="text-xs text-zinc-600">
                          Optional — test before adding to confirm the model is reachable.
                        </p>
                      )}
                    </div>
                  </div>
                )}
              </div>
            )}
          </div>

          <div className="flex gap-2 justify-end">
            <Button variant="ghost" size="sm" onClick={resetForm}>
              Cancel
            </Button>
            <Button
              size="sm"
              disabled={
                !name.trim() ||
                (kind === "ai_agent" &&
                  aiSetupMode === "new" &&
                  !newModelId.trim())
              }
              isLoading={createMutation.isPending}
              onClick={() => createMutation.mutate()}
            >
              Add member
            </Button>
          </div>
        </Card>
      )}

      {/* Team roster */}
      {isLoading ? (
        <div className="flex justify-center py-12">
          <Spinner />
        </div>
      ) : !people?.length ? (
        <Card className="text-center py-12">
          <Users className="h-10 w-10 text-zinc-700 mx-auto mb-3" />
          <p className="text-zinc-400 text-sm">No team members yet.</p>
          <p className="text-zinc-600 text-xs mt-1">
            Add human founders or AI agents above.
          </p>
        </Card>
      ) : (
        <div className="space-y-6">
          {/* Human founders */}
          {humanMembers.length > 0 && (
            <section className="space-y-3">
              <h2 className="text-xs font-medium text-zinc-500 uppercase tracking-wider flex items-center gap-2">
                <User className="h-3.5 w-3.5" />
                Human founders
              </h2>
              <div className="space-y-2">
                {humanMembers.map((person) => (
                  <PersonRow
                    key={person.id}
                    companyId={companyId}
                    person={person}
                    aiProfiles={aiProfiles}
                    providers={providers}
                    onDelete={() => deleteMutation.mutate(person.id)}
                    isDeleting={deleteMutation.isPending}
                  />
                ))}
              </div>
            </section>
          )}

          {/* AI agents */}
          {aiAgents.length > 0 && (
            <section className="space-y-3">
              <h2 className="text-xs font-medium text-zinc-500 uppercase tracking-wider flex items-center gap-2">
                <Bot className="h-3.5 w-3.5" />
                AI agents
              </h2>
              <div className="space-y-2">
                {aiAgents.map((person) => (
                  <PersonRow
                    key={person.id}
                    companyId={companyId}
                    person={person}
                    aiProfiles={aiProfiles}
                    providers={providers}
                    onDelete={() => deleteMutation.mutate(person.id)}
                    isDeleting={deleteMutation.isPending}
                  />
                ))}
              </div>
            </section>
          )}
        </div>
      )}

      </>}
    </div>
  );
}

// ─── Org Chart View ───────────────────────────────────────────────────────────

function OrgChartView({
  companyId,
  people,
}: {
  companyId: string;
  people: Person[];
}) {
  const queryClient = useQueryClient();

  const { data: orgNodes, isLoading } = useQuery({
    queryKey: ["org-chart", companyId],
    queryFn: () => getOrgChart(companyId),
  });

  const updateMutation = useMutation({
    mutationFn: ({
      personId,
      managerId,
    }: {
      personId: string;
      managerId: string | null;
    }) => updateReportingLine(companyId, personId, managerId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["org-chart", companyId] });
      queryClient.invalidateQueries({ queryKey: ["people", companyId] });
    },
  });

  if (isLoading) {
    return (
      <div className="flex justify-center py-12">
        <Spinner />
      </div>
    );
  }

  const nodes = orgNodes ?? [];

  if (nodes.length === 0) {
    return (
      <Card className="text-center py-12">
        <GitBranch className="h-10 w-10 text-zinc-700 mx-auto mb-3" />
        <p className="text-zinc-400 text-sm">No team members yet.</p>
        <p className="text-zinc-600 text-xs mt-1">
          Add team members in the Members tab first.
        </p>
      </Card>
    );
  }

  // Build tree: roots are nodes with no manager (or unresolvable manager)
  const nodeMap = new Map(nodes.map((n) => [n.id, n]));
  const roots = nodes.filter(
    (n) => !n.reports_to_person_id || !nodeMap.has(n.reports_to_person_id)
  );

  function getDirectReports(managerId: string): OrgNode[] {
    return nodes.filter((n) => n.reports_to_person_id === managerId);
  }

  return (
    <div className="space-y-4">
      <p className="text-xs text-zinc-500">
        Set reporting lines by choosing each person&apos;s manager below. The chart
        stays acyclic — the API will reject changes that would create a loop.
      </p>

      {/* Flat list with manager pickers */}
      <div className="space-y-2">
        {nodes.map((node) => {
          const isUpdating =
            updateMutation.isPending &&
            (updateMutation.variables as { personId: string })?.personId === node.id;
          const managerNode = node.reports_to_person_id
            ? nodeMap.get(node.reports_to_person_id)
            : null;

          return (
            <Card key={node.id} className="p-4">
              <div className="flex items-center justify-between gap-4">
                <div className="flex items-center gap-3 min-w-0">
                  <div
                    className={cn(
                      "flex h-9 w-9 shrink-0 items-center justify-center rounded-full",
                      node.kind === "ai_agent" ? "bg-blue-950" : "bg-zinc-800"
                    )}
                  >
                    {node.kind === "ai_agent" ? (
                      <Bot className="h-4 w-4 text-blue-400" />
                    ) : (
                      <User className="h-4 w-4 text-zinc-400" />
                    )}
                  </div>
                  <div className="min-w-0">
                    <p className="font-medium text-white truncate">
                      {node.display_name}
                    </p>
                    <p className="text-xs text-zinc-500 truncate">
                      {ROLE_LABELS[node.role_type as RoleType] ?? node.role_type}
                      {node.specialty ? ` · ${node.specialty}` : ""}
                    </p>
                  </div>
                </div>

                {/* Manager picker */}
                <div className="flex items-center gap-2 shrink-0">
                  {managerNode && (
                    <span className="text-xs text-zinc-500 flex items-center gap-1">
                      <ArrowRight className="h-3 w-3" />
                      {managerNode.display_name}
                    </span>
                  )}
                  <div className="relative">
                    <select
                      disabled={isUpdating}
                      value={node.reports_to_person_id ?? ""}
                      onChange={(e) =>
                        updateMutation.mutate({
                          personId: node.id,
                          managerId: e.target.value || null,
                        })
                      }
                      className="rounded-md border border-zinc-700 bg-zinc-800 px-2 py-1 text-xs text-zinc-300 focus:outline-none focus:ring-1 focus:ring-zinc-500 disabled:opacity-50"
                    >
                      <option value="">No manager (root)</option>
                      {nodes
                        .filter((n) => n.id !== node.id)
                        .map((n) => (
                          <option key={n.id} value={n.id}>
                            {n.display_name}
                          </option>
                        ))}
                    </select>
                    {isUpdating && (
                      <Spinner className="absolute right-6 top-1/2 -translate-y-1/2 h-3 w-3" />
                    )}
                  </div>
                </div>
              </div>
            </Card>
          );
        })}
      </div>

      {/* Tree view */}
      {roots.length > 0 && (
        <div className="mt-6">
          <p className="text-xs font-medium text-zinc-500 uppercase tracking-wider mb-3">
            Reporting tree
          </p>
          <div className="space-y-1">
            {roots.map((root) => (
              <OrgTreeNode
                key={root.id}
                node={root}
                getReports={getDirectReports}
                depth={0}
              />
            ))}
          </div>
        </div>
      )}

      {updateMutation.isError && (
        <p className="text-xs text-red-400 mt-2">
          {String((updateMutation.error as Error)?.message ?? "Update failed")}
        </p>
      )}
    </div>
  );
}

function OrgTreeNode({
  node,
  getReports,
  depth,
}: {
  node: OrgNode;
  getReports: (id: string) => OrgNode[];
  depth: number;
}) {
  const reports = getReports(node.id);

  return (
    <div style={{ paddingLeft: depth * 20 }}>
      <div className="flex items-center gap-2 py-1">
        {depth > 0 && (
          <span className="text-zinc-700 select-none">└─</span>
        )}
        <div
          className={cn(
            "flex h-6 w-6 shrink-0 items-center justify-center rounded-full",
            node.kind === "ai_agent" ? "bg-blue-950" : "bg-zinc-800"
          )}
        >
          {node.kind === "ai_agent" ? (
            <Bot className="h-3 w-3 text-blue-400" />
          ) : (
            <User className="h-3 w-3 text-zinc-400" />
          )}
        </div>
        <span className="text-sm text-white">{node.display_name}</span>
        <span className="text-xs text-zinc-600">
          {ROLE_LABELS[node.role_type as RoleType] ?? node.role_type}
        </span>
      </div>
      {reports.map((child) => (
        <OrgTreeNode
          key={child.id}
          node={child}
          getReports={getReports}
          depth={depth + 1}
        />
      ))}
    </div>
  );
}

function PersonRow({
  companyId,
  person,
  aiProfiles,
  providers,
  onDelete,
  isDeleting,
}: {
  companyId: string;
  person: Person;
  aiProfiles?: AiProfile[];
  providers?: ProviderInfo[];
  onDelete: () => void;
  isDeleting: boolean;
}) {
  const queryClient = useQueryClient();
  const [expanded, setExpanded] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const isAi = person.kind === "ai_agent";
  const linkedProfile = aiProfiles?.find((p) => p.id === person.ai_profile_id);

  // Edit field states
  const [editName, setEditName] = useState(person.display_name);
  const [editRole, setEditRole] = useState<RoleType>(person.role_type);
  const [editSpecialty, setEditSpecialty] = useState(person.specialty ?? "");

  // AI profile edit states
  // "edit_model" = patch the linked profile in-place; "link" = change which profile is linked
  const [aiEditMode, setAiEditMode] = useState<"edit_model" | "link">("edit_model");
  const [editAiProfileId, setEditAiProfileId] = useState(person.ai_profile_id ?? "");
  const [editModelId, setEditModelId] = useState(linkedProfile?.model_id ?? "");
  const [editProviderConfig, setEditProviderConfig] = useState<Record<string, string>>(() =>
    Object.fromEntries(
      Object.entries(linkedProfile?.provider_config ?? {})
        .filter(([, v]) => typeof v === "string")
        .map(([k, v]) => [k, v as string])
    )
  );
  const [connStatus, setConnStatus] = useState<"idle" | "testing" | "ok" | "error">("idle");
  const [connError, setConnError] = useState<string | null>(null);

  // Save state
  const [isSaving, setIsSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);

  const currentProvider = providers?.find((p) => p.kind === linkedProfile?.provider_kind);

  function startEditing() {
    setEditName(person.display_name);
    setEditRole(person.role_type);
    setEditSpecialty(person.specialty ?? "");
    setAiEditMode(linkedProfile ? "edit_model" : "link");
    setEditAiProfileId(person.ai_profile_id ?? "");
    setEditModelId(linkedProfile?.model_id ?? "");
    setEditProviderConfig(
      Object.fromEntries(
        Object.entries(linkedProfile?.provider_config ?? {})
          .filter(([, v]) => typeof v === "string")
          .map(([k, v]) => [k, v as string])
      )
    );
    setConnStatus("idle");
    setConnError(null);
    setSaveError(null);
    setExpanded(false);
    setIsEditing(true);
  }

  async function handleTestConnection() {
    if (!linkedProfile) return;
    setConnStatus("testing");
    setConnError(null);
    try {
      const result = await testConnection({
        provider_kind: linkedProfile.provider_kind,
        provider_config: {
          schema_version: 1,
          ...Object.fromEntries(
            Object.entries(editProviderConfig).map(([k, v]) => [k, v.trim()])
          ),
        },
        model_id: editModelId.trim() || undefined,
      });
      setConnStatus(result.ok ? "ok" : "error");
      if (!result.ok) setConnError(result.error ?? "Connection failed");
    } catch {
      setConnStatus("error");
      setConnError("Network error — is the API running?");
    }
  }

  async function handleSave() {
    if (!editName.trim()) return;
    setIsSaving(true);
    setSaveError(null);
    try {
      // Patch the linked AI profile in-place (model + config only; provider_kind is immutable)
      if (isAi && aiEditMode === "edit_model" && linkedProfile) {
        const modelChanged = editModelId.trim() !== linkedProfile.model_id;
        const origConfig = Object.fromEntries(
          Object.entries(linkedProfile.provider_config)
            .filter(([, v]) => typeof v === "string")
            .map(([k, v]) => [k, v as string])
        );
        const configChanged =
          JSON.stringify(editProviderConfig) !== JSON.stringify(origConfig);

        if (modelChanged || configChanged) {
          await updateAiProfile(companyId, linkedProfile.id, {
            model_id: editModelId.trim() || undefined,
            provider_config: {
              schema_version: 1,
              ...Object.fromEntries(
                Object.entries(editProviderConfig).map(([k, v]) => [k, v.trim()])
              ),
            },
          });
          queryClient.invalidateQueries({ queryKey: ["ai-profiles", companyId] });
        }
      }

      // Patch person fields
      const personInput: Parameters<typeof updatePerson>[2] = {
        display_name: editName.trim(),
        role_type: editRole,
        specialty: editSpecialty.trim() || null,
      };
      if (isAi && aiEditMode === "link") {
        personInput.ai_profile_id = editAiProfileId || null;
      }

      await updatePerson(companyId, person.id, personInput);
      queryClient.invalidateQueries({ queryKey: ["people", companyId] });
      setIsEditing(false);
    } catch {
      setSaveError("Failed to save changes. Please try again.");
    } finally {
      setIsSaving(false);
    }
  }

  return (
    <Card className="p-4">
      <div className="flex items-center justify-between gap-4">
        {/* Avatar + name */}
        <div className="flex items-center gap-3 min-w-0">
          <div
            className={cn(
              "flex h-9 w-9 shrink-0 items-center justify-center rounded-full",
              isAi ? "bg-blue-950" : "bg-zinc-800"
            )}
          >
            {isAi ? (
              <Bot className="h-4 w-4 text-blue-400" />
            ) : (
              <User className="h-4 w-4 text-zinc-400" />
            )}
          </div>
          <div className="min-w-0">
            <p className="font-medium text-white truncate">{person.display_name}</p>
            {person.specialty && (
              <p className="text-xs text-zinc-500 truncate">{person.specialty}</p>
            )}
          </div>
        </div>

        {/* Role badge + actions */}
        <div className="flex items-center gap-3 shrink-0">
          <span
            className={cn(
              "text-xs font-medium px-2 py-0.5 rounded-full",
              ROLE_COLORS[person.role_type]
            )}
          >
            {ROLE_LABELS[person.role_type]}
          </span>

          {isAi && !isEditing && (
            <button
              onClick={() => setExpanded((v) => !v)}
              className="text-zinc-600 hover:text-zinc-400 transition-colors"
              title="AI details"
            >
              {expanded ? (
                <ChevronUp className="h-4 w-4" />
              ) : (
                <ChevronDown className="h-4 w-4" />
              )}
            </button>
          )}

          <button
            onClick={isEditing ? () => setIsEditing(false) : startEditing}
            disabled={isDeleting || isSaving}
            className={cn(
              "transition-colors disabled:opacity-50",
              isEditing
                ? "text-zinc-500 hover:text-zinc-300"
                : "text-zinc-600 hover:text-zinc-300"
            )}
            title={isEditing ? "Cancel editing" : "Edit member"}
          >
            <Pencil className="h-4 w-4" />
          </button>

          <button
            onClick={onDelete}
            disabled={isDeleting || isEditing || isSaving}
            className="text-zinc-700 hover:text-red-400 transition-colors disabled:opacity-50"
            title="Remove member"
          >
            <Trash2 className="h-4 w-4" />
          </button>
        </div>
      </div>

      {/* AI profile details (read-only expand) */}
      {expanded && isAi && !isEditing && (
        <div className="mt-3 pt-3 border-t border-zinc-800">
          {linkedProfile ? (
            <div className="flex items-center gap-2 text-xs text-zinc-400">
              <Cpu className="h-3.5 w-3.5 text-zinc-500" />
              <span className="text-zinc-500">Model:</span>
              <span className="font-mono text-zinc-300">{linkedProfile.model_id}</span>
              <span className="text-zinc-700">·</span>
              <span className="capitalize text-zinc-500">{linkedProfile.provider_kind}</span>
            </div>
          ) : (
            <p className="text-xs text-zinc-600 flex items-center gap-2">
              <Cpu className="h-3.5 w-3.5" />
              No AI profile linked
            </p>
          )}
        </div>
      )}

      {/* Inline edit form */}
      {isEditing && (
        <div className="mt-4 pt-4 border-t border-zinc-800 space-y-4">
          {/* Basic fields */}
          <div className="grid grid-cols-2 gap-3">
            <div className="col-span-2">
              <Input
                placeholder="Display name"
                value={editName}
                onChange={(e) => setEditName(e.target.value)}
                autoFocus
              />
            </div>
            <div>
              <label className="text-xs text-zinc-500 mb-1.5 block">Role</label>
              <select
                value={editRole}
                onChange={(e) => setEditRole(e.target.value as RoleType)}
                className="w-full rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-white focus:outline-none focus:ring-2 focus:ring-zinc-400"
              >
                <option value="co_founder">Co-founder</option>
                <option value="ceo">CEO</option>
                <option value="cto">CTO</option>
                <option value="specialist">Specialist</option>
              </select>
            </div>
            <div>
              <Input
                label="Specialty (optional)"
                placeholder="e.g. Frontend, Design, Growth"
                value={editSpecialty}
                onChange={(e) => setEditSpecialty(e.target.value)}
              />
            </div>
          </div>

          {/* AI profile section */}
          {isAi && (
            <div className="border-t border-zinc-800 pt-4 space-y-3">
              <div className="flex items-center justify-between">
                <span className="text-xs font-semibold text-zinc-400 uppercase tracking-wider flex items-center gap-1.5">
                  <Cpu className="h-3.5 w-3.5" />
                  AI Model
                </span>
                <div className="flex rounded-lg border border-zinc-700 overflow-hidden">
                  {(["edit_model", "link"] as const).map((mode) => (
                    <button
                      key={mode}
                      type="button"
                      onClick={() => setAiEditMode(mode)}
                      className={cn(
                        "px-3 py-1 text-xs font-medium transition-colors",
                        aiEditMode === mode
                          ? "bg-zinc-700 text-white"
                          : "text-zinc-500 hover:text-zinc-300"
                      )}
                    >
                      {mode === "edit_model" ? "Edit model" : "Switch profile"}
                    </button>
                  ))}
                </div>
              </div>

              {aiEditMode === "link" ? (
                /* Switch to a different existing profile */
                <div className="space-y-1.5">
                  <label className="text-xs text-zinc-500">AI profile</label>
                  <select
                    value={editAiProfileId}
                    onChange={(e) => setEditAiProfileId(e.target.value)}
                    className="w-full rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-white focus:outline-none focus:ring-2 focus:ring-zinc-400"
                  >
                    <option value="">No profile (disable AI)</option>
                    {aiProfiles?.map((p) => (
                      <option key={p.id} value={p.id}>
                        {p.display_name ?? p.model_id} · {p.provider_kind}
                      </option>
                    ))}
                  </select>
                  <p className="text-xs text-zinc-600">
                    Select a previously configured AI profile, or remove the link.
                  </p>
                </div>
              ) : linkedProfile ? (
                /* Edit the linked profile's model + config in-place */
                <div className="space-y-3">
                  <div className="flex items-center gap-2 text-xs">
                    <Cpu className="h-3 w-3 text-zinc-500" />
                    <span className="text-zinc-500">Provider:</span>
                    <span className="text-zinc-300 capitalize">{linkedProfile.provider_kind}</span>
                    <span className="text-zinc-700 text-xs">(immutable — switch profiles to change)</span>
                  </div>

                  <Input
                    label="Model"
                    placeholder="e.g. llama3.2, mistral, codellama"
                    value={editModelId}
                    onChange={(e) => {
                      setEditModelId(e.target.value);
                      setConnStatus("idle");
                    }}
                    hint={
                      linkedProfile.provider_kind === "ollama"
                        ? "Run `ollama list` to see available models."
                        : undefined
                    }
                  />

                  {currentProvider?.config_fields.map((field) => (
                    <Input
                      key={field.key}
                      label={field.label}
                      placeholder={field.placeholder}
                      type={field.field_type === "password" ? "password" : "text"}
                      value={editProviderConfig[field.key] ?? field.default_value ?? ""}
                      onChange={(e) => {
                        setEditProviderConfig((prev) => ({
                          ...prev,
                          [field.key]: e.target.value,
                        }));
                        setConnStatus("idle");
                      }}
                    />
                  ))}

                  {/* Test connection */}
                  <div className="space-y-2">
                    <Button
                      variant="outline"
                      size="sm"
                      type="button"
                      onClick={handleTestConnection}
                      disabled={connStatus === "testing"}
                      className="w-full"
                    >
                      {connStatus === "testing" ? (
                        <>
                          <Loader2 className="h-3.5 w-3.5 animate-spin" />
                          Testing connection…
                        </>
                      ) : (
                        "Test connection"
                      )}
                    </Button>
                    {connStatus === "ok" && (
                      <div className="flex items-center gap-2 text-xs text-emerald-400">
                        <CheckCircle className="h-3.5 w-3.5 shrink-0" />
                        {editModelId.trim()
                          ? `Connected — ${editModelId.trim()} responded successfully.`
                          : "Connected successfully."}
                      </div>
                    )}
                    {connStatus === "error" && (
                      <div className="flex items-start gap-2 text-xs text-red-400">
                        <XCircle className="h-3.5 w-3.5 shrink-0 mt-0.5" />
                        <span>{connError ?? "Connection failed."}</span>
                      </div>
                    )}
                  </div>
                </div>
              ) : (
                <p className="text-xs text-zinc-600">
                  No AI profile linked. Switch to &quot;Switch profile&quot; to link one.
                </p>
              )}
            </div>
          )}

          {/* Error */}
          {saveError && (
            <p className="text-xs text-red-400 flex items-center gap-1.5">
              <XCircle className="h-3.5 w-3.5 shrink-0" />
              {saveError}
            </p>
          )}

          {/* Actions */}
          <div className="flex justify-end gap-2">
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setIsEditing(false)}
              disabled={isSaving}
            >
              Cancel
            </Button>
            <Button
              size="sm"
              onClick={handleSave}
              isLoading={isSaving}
              disabled={!editName.trim() || isSaving}
            >
              Save changes
            </Button>
          </div>
        </div>
      )}
    </Card>
  );
}
