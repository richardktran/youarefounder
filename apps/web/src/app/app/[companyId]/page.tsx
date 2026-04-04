"use client";

import { useParams } from "next/navigation";
import { useQuery } from "@tanstack/react-query";
import { Package, Users, FolderKanban, Inbox, ArrowRight } from "lucide-react";
import Link from "next/link";
import { getCompany, listProducts } from "@/lib/api";
import { Spinner } from "@/components/ui/spinner";
import { Card } from "@/components/ui/card";
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

export default function CompanyOverview() {
  const params = useParams<{ companyId: string }>();
  const companyId = params.companyId;

  const { data: company, isLoading: companyLoading } = useQuery({
    queryKey: ["company", companyId],
    queryFn: () => getCompany(companyId),
  });

  const { data: products, isLoading: productsLoading } = useQuery({
    queryKey: ["products", companyId],
    queryFn: () => listProducts(companyId),
  });

  if (companyLoading) {
    return (
      <div className="flex h-full items-center justify-center p-12">
        <Spinner />
      </div>
    );
  }

  if (!company) return null;

  const upcomingPhases = [
    {
      phase: "Phase 1",
      title: "Onboarding + Ollama",
      description: "AI co-founder, model config, and test connection.",
    },
    {
      phase: "Phase 2",
      title: "Workspaces & Tickets",
      description: "Jira-lite CRUD — tickets, comments, status flows.",
    },
    {
      phase: "Phase 3",
      title: "First agent loop",
      description: "Worker picks tickets, calls LLM, writes actions back.",
    },
  ];

  return (
    <div className="p-8 space-y-8 max-w-4xl">
      {/* Page header */}
      <div>
        <h1 className="text-2xl font-bold text-white">{company.name}</h1>
        <p className="text-zinc-400 mt-1">Company overview</p>
      </div>

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
          value="0"
          href={`/app/${companyId}/workspaces`}
          dimmed
        />
        <StatCard
          icon={<Users className="h-5 w-5" />}
          label="Team members"
          value="0"
          href={`/app/${companyId}/team`}
          dimmed
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

      {/* Roadmap preview */}
      <section className="space-y-3">
        <h2 className="text-sm font-medium text-zinc-400 uppercase tracking-wider">
          Coming up
        </h2>
        <div className="space-y-2">
          {upcomingPhases.map((item) => (
            <div
              key={item.phase}
              className="flex items-start gap-4 rounded-xl border border-zinc-800/60 bg-zinc-900/30 px-5 py-4"
            >
              <span className="shrink-0 text-xs font-mono text-zinc-600 pt-0.5 w-16">
                {item.phase}
              </span>
              <div>
                <p className="text-sm font-medium text-zinc-300">
                  {item.title}
                </p>
                <p className="text-xs text-zinc-500 mt-0.5">
                  {item.description}
                </p>
              </div>
            </div>
          ))}
        </div>
      </section>
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
