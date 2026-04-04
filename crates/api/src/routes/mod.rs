pub mod agent_jobs;
pub mod ai_profiles;
pub mod bootstrap;
pub mod companies;
pub mod hiring;
pub mod people;
pub mod products;
pub mod providers;
pub mod simulation;
pub mod tickets;
pub mod workspace_members;
pub mod workspaces;

use axum::{routing::get, Router};

use crate::state::AppState;

/// Build the versioned API router.
pub fn v1_router() -> Router<AppState> {
    Router::new()
        // System
        .route("/bootstrap", get(bootstrap::get_bootstrap))
        // AI providers meta (Phase 1: Ollama only)
        .route("/ai-providers", get(providers::list_providers))
        .route(
            "/ai-providers/test-connection",
            axum::routing::post(providers::test_connection),
        )
        // Companies
        .route(
            "/companies",
            get(companies::list_companies).post(companies::create_company),
        )
        .route(
            "/companies/:id",
            get(companies::get_company).patch(companies::update_company),
        )
        .route(
            "/companies/:id/complete-onboarding",
            axum::routing::post(companies::complete_onboarding),
        )
        // Simulation controls (Phase 4)
        .route(
            "/companies/:id/run",
            axum::routing::post(simulation::run_company),
        )
        .route(
            "/companies/:id/stop",
            axum::routing::post(simulation::stop_company),
        )
        .route(
            "/companies/:id/terminate",
            axum::routing::post(simulation::terminate_company),
        )
        // Agent jobs
        .route(
            "/companies/:id/agent-jobs",
            get(agent_jobs::list_agent_jobs),
        )
        .route(
            "/agent-jobs/:job_id/stream",
            get(agent_jobs::stream_job_events),
        )
        // Products (nested under company)
        .route(
            "/companies/:id/products",
            get(products::list_products).post(products::create_product),
        )
        .route(
            "/companies/:id/products/:product_id",
            get(products::get_product).patch(products::update_product),
        )
        // AI profiles (nested under company)
        .route(
            "/companies/:id/ai-profiles",
            get(ai_profiles::list_ai_profiles).post(ai_profiles::create_ai_profile),
        )
        .route(
            "/companies/:id/ai-profiles/:profile_id",
            get(ai_profiles::get_ai_profile).patch(ai_profiles::update_ai_profile),
        )
        // People (nested under company)
        .route(
            "/companies/:id/people",
            get(people::list_people).post(people::create_person),
        )
        .route(
            "/companies/:id/people/:person_id",
            get(people::get_person)
                .patch(people::update_person)
                .delete(people::delete_person),
        )
        .route(
            "/companies/:id/people/:person_id/reporting-line",
            axum::routing::patch(people::update_reporting_line),
        )
        // Org chart (Phase 3.5)
        .route(
            "/companies/:id/org-chart",
            get(people::get_org_chart),
        )
        // Hiring proposals (Phase 3)
        .route(
            "/companies/:id/hiring-proposals",
            get(hiring::list_proposals).post(hiring::create_proposal),
        )
        .route(
            "/companies/:id/hiring-proposals/:proposal_id",
            get(hiring::get_proposal),
        )
        .route(
            "/companies/:id/hiring-proposals/:proposal_id/accept",
            axum::routing::post(hiring::accept_proposal),
        )
        .route(
            "/companies/:id/hiring-proposals/:proposal_id/decline",
            axum::routing::post(hiring::decline_proposal),
        )
        // Workspaces (nested under company)
        .route(
            "/companies/:id/workspaces",
            get(workspaces::list_workspaces).post(workspaces::create_workspace),
        )
        .route(
            "/companies/:id/workspaces/:workspace_id",
            get(workspaces::get_workspace)
                .patch(workspaces::update_workspace)
                .delete(workspaces::delete_workspace),
        )
        // Tickets (nested under workspace)
        .route(
            "/companies/:id/workspaces/:workspace_id/tickets",
            get(tickets::list_tickets).post(tickets::create_ticket),
        )
        .route(
            "/companies/:id/workspaces/:workspace_id/tickets/:ticket_id",
            get(tickets::get_ticket)
                .patch(tickets::update_ticket)
                .delete(tickets::delete_ticket),
        )
        // Comments (nested under ticket)
        .route(
            "/companies/:id/workspaces/:workspace_id/tickets/:ticket_id/comments",
            get(tickets::list_comments).post(tickets::create_comment),
        )
        // Agent runs for a ticket (Phase 4)
        .route(
            "/companies/:id/workspaces/:workspace_id/tickets/:ticket_id/run-agent",
            axum::routing::post(agent_jobs::enqueue_ticket_run),
        )
        .route(
            "/companies/:id/workspaces/:workspace_id/tickets/:ticket_id/agent-runs",
            get(agent_jobs::list_ticket_agent_runs),
        )
        // Workspace members (team permissions)
        .route(
            "/companies/:id/workspaces/:workspace_id/members",
            get(workspace_members::list_workspace_members)
                .post(workspace_members::add_workspace_member),
        )
        .route(
            "/companies/:id/workspaces/:workspace_id/members/:person_id",
            axum::routing::delete(workspace_members::remove_workspace_member),
        )
}
