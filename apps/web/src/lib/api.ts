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

export type RunState = "stopped" | "running" | "terminated";

export interface Company {
  id: string;
  name: string;
  slug: string;
  onboarding_complete: boolean;
  /** Phase 4: simulation control state. */
  run_state: RunState;
  /** Maximum number of agent jobs that may run concurrently (default 1). */
  max_concurrent_agents: number;
  /** Founder-written standing instructions for ticket work; injected into every agent run. */
  agent_ticket_memory: string | null;
  /** Founder-written standing instructions for decisions / escalations; injected into every agent run. */
  agent_decision_memory: string | null;
  created_at: string;
  updated_at: string;
}

export type JobStatus = "pending" | "running" | "succeeded" | "failed";
export type JobKind = "agent_ticket_run" | "index_repository";

export interface AgentJob {
  id: string;
  kind: JobKind;
  company_id: string;
  payload: Record<string, unknown>;
  status: JobStatus;
  run_at: string;
  started_at: string | null;
  completed_at: string | null;
  error: string | null;
  attempts: number;
  max_attempts: number;
  created_at: string;
}

export interface AgentRun {
  id: string;
  agent_job_id: string;
  ticket_id: string;
  person_id: string;
  prompt_tokens: number | null;
  completion_tokens: number | null;
  raw_response: string | null;
  actions_applied: unknown[];
  error: string | null;
  created_at: string;
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
  /** Phase 3.5: nullable FK to the manager (reports_to_person_id). */
  reports_to_person_id: string | null;
  created_at: string;
  updated_at: string;
}

// ─── Org chart ────────────────────────────────────────────────────────────────

export interface OrgNode {
  id: string;
  display_name: string;
  role_type: string;
  specialty: string | null;
  kind: string;
  reports_to_person_id: string | null;
}

// ─── Hiring proposals ─────────────────────────────────────────────────────────

export type ProposalStatus =
  | "pending_founder"
  | "accepted"
  | "declined"
  | "withdrawn";

