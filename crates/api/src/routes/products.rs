use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use domain::{CreateProductInput, Product, UpdateProductInput};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

/// `GET /v1/companies/:id/products`
pub async fn list_products(
    State(state): State<AppState>,
    Path(company_id): Path<Uuid>,
) -> ApiResult<Json<Vec<Product>>> {
    // Verify company exists.
    db::company::get_company(&state.pool, company_id)
        .await?
        .ok_or(ApiError::NotFound)?;

    let products = db::product::list_products(&state.pool, company_id).await?;
    Ok(Json(products))
}

/// `GET /v1/companies/:id/products/:product_id`
pub async fn get_product(
    State(state): State<AppState>,
    Path((company_id, product_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Product>> {
    let product = db::product::get_product(&state.pool, product_id)
        .await?
        .ok_or(ApiError::NotFound)?;

    if product.company_id != company_id {
        return Err(ApiError::NotFound);
    }

    Ok(Json(product))
}

/// `POST /v1/companies/:id/products`
pub async fn create_product(
    State(state): State<AppState>,
    Path(company_id): Path<Uuid>,
    Json(input): Json<CreateProductInput>,
) -> ApiResult<(StatusCode, Json<Product>)> {
    if input.name.trim().is_empty() {
        return Err(ApiError::BadRequest("product name is required".into()));
    }

    db::company::get_company(&state.pool, company_id)
        .await?
        .ok_or(ApiError::NotFound)?;

    let product = db::product::create_product(&state.pool, company_id, input).await?;
    Ok((StatusCode::CREATED, Json(product)))
}

/// `PATCH /v1/companies/:id/products/:product_id`
pub async fn update_product(
    State(state): State<AppState>,
    Path((company_id, product_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<UpdateProductInput>,
) -> ApiResult<Json<Product>> {
    let product = db::product::get_product(&state.pool, product_id)
        .await?
        .ok_or(ApiError::NotFound)?;

    if product.company_id != company_id {
        return Err(ApiError::NotFound);
    }

    let updated = db::product::update_product(&state.pool, product_id, input)
        .await?
        .ok_or(ApiError::NotFound)?;

    Ok(Json(updated))
}
