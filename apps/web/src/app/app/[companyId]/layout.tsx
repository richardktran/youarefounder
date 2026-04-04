"use client";

import Link from "next/link";
import { useParams, usePathname } from "next/navigation";
import { useQuery } from "@tanstack/react-query";
import {
  Building2,
  LayoutDashboard,
  FolderKanban,
  Users,
  Settings,
  Inbox,
} from "lucide-react";
import { getCompany } from "@/lib/api";
import { cn } from "@/lib/utils";

const navItems = (companyId: string) => [
  {
    href: `/app/${companyId}`,
    label: "Overview",
    icon: <LayoutDashboard className="h-4 w-4" />,
    exact: true,
  },
  {
    href: `/app/${companyId}/workspaces`,
    label: "Workspaces",
    icon: <FolderKanban className="h-4 w-4" />,
  },
  {
    href: `/app/${companyId}/inbox`,
    label: "Inbox",
    icon: <Inbox className="h-4 w-4" />,
  },
  {
    href: `/app/${companyId}/team`,
    label: "Team",
    icon: <Users className="h-4 w-4" />,
  },
  {
    href: `/app/${companyId}/settings`,
    label: "Settings",
    icon: <Settings className="h-4 w-4" />,
  },
];

export default function AppLayout({ children }: { children: React.ReactNode }) {
  const params = useParams<{ companyId: string }>();
  const pathname = usePathname();
  const companyId = params.companyId;

  const { data: company } = useQuery({
    queryKey: ["company", companyId],
    queryFn: () => getCompany(companyId),
    enabled: !!companyId,
  });

  const items = navItems(companyId);

  return (
    <div className="flex h-screen overflow-hidden">
      {/* Sidebar */}
      <aside className="w-60 shrink-0 border-r border-zinc-800 bg-zinc-950 flex flex-col">
        {/* Brand */}
        <div className="flex items-center gap-3 px-4 py-5 border-b border-zinc-800">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-zinc-800">
            <Building2 className="h-4 w-4 text-zinc-300" />
          </div>
          <div className="min-w-0">
            <p className="text-sm font-semibold text-white truncate">
              {company?.name ?? "Loading…"}
            </p>
            <p className="text-xs text-zinc-500">Founder dashboard</p>
          </div>
        </div>

        {/* Nav */}
        <nav className="flex-1 px-3 py-4 space-y-0.5">
          {items.map((item) => {
            const isActive = item.exact
              ? pathname === item.href
              : pathname.startsWith(item.href);
            return (
              <Link
                key={item.href}
                href={item.href}
                className={cn(
                  "flex items-center gap-3 rounded-lg px-3 py-2 text-sm transition-colors",
                  isActive
                    ? "bg-zinc-800 text-white font-medium"
                    : "text-zinc-400 hover:bg-zinc-900 hover:text-zinc-200"
                )}
              >
                {item.icon}
                {item.label}
              </Link>
            );
          })}
        </nav>

        {/* Phase badge */}
        <div className="px-4 py-4 border-t border-zinc-800">
          <div className="rounded-lg bg-zinc-900 px-3 py-2 text-xs text-zinc-500">
            Phase 3–3.5 · Hiring & Org chart
          </div>
        </div>
      </aside>

      {/* Main content */}
      <main className="flex-1 overflow-y-auto overflow-x-hidden">{children}</main>
    </div>
  );
}
