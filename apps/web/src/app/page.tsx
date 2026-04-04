"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";
import { useQuery } from "@tanstack/react-query";
import { getBootstrap } from "@/lib/api";
import { Spinner } from "@/components/ui/spinner";

/**
 * Root page — determines where to route the user on launch.
 *
 * - No company / onboarding incomplete → `/onboarding`
 * - Company exists + onboarding complete → `/app/:companyId`
 *
 * No login screen. No JWT. Phase 1.
 */
export default function RootPage() {
  const router = useRouter();

  const { data, isError } = useQuery({
    queryKey: ["bootstrap"],
    queryFn: getBootstrap,
    retry: 3,
    retryDelay: 1000,
  });

  useEffect(() => {
    if (!data) return;

    if (data.onboarding_complete && data.company_id) {
      router.replace(`/app/${data.company_id}`);
    } else {
      router.replace("/onboarding");
    }
  }, [data, router]);

  if (isError) {
    return (
      <div className="flex min-h-screen items-center justify-center">
        <div className="text-center space-y-3">
          <p className="text-red-400 font-medium">Cannot connect to the API.</p>
          <p className="text-zinc-500 text-sm">
            Make sure the backend is running on port 3001.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex min-h-screen items-center justify-center">
      <Spinner />
    </div>
  );
}
