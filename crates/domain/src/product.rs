use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProductStatus {
    Idea,
    Discovery,
    Spec,
    Building,
    Launched,
}

impl Default for ProductStatus {
    fn default() -> Self {
        Self::Idea
    }
}

impl std::fmt::Display for ProductStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idea => write!(f, "idea"),
            Self::Discovery => write!(f, "discovery"),
            Self::Spec => write!(f, "spec"),
            Self::Building => write!(f, "building"),
            Self::Launched => write!(f, "launched"),
        }
    }
}

impl std::str::FromStr for ProductStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "idea" => Ok(Self::Idea),
            "discovery" => Ok(Self::Discovery),
            "spec" => Ok(Self::Spec),
            "building" => Ok(Self::Building),
            "launched" => Ok(Self::Launched),
            other => Err(format!("unknown product status: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: Uuid,
    pub company_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: ProductStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProductInput {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProductInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<ProductStatus>,
}
