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
} from "lucide-react";
import {
  listPeople,
  createPerson,
  deletePerson,
  listAiProfiles,
  listAiProviders,
  createAiProfile,
  testConnection,
  type Person,
  type PersonKind,
  type RoleType,
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

export default function TeamPage() {
  const params = useParams<{ companyId: string }>();
  const companyId = params.companyId;
  const queryClient = useQueryClient();

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
  const [newModelId, setNewModelId] = useState("");
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
    setNewModelId("");
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
        <Button size="sm" onClick={() => setShowForm((v) => !v)}>
          <Plus className="h-4 w-4" />
          Add member
        </Button>
      </div>

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
                    person={person}
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
                    person={person}
                    aiProfiles={aiProfiles}
                    onDelete={() => deleteMutation.mutate(person.id)}
                    isDeleting={deleteMutation.isPending}
                  />
                ))}
              </div>
            </section>
          )}
        </div>
      )}
    </div>
  );
}

function PersonRow({
  person,
  aiProfiles,
  onDelete,
  isDeleting,
}: {
  person: Person;
  aiProfiles?: import("@/lib/api").AiProfile[];
  onDelete: () => void;
  isDeleting: boolean;
}) {
  const [expanded, setExpanded] = useState(false);
  const isAi = person.kind === "ai_agent";
  const linkedProfile = aiProfiles?.find((p) => p.id === person.ai_profile_id);

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

          {isAi && (
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
            onClick={onDelete}
            disabled={isDeleting}
            className="text-zinc-700 hover:text-red-400 transition-colors disabled:opacity-50"
            title="Remove member"
          >
            <Trash2 className="h-4 w-4" />
          </button>
        </div>
      </div>

      {/* AI profile details */}
      {expanded && isAi && (
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
    </Card>
  );
}
