use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Company {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub onboarding_complete: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCompanyInput {
    pub name: String,
    /// Initial product to create atomically with the company.
    pub product: Option<CreateProductInline>,
}

/// Product fields inlined into company creation for the onboarding transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProductInline {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCompanyInput {
    pub name: Option<String>,
    pub onboarding_complete: Option<bool>,
}

/// Response returned by `GET /v1/bootstrap` so the UI knows
/// whether to show onboarding or the main app shell.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapStatus {
    pub onboarding_complete: bool,
    pub company_id: Option<Uuid>,
}
