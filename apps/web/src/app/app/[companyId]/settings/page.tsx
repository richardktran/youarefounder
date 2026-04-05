"use client";

import { useParams } from "next/navigation";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { getCompany, listProducts, updateCompany, updateProduct } from "@/lib/api";
import { Zap } from "lucide-react";
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
  const qc = useQueryClient();

  const { data: company, isLoading: companyLoading } = useQuery({
    queryKey: ["company", companyId],
    queryFn: () => getCompany(companyId),
  });

  const { data: products, isLoading: productsLoading } = useQuery({
    queryKey: ["products", companyId],
    queryFn: () => listProducts(companyId),
  });

  const [companyName, setCompanyName] = useState<string>("");
  const [savedCompany, setSavedCompany] = useState(false);
  const [maxConcurrent, setMaxConcurrent] = useState<number>(1);
  const [savedConcurrency, setSavedConcurrency] = useState(false);

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
