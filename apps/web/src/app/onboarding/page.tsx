"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { useMutation } from "@tanstack/react-query";
import {
  Building2,
  Package,
  Bot,
  CheckCircle2,
  CheckCircle,
  XCircle,
  Loader2,
} from "lucide-react";
import {
  createCompany,
  createAiProfile,
  createPerson,
  completeOnboarding,
  testConnection,
} from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";

// ─── Wizard steps ─────────────────────────────────────────────────────────────

type Step = "company" | "product" | "ai" | "confirm";

const STEPS: { id: Step; label: string; icon: React.ReactNode }[] = [
  { id: "company", label: "Company", icon: <Building2 className="h-4 w-4" /> },
  { id: "product", label: "Product", icon: <Package className="h-4 w-4" /> },
  { id: "ai", label: "AI Setup", icon: <Bot className="h-4 w-4" /> },
  {
    id: "confirm",
    label: "Launch",
    icon: <CheckCircle2 className="h-4 w-4" />,
  },
];

export default function OnboardingPage() {
  const router = useRouter();

  const [step, setStep] = useState<Step>("company");

  // Company + product fields
  const [companyName, setCompanyName] = useState("");
  const [productName, setProductName] = useState("");
  const [productDescription, setProductDescription] = useState("");

  // AI step fields
  const [ollamaBaseUrl, setOllamaBaseUrl] = useState(
    "http://127.0.0.1:11434"
  );
  const [modelId, setModelId] = useState("llama3.2");
  const [cofounderName, setCofounderName] = useState("Alex");
  const [connectionStatus, setConnectionStatus] = useState<
    "idle" | "testing" | "ok" | "error"
  >("idle");
  const [connectionError, setConnectionError] = useState<string | null>(null);

  const [errors, setErrors] = useState<Record<string, string>>({});

  // ── Mutations ────────────────────────────────────────────────────────────────

  const launchMutation = useMutation({
    mutationFn: async () => {
      // 1. Create company (with product inline)
      const company = await createCompany({
        name: companyName.trim(),
        product: {
          name: productName.trim(),
          description: productDescription.trim() || undefined,
        },
      });

      // 2. Create AI profile
      const profile = await createAiProfile(company.id, {
        display_name: cofounderName.trim(),
        provider_kind: "ollama",
        model_id: modelId.trim(),
        provider_config: {
          schema_version: 1,
          base_url: ollamaBaseUrl.trim(),
        },
      });

      // 3. Create AI co-founder person linked to the profile
      await createPerson(company.id, {
        kind: "ai_agent",
        display_name: cofounderName.trim(),
        role_type: "co_founder",
        ai_profile_id: profile.id,
      });

      // 4. Complete onboarding
      await completeOnboarding(company.id);
      return company;
    },
    onSuccess: (company) => {
      router.push(`/app/${company.id}`);
    },
  });

  // ── Validation ───────────────────────────────────────────────────────────────

  function validate(currentStep: Step): boolean {
    const errs: Record<string, string> = {};
    if (currentStep === "company" && !companyName.trim()) {
      errs.companyName = "Company name is required";
    }
    if (currentStep === "product" && !productName.trim()) {
      errs.productName = "Product name is required";
    }
    if (currentStep === "ai") {
      if (!ollamaBaseUrl.trim())
        errs.ollamaBaseUrl = "Ollama base URL is required";
      if (!modelId.trim()) errs.modelId = "Model ID is required";
      if (!cofounderName.trim())
        errs.cofounderName = "Co-founder name is required";
    }
    setErrors(errs);
    return Object.keys(errs).length === 0;
  }

  function next() {
    if (!validate(step)) return;
    if (step === "company") setStep("product");
    else if (step === "product") setStep("ai");
    else if (step === "ai") setStep("confirm");
  }

  function back() {
    if (step === "product") setStep("company");
    else if (step === "ai") setStep("product");
    else if (step === "confirm") setStep("ai");
  }

  async function handleTestConnection() {
    if (!ollamaBaseUrl.trim()) {
      setErrors((e) => ({ ...e, ollamaBaseUrl: "Base URL is required" }));
      return;
    }
    setConnectionStatus("testing");
    setConnectionError(null);
    try {
      const result = await testConnection({
        provider_kind: "ollama",
        provider_config: {
          schema_version: 1,
          base_url: ollamaBaseUrl.trim(),
        },
        model_id: modelId.trim() || undefined,
      });
      if (result.ok) {
        setConnectionStatus("ok");
      } else {
        setConnectionStatus("error");
        setConnectionError(result.error ?? "Connection failed");
      }
    } catch {
      setConnectionStatus("error");
      setConnectionError("Network error — is the API running?");
    }
  }

  const stepIndex = STEPS.findIndex((s) => s.id === step);

  return (
    <div className="min-h-screen flex items-center justify-center p-6">
      <div className="w-full max-w-lg space-y-8">
        {/* Header */}
        <div className="text-center space-y-2">
          <div className="inline-flex h-12 w-12 items-center justify-center rounded-xl bg-zinc-800 mb-4">
            <Building2 className="h-6 w-6 text-zinc-300" />
          </div>
          <h1 className="text-3xl font-bold text-white">You Are Founder</h1>
          <p className="text-zinc-400">
            Set up your autonomous AI company in minutes.
          </p>
        </div>

        {/* Step progress */}
        <div className="flex items-center gap-2">
          {STEPS.map((s, i) => (
            <div key={s.id} className="flex items-center gap-2 flex-1">
              <div
                className={cn(
                  "flex items-center gap-1.5 text-xs font-medium px-3 py-1.5 rounded-full transition-colors whitespace-nowrap",
                  i < stepIndex
                    ? "bg-zinc-700 text-zinc-300"
                    : i === stepIndex
                      ? "bg-white text-black"
                      : "bg-zinc-900 text-zinc-600 border border-zinc-800"
                )}
              >
                {s.icon}
                {s.label}
              </div>
              {i < STEPS.length - 1 && (
                <div
                  className={cn(
                    "h-px flex-1 transition-colors",
                    i < stepIndex ? "bg-zinc-600" : "bg-zinc-800"
                  )}
                />
              )}
            </div>
          ))}
        </div>

        {/* Step content */}
        <div className="rounded-xl border border-zinc-800 bg-zinc-900/50 p-8 space-y-6">
          {step === "company" && (
            <CompanyStep
              value={companyName}
              onChange={setCompanyName}
              error={errors.companyName}
            />
          )}
          {step === "product" && (
            <ProductStep
              name={productName}
              description={productDescription}
              onChangeName={setProductName}
              onChangeDescription={setProductDescription}
              error={errors.productName}
            />
          )}
          {step === "ai" && (
            <AiStep
              ollamaBaseUrl={ollamaBaseUrl}
              onChangeOllamaBaseUrl={setOllamaBaseUrl}
              modelId={modelId}
              onChangeModelId={setModelId}
              cofounderName={cofounderName}
              onChangeCofounderName={setCofounderName}
              connectionStatus={connectionStatus}
              connectionError={connectionError}
              onTestConnection={handleTestConnection}
              errors={errors}
            />
          )}
          {step === "confirm" && (
            <ConfirmStep
              companyName={companyName}
              productName={productName}
              productDescription={productDescription}
              cofounderName={cofounderName}
              modelId={modelId}
              ollamaBaseUrl={ollamaBaseUrl}
              error={launchMutation.error?.message}
            />
          )}
        </div>

        {/* Navigation */}
        <div className="flex items-center justify-between">
          <Button
            variant="ghost"
            onClick={back}
            disabled={step === "company" || launchMutation.isPending}
          >
            Back
          </Button>

          {step !== "confirm" ? (
            <Button onClick={next}>Continue</Button>
          ) : (
            <Button
              onClick={() => launchMutation.mutate()}
              isLoading={launchMutation.isPending}
            >
              Launch my company
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}

// ─── Step sub-components ──────────────────────────────────────────────────────

function CompanyStep({
  value,
  onChange,
  error,
}: {
  value: string;
  onChange: (v: string) => void;
  error?: string;
}) {
  return (
    <div className="space-y-4">
      <div>
        <h2 className="text-xl font-semibold text-white">Name your company</h2>
        <p className="text-sm text-zinc-400 mt-1">
          This is what your AI team will call themselves. You can change it
          later.
        </p>
      </div>
      <Input
        label="Company name"
        placeholder="e.g. Acme Corp, Moonshot Labs"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        error={error}
        autoFocus
      />
    </div>
  );
}

function ProductStep({
  name,
  description,
  onChangeName,
  onChangeDescription,
  error,
}: {
  name: string;
  description: string;
  onChangeName: (v: string) => void;
  onChangeDescription: (v: string) => void;
  error?: string;
}) {
  return (
    <div className="space-y-4">
      <div>
        <h2 className="text-xl font-semibold text-white">
          Describe your first product
        </h2>
        <p className="text-sm text-zinc-400 mt-1">
          Your AI co-founder will use this as the starting context for all
          strategy and work.
        </p>
      </div>
      <Input
        label="Product name"
        placeholder="e.g. FounderOS, AutoPilot, NexGen CRM"
        value={name}
        onChange={(e) => onChangeName(e.target.value)}
        error={error}
        autoFocus
      />
      <Textarea
        label="What problem does it solve?"
        placeholder="Describe the problem, target audience, and what makes it different…"
        value={description}
        onChange={(e) => onChangeDescription(e.target.value)}
        rows={4}
        hint="Optional but recommended — more context = better AI output."
      />
    </div>
  );
}

function AiStep({
  ollamaBaseUrl,
  onChangeOllamaBaseUrl,
  modelId,
  onChangeModelId,
  cofounderName,
  onChangeCofounderName,
  connectionStatus,
  connectionError,
  onTestConnection,
  errors,
}: {
  ollamaBaseUrl: string;
  onChangeOllamaBaseUrl: (v: string) => void;
  modelId: string;
  onChangeModelId: (v: string) => void;
  cofounderName: string;
  onChangeCofounderName: (v: string) => void;
  connectionStatus: "idle" | "testing" | "ok" | "error";
  connectionError: string | null;
  onTestConnection: () => void;
  errors: Record<string, string>;
}) {
  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-xl font-semibold text-white">
          Configure your AI co-founder
        </h2>
        <p className="text-sm text-zinc-400 mt-1">
          Connect to your local Ollama instance. Make sure Ollama is running
          before testing the connection.
        </p>
      </div>

      <div className="space-y-4">
        <Input
          label="Co-founder name"
          placeholder="e.g. Alex, Jordan, Sam"
          value={cofounderName}
          onChange={(e) => onChangeCofounderName(e.target.value)}
          error={errors.cofounderName}
          autoFocus
          hint="Give your AI co-founder a name — it'll use this identity when communicating."
        />

        <Input
          label="Ollama base URL"
          placeholder="http://127.0.0.1:11434"
          value={ollamaBaseUrl}
          onChange={(e) => onChangeOllamaBaseUrl(e.target.value)}
          error={errors.ollamaBaseUrl}
        />

        <Input
          label="Model"
          placeholder="e.g. llama3.2, mistral, codellama"
          value={modelId}
          onChange={(e) => onChangeModelId(e.target.value)}
          error={errors.modelId}
          hint="Run `ollama list` to see available models on your machine."
        />
      </div>

      {/* Test connection button + status */}
      <div className="space-y-2">
        <Button
          variant="outline"
          onClick={onTestConnection}
          disabled={connectionStatus === "testing"}
          className="w-full"
        >
          {connectionStatus === "testing" ? (
            <>
              <Loader2 className="h-4 w-4 mr-2 animate-spin" />
              Testing connection…
            </>
          ) : (
            "Test connection"
          )}
        </Button>

        {connectionStatus === "ok" && (
          <div className="flex items-center gap-2 text-sm text-emerald-400">
            <CheckCircle className="h-4 w-4 shrink-0" />
            {modelId.trim()
              ? `Connected — ${modelId.trim()} responded successfully.`
              : "Connected to Ollama successfully."}
          </div>
        )}
        {connectionStatus === "error" && (
          <div className="flex items-start gap-2 text-sm text-red-400">
            <XCircle className="h-4 w-4 shrink-0 mt-0.5" />
            <span>{connectionError ?? "Connection failed."}</span>
          </div>
        )}
        {connectionStatus === "idle" && (
          <p className="text-xs text-zinc-500">
            Test the connection before continuing — it&apos;s optional but
            recommended.
          </p>
        )}
      </div>
    </div>
  );
}

function ConfirmStep({
  companyName,
  productName,
  productDescription,
  cofounderName,
  modelId,
  ollamaBaseUrl,
  error,
}: {
  companyName: string;
  productName: string;
  productDescription: string;
  cofounderName: string;
  modelId: string;
  ollamaBaseUrl: string;
  error?: string;
}) {
  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-xl font-semibold text-white">Ready to launch</h2>
        <p className="text-sm text-zinc-400 mt-1">
          Review your setup before we create your company.
        </p>
      </div>

      <div className="space-y-3">
        <SummaryRow label="Company" value={companyName} />
        <SummaryRow label="Product" value={productName} />
        {productDescription && (
          <SummaryRow label="Description" value={productDescription} multiline />
        )}
        <div className="h-px bg-zinc-800 my-1" />
        <SummaryRow label="AI co-founder" value={cofounderName} />
        <SummaryRow label="Provider" value="Ollama (Local)" />
        <SummaryRow label="Model" value={modelId} />
        <SummaryRow label="Ollama URL" value={ollamaBaseUrl} />
      </div>

      {error && (
        <div className="rounded-lg bg-red-950 border border-red-800 px-4 py-3">
          <p className="text-sm text-red-400">{error}</p>
        </div>
      )}

      <div className="rounded-lg bg-zinc-800/60 px-4 py-3 text-sm text-zinc-400">
        <strong className="text-zinc-300">What happens next:</strong> Your
        company, product, and AI co-founder are created locally. Your AI team
        will start working on tickets once you&apos;re set up.
      </div>
    </div>
  );
}

function SummaryRow({
  label,
  value,
  multiline,
}: {
  label: string;
  value: string;
  multiline?: boolean;
}) {
  return (
    <div className="flex gap-4">
      <span className="text-sm text-zinc-500 w-28 shrink-0 pt-0.5">{label}</span>
      <span
        className={cn(
          "text-sm text-zinc-200",
          multiline && "whitespace-pre-wrap"
        )}
      >
        {value}
      </span>
    </div>
  );
}
