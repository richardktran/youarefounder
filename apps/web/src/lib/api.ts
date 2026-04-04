/**
 * Typed API client.
 * In development, Next.js rewrites `/api/*` to `http://127.0.0.1:3001/v1/*`.
 * In production you'd point this at the same-origin backend.
 */
import axios from "axios";

export const apiClient = axios.create({
  baseURL: "/api",
  headers: { "Content-Type": "application/json" },
});

// ─── Types (mirroring Rust domain structs) ────────────────────────────────────

export interface BootstrapStatus {
  onboarding_complete: boolean;
  company_id: string | null;
}

export interface Company {
  id: string;
  name: string;
  slug: string;
  onboarding_complete: boolean;
  created_at: string;
  updated_at: string;
}

export type ProductStatus =
  | "idea"
  | "discovery"
  | "spec"
  | "building"
  | "launched";

export interface Product {
  id: string;
  company_id: string;
  name: string;
  description: string | null;
  status: ProductStatus;
  created_at: string;
  updated_at: string;
}

export interface AiProfile {
  id: string;
  company_id: string;
  display_name: string | null;
  provider_kind: string;
  model_id: string;
  provider_config: Record<string, unknown>;
  default_temperature: number | null;
  default_max_tokens: number | null;
  created_at: string;
  updated_at: string;
}

export type PersonKind = "human_founder" | "ai_agent";
export type RoleType = "co_founder" | "ceo" | "cto" | "specialist";

export interface Person {
  id: string;
  company_id: string;
  kind: PersonKind;
  display_name: string;
  role_type: RoleType;
  specialty: string | null;
  ai_profile_id: string | null;
  created_at: string;
  updated_at: string;
}

export interface ProviderConfigField {
  key: string;
  label: string;
  field_type: "text" | "url" | "password";
  required: boolean;
  default_value?: string;
  placeholder?: string;
}

export interface ProviderInfo {
  kind: string;
  display_name: string;
  config_fields: ProviderConfigField[];
}

export interface TestConnectionResult {
  ok: boolean;
  error: string | null;
}

// ─── API functions ─────────────────────────────────────────────────────────────

export async function getBootstrap(): Promise<BootstrapStatus> {
  const { data } = await apiClient.get<BootstrapStatus>("/bootstrap");
  return data;
}

export async function listCompanies(): Promise<Company[]> {
  const { data } = await apiClient.get<Company[]>("/companies");
  return data;
}

export async function getCompany(id: string): Promise<Company> {
  const { data } = await apiClient.get<Company>(`/companies/${id}`);
  return data;
}

export async function createCompany(input: {
  name: string;
  product?: { name: string; description?: string };
}): Promise<Company> {
  const { data } = await apiClient.post<Company>("/companies", input);
  return data;
}

export async function updateCompany(
  id: string,
  input: Partial<{ name: string; onboarding_complete: boolean }>
): Promise<Company> {
  const { data } = await apiClient.patch<Company>(`/companies/${id}`, input);
  return data;
}

export async function completeOnboarding(companyId: string): Promise<Company> {
  const { data } = await apiClient.post<Company>(
    `/companies/${companyId}/complete-onboarding`
  );
  return data;
}

export async function listProducts(companyId: string): Promise<Product[]> {
  const { data } = await apiClient.get<Product[]>(
    `/companies/${companyId}/products`
  );
  return data;
}

export async function createProduct(
  companyId: string,
  input: { name: string; description?: string }
): Promise<Product> {
  const { data } = await apiClient.post<Product>(
    `/companies/${companyId}/products`,
    input
  );
  return data;
}

export async function updateProduct(
  companyId: string,
  productId: string,
  input: Partial<{ name: string; description: string; status: ProductStatus }>
): Promise<Product> {
  const { data } = await apiClient.patch<Product>(
    `/companies/${companyId}/products/${productId}`,
    input
  );
  return data;
}

// ─── AI Providers ──────────────────────────────────────────────────────────────

export async function listAiProviders(): Promise<{ providers: ProviderInfo[] }> {
  const { data } = await apiClient.get<{ providers: ProviderInfo[] }>(
    "/ai-providers"
  );
  return data;
}

export async function testConnection(input: {
  provider_kind: string;
  provider_config: Record<string, unknown>;
  model_id?: string;
}): Promise<TestConnectionResult> {
  const { data } = await apiClient.post<TestConnectionResult>(
    "/ai-providers/test-connection",
    input
  );
  return data;
}

// ─── AI Profiles ──────────────────────────────────────────────────────────────

export async function listAiProfiles(companyId: string): Promise<AiProfile[]> {
  const { data } = await apiClient.get<AiProfile[]>(
    `/companies/${companyId}/ai-profiles`
  );
  return data;
}

export async function createAiProfile(
  companyId: string,
  input: {
    provider_kind: string;
    model_id: string;
    provider_config?: Record<string, unknown>;
    display_name?: string;
    default_temperature?: number;
    default_max_tokens?: number;
  }
): Promise<AiProfile> {
  const { data } = await apiClient.post<AiProfile>(
    `/companies/${companyId}/ai-profiles`,
    input
  );
  return data;
}

// ─── People ───────────────────────────────────────────────────────────────────

export async function listPeople(companyId: string): Promise<Person[]> {
  const { data } = await apiClient.get<Person[]>(
    `/companies/${companyId}/people`
  );
  return data;
}

export async function createPerson(
  companyId: string,
  input: {
    kind: PersonKind;
    display_name: string;
    role_type: RoleType;
    specialty?: string;
    ai_profile_id?: string;
  }
): Promise<Person> {
  const { data } = await apiClient.post<Person>(
    `/companies/${companyId}/people`,
    input
  );
  return data;
}