export interface HiringProposal {
  id: string;
  company_id: string;
  proposed_by_person_id: string | null;
  employee_display_name: string;
  role_type: RoleType;
  specialty: string | null;
  ai_profile_id: string | null;
  rationale: string | null;
  scope_of_work: string | null;
  status: ProposalStatus;
  founder_response_text: string | null;
  /** Populated after accept — the newly created Person id. */
  created_person_id: string | null;
  /** Workspaces the hire joins on accept (co-founder hires join all regardless). */
  workspace_ids: string[] | null;
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

/** Must match `RESET_INSTALL_CONFIRM_PHRASE` in `crates/domain/src/company.rs`. */
export const RESET_INSTALL_CONFIRM_PHRASE = "DELETE ALL LOCAL DATA" as const;

export async function resetInstall(confirmPhrase: string): Promise<void> {
  await apiClient.post("/system/reset-install", {
    confirm_phrase: confirmPhrase,
  });
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
  input: Partial<{
    name: string;
    onboarding_complete: boolean;
    max_concurrent_agents: number;
    agent_ticket_memory: string;
    agent_decision_memory: string;
  }>
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

export async function updateAiProfile(
  companyId: string,
  profileId: string,
  input: Partial<{
    display_name: string | null;
    model_id: string;
    provider_config: Record<string, unknown>;
    default_temperature: number | null;
    default_max_tokens: number | null;
  }>
): Promise<AiProfile> {
  const { data } = await apiClient.patch<AiProfile>(
    `/companies/${companyId}/ai-profiles/${profileId}`,
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

export async function updatePerson(
  companyId: string,
  personId: string,
  input: Partial<{
    display_name: string;
    role_type: RoleType;
    specialty: string | null;
    ai_profile_id: string | null;
  }>
): Promise<Person> {
  const { data } = await apiClient.patch<Person>(
    `/companies/${companyId}/people/${personId}`,
    input
  );
  return data;
}

export async function deletePerson(
  companyId: string,
  personId: string
): Promise<void> {
  await apiClient.delete(`/companies/${companyId}/people/${personId}`);
}

// ─── Workspace Members ────────────────────────────────────────────────────────

export type WorkspaceMemberRole = "member" | "lead";

export interface WorkspaceMember {
  id: string;
  workspace_id: string;
  person_id: string;
  role: WorkspaceMemberRole;
  created_at: string;
  display_name: string;
  person_kind: PersonKind;
  role_type: string;
  specialty: string | null;
  ai_profile_id: string | null;
}

export async function listWorkspaceMembers(
  companyId: string,
  workspaceId: string
): Promise<WorkspaceMember[]> {
  const { data } = await apiClient.get<WorkspaceMember[]>(
    `/companies/${companyId}/workspaces/${workspaceId}/members`
  );
  return data;
}

export async function addWorkspaceMember(
  companyId: string,
  workspaceId: string,
  input: { person_id: string; role?: WorkspaceMemberRole }
): Promise<WorkspaceMember> {
  const { data } = await apiClient.post<WorkspaceMember>(
    `/companies/${companyId}/workspaces/${workspaceId}/members`,
    input
  );
  return data;
}

export async function removeWorkspaceMember(
  companyId: string,
  workspaceId: string,
  personId: string
): Promise<void> {
  await apiClient.delete(
    `/companies/${companyId}/workspaces/${workspaceId}/members/${personId}`
  );
}

// ─── Workspaces ───────────────────────────────────────────────────────────────

export interface Workspace {
  id: string;
  company_id: string;
  name: string;
  slug: string;
  description: string | null;
  created_at: string;
  updated_at: string;
}

export async function listWorkspaces(companyId: string): Promise<Workspace[]> {
  const { data } = await apiClient.get<Workspace[]>(
    `/companies/${companyId}/workspaces`
  );
  return data;
}

export async function getWorkspace(
  companyId: string,
  workspaceId: string
): Promise<Workspace> {
  const { data } = await apiClient.get<Workspace>(
    `/companies/${companyId}/workspaces/${workspaceId}`
  );
  return data;
}

export async function createWorkspace(
  companyId: string,
  input: { name: string; slug?: string; description?: string }
): Promise<Workspace> {
  const { data } = await apiClient.post<Workspace>(
    `/companies/${companyId}/workspaces`,
    input
  );
  return data;
}

export async function updateWorkspace(
  companyId: string,
  workspaceId: string,
  input: Partial<{ name: string; description: string }>
): Promise<Workspace> {
  const { data } = await apiClient.patch<Workspace>(
    `/companies/${companyId}/workspaces/${workspaceId}`,
    input
  );
  return data;
}

export async function deleteWorkspace(
  companyId: string,
  workspaceId: string
): Promise<void> {
  await apiClient.delete(
    `/companies/${companyId}/workspaces/${workspaceId}`
  );
}

// ─── Tickets ──────────────────────────────────────────────────────────────────

export type TicketStatus =
  | "backlog"
  | "todo"
  | "in_progress"
  | "blocked"
  | "done"
  | "cancelled";

export type TicketType = "task" | "epic" | "research";
export type TicketPriority = "low" | "medium" | "high";

export interface Ticket {
  id: string;
  workspace_id: string;
  title: string;
  description: string | null;
  /** Checklist for marking this ticket done (aligns agents and humans). */
  definition_of_done: string | null;
  /** Founder-only instructions for this ticket; injected into agent prompts. */
  founder_memory: string | null;
  /** Short completion note when done (optional; used in cross-ticket snapshots). */
  outcome_summary: string | null;
  ticket_type: TicketType;
  status: TicketStatus;
  priority: TicketPriority;
  assignee_person_id: string | null;
  parent_ticket_id: string | null;
  created_at: string;
  updated_at: string;
}

export interface TicketComment {
  id: string;
  ticket_id: string;
  body: string;
  author_person_id: string | null;
  created_at: string;
}

export async function listTickets(
  companyId: string,
  workspaceId: string,
  options?: { rootsOnly?: boolean; parentTicketId?: string }
): Promise<Ticket[]> {
  const params = new URLSearchParams();
  if (options?.rootsOnly) params.set("roots_only", "true");
  if (options?.parentTicketId)
    params.set("parent_ticket_id", options.parentTicketId);
  const q = params.toString();
  const { data } = await apiClient.get<Ticket[]>(
    `/companies/${companyId}/workspaces/${workspaceId}/tickets${q ? `?${q}` : ""}`
  );
  return data;
}

export async function getTicket(
  companyId: string,
  workspaceId: string,
  ticketId: string
): Promise<Ticket> {
  const { data } = await apiClient.get<Ticket>(
    `/companies/${companyId}/workspaces/${workspaceId}/tickets/${ticketId}`
  );
  return data;
}

export async function createTicket(
  companyId: string,
  workspaceId: string,
  input: {
    title: string;
    description?: string;
    definition_of_done?: string;
    founder_memory?: string;
    outcome_summary?: string;
    ticket_type?: TicketType;
    status?: TicketStatus;
    priority?: TicketPriority;
    assignee_person_id?: string;
    parent_ticket_id?: string;
  }
): Promise<Ticket> {
  const { data } = await apiClient.post<Ticket>(
    `/companies/${companyId}/workspaces/${workspaceId}/tickets`,
    input
  );
  return data;
}

export async function updateTicket(
  companyId: string,
  workspaceId: string,
  ticketId: string,
  input: Partial<{
    title: string;
    description: string;
    definition_of_done: string | null;
    founder_memory: string | null;
    outcome_summary: string | null;
    ticket_type: TicketType;
    status: TicketStatus;
    priority: TicketPriority;
    assignee_person_id: string | null;
    parent_ticket_id: string | null;
  }>
): Promise<Ticket> {
  const { data } = await apiClient.patch<Ticket>(
    `/companies/${companyId}/workspaces/${workspaceId}/tickets/${ticketId}`,
    input
  );
  return data;
}

export async function listComments(
  companyId: string,
  workspaceId: string,
  ticketId: string
): Promise<TicketComment[]> {
  const { data } = await apiClient.get<TicketComment[]>(
    `/companies/${companyId}/workspaces/${workspaceId}/tickets/${ticketId}/comments`
  );
  return data;
}

export async function createComment(
  companyId: string,
  workspaceId: string,
  ticketId: string,
  input: { body: string; author_person_id?: string }
): Promise<TicketComment> {
  const { data } = await apiClient.post<TicketComment>(
    `/companies/${companyId}/workspaces/${workspaceId}/tickets/${ticketId}/comments`,
    input
  );
  return data;
}

// ─── Product brain & ticket references ────────────────────────────────────────

export type ProductBrainPendingStatus = "pending" | "rejected" | "promoted";

export interface ProductBrainEntry {
  id: string;
  company_id: string;
  workspace_id: string | null;
  body: string;
  source_ticket_id: string | null;
  created_at: string;
}

export interface ProductBrainPending {
  id: string;
  company_id: string;
  workspace_id: string | null;
  body: string;
  source_ticket_id: string | null;
  status: ProductBrainPendingStatus;
  proposed_at: string;
  reviewed_at: string | null;
}

export interface TicketReference {
  from_ticket_id: string;
  to_ticket_id: string;
  note: string | null;
  created_at: string;
}

export async function listProductBrainEntries(
  companyId: string
): Promise<ProductBrainEntry[]> {
  const { data } = await apiClient.get<ProductBrainEntry[]>(
    `/companies/${companyId}/product-brain/entries`
  );
  return data;
}

export async function listProductBrainPending(
  companyId: string
): Promise<ProductBrainPending[]> {
  const { data } = await apiClient.get<ProductBrainPending[]>(
    `/companies/${companyId}/product-brain/pending`
  );
  return data;
}

export async function approveProductBrainPending(
  companyId: string,
  pendingId: string,
  input?: { body?: string | null }
): Promise<ProductBrainEntry> {
  const { data } = await apiClient.post<ProductBrainEntry>(
    `/companies/${companyId}/product-brain/pending/${pendingId}/approve`,
    input ?? {}
  );
  return data;
}

export async function rejectProductBrainPending(
  companyId: string,
  pendingId: string
): Promise<void> {
  await apiClient.post(
    `/companies/${companyId}/product-brain/pending/${pendingId}/reject`
  );
}

export async function listTicketReferences(
  companyId: string,
  workspaceId: string,
  ticketId: string
): Promise<TicketReference[]> {
  const { data } = await apiClient.get<TicketReference[]>(
    `/companies/${companyId}/workspaces/${workspaceId}/tickets/${ticketId}/references`
  );
  return data;
}

export async function createTicketReference(
  companyId: string,
  workspaceId: string,
  ticketId: string,
  input: { to_ticket_id: string; note?: string | null }
): Promise<void> {
  await apiClient.post(
    `/companies/${companyId}/workspaces/${workspaceId}/tickets/${ticketId}/references`,
    input
  );
}

export async function deleteTicketReference(
  companyId: string,
  workspaceId: string,
  ticketId: string,
  toTicketId: string
): Promise<void> {
  await apiClient.delete(
    `/companies/${companyId}/workspaces/${workspaceId}/tickets/${ticketId}/references/${toTicketId}`
  );
}

// ─── Org chart ────────────────────────────────────────────────────────────────

export async function getOrgChart(companyId: string): Promise<OrgNode[]> {
  const { data } = await apiClient.get<OrgNode[]>(
    `/companies/${companyId}/org-chart`
  );
  return data;
}

export async function updateReportingLine(
  companyId: string,
  personId: string,
  reportsToPersonId: string | null
): Promise<Person> {
  const { data } = await apiClient.patch<Person>(
    `/companies/${companyId}/people/${personId}/reporting-line`,
    { reports_to_person_id: reportsToPersonId }
  );
  return data;
}

// ─── Hiring proposals ─────────────────────────────────────────────────────────

export async function listHiringProposals(
  companyId: string,
  status?: ProposalStatus
): Promise<HiringProposal[]> {
  const { data } = await apiClient.get<HiringProposal[]>(
    `/companies/${companyId}/hiring-proposals`,
    { params: status ? { status } : undefined }
  );
  return data;
}

export async function getHiringProposal(
  companyId: string,
  proposalId: string
): Promise<HiringProposal> {
  const { data } = await apiClient.get<HiringProposal>(
    `/companies/${companyId}/hiring-proposals/${proposalId}`
  );
  return data;
}

export async function createHiringProposal(
  companyId: string,
  input: {
    employee_display_name: string;
    role_type: RoleType;
    specialty?: string;
    ai_profile_id?: string;
    rationale?: string;
    scope_of_work?: string;
    proposed_by_person_id?: string;
    workspace_ids?: string[];
  }
): Promise<HiringProposal> {
  const { data } = await apiClient.post<HiringProposal>(
    `/companies/${companyId}/hiring-proposals`,
    input
  );
  return data;
}

export async function acceptHiringProposal(
  companyId: string,
  proposalId: string,
  founderNote?: string
): Promise<HiringProposal> {
  const { data } = await apiClient.post<HiringProposal>(
    `/companies/${companyId}/hiring-proposals/${proposalId}/accept`,
    founderNote ? { founder_response_text: founderNote } : {}
  );
  return data;
}

export async function declineHiringProposal(
  companyId: string,
  proposalId: string,
  reason: string
): Promise<HiringProposal> {
  const { data } = await apiClient.post<HiringProposal>(
    `/companies/${companyId}/hiring-proposals/${proposalId}/decline`,
    { founder_response_text: reason }
  );
  return data;
}

export async function deleteHiringProposal(
  companyId: string,
  proposalId: string
): Promise<void> {
  await apiClient.delete(
    `/companies/${companyId}/hiring-proposals/${proposalId}`
  );
}

// ─── Decision requests (Phase 6) ─────────────────────────────────────────────

export type DecisionStatus = "pending_founder" | "answered";

export interface DecisionRequest {
  id: string;
  company_id: string;
  workspace_id: string;
  ticket_id: string;
  raised_by_person_id: string | null;
  question: string;
  context_note: string | null;
  status: DecisionStatus;
  founder_answer: string | null;
  created_at: string;
  updated_at: string;
}

export async function listDecisionRequests(
  companyId: string,
  status?: DecisionStatus
): Promise<DecisionRequest[]> {
  const { data } = await apiClient.get<DecisionRequest[]>(
    `/companies/${companyId}/decision-requests`,
    { params: status ? { status } : undefined }
  );
  return data;
}

export async function getDecisionRequest(
  companyId: string,
  decisionId: string
): Promise<DecisionRequest> {
  const { data } = await apiClient.get<DecisionRequest>(
    `/companies/${companyId}/decision-requests/${decisionId}`
  );
  return data;
}

export async function answerDecisionRequest(
  companyId: string,
  decisionId: string,
  founderAnswer: string
): Promise<DecisionRequest> {
  const { data } = await apiClient.post<DecisionRequest>(
    `/companies/${companyId}/decision-requests/${decisionId}/answer`,
    { founder_answer: founderAnswer }
  );
  return data;
}

export async function deleteDecisionRequest(
  companyId: string,
  decisionId: string
): Promise<void> {
  await apiClient.delete(
    `/companies/${companyId}/decision-requests/${decisionId}`
  );
}

// ─── Simulation controls (Phase 4) ────────────────────────────────────────────

export async function runCompany(companyId: string): Promise<Company> {
  const { data } = await apiClient.post<Company>(
    `/companies/${companyId}/run`
  );
  return data;
}

export async function stopCompany(companyId: string): Promise<Company> {
  const { data } = await apiClient.post<Company>(
    `/companies/${companyId}/stop`
  );
  return data;
}

export async function terminateCompany(
  companyId: string,
  confirmName: string
): Promise<void> {
  await apiClient.post(`/companies/${companyId}/terminate`, {
    confirm_name: confirmName,
  });
}

// ─── Agent jobs ────────────────────────────────────────────────────────────────

export async function listAgentJobs(companyId: string): Promise<AgentJob[]> {
  const { data } = await apiClient.get<AgentJob[]>(
    `/companies/${companyId}/agent-jobs`
  );
  return data;
}

export async function enqueueTicketRun(
  companyId: string,
  workspaceId: string,
  ticketId: string,
  personId: string
): Promise<AgentJob> {
  const { data } = await apiClient.post<AgentJob>(
    `/companies/${companyId}/workspaces/${workspaceId}/tickets/${ticketId}/run-agent`,
    { person_id: personId }
  );
  return data;
}

export async function listTicketAgentRuns(
  companyId: string,
  workspaceId: string,
  ticketId: string
): Promise<AgentRun[]> {
  const { data } = await apiClient.get<AgentRun[]>(
    `/companies/${companyId}/workspaces/${workspaceId}/tickets/${ticketId}/agent-runs`
  );
  return data;
}
