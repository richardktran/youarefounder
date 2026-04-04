import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // Rewrite API calls in dev so the browser talks to localhost:3001
  async rewrites() {
    return [
      {
        source: "/api/:path*",
        destination: "http://127.0.0.1:3001/v1/:path*",
      },
    ];
  },
};

export default nextConfig;
