pub mod ai_profiles;
pub mod bootstrap;
pub mod companies;
pub mod people;
pub mod products;
pub mod providers;

use axum::{routing::get, Router};

use crate::state::AppState;

/// Build the versioned API router.
pub fn v1_router() -> Router<AppState> {
    Router::new()
        // System
        .route("/bootstrap", get(bootstrap::get_bootstrap))
        // AI providers meta (Phase 1: Ollama only)
        .route(
            "/ai-providers",
            get(providers::list_providers),
        )
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
            get(people::get_person),
        )
}
