use crate::error::Error;
use crate::marketplace::holder::Filters;
use crate::rest::marketplace::WebFilter;
use crate::rest::{parse_address, respond_with_transaction, AppState};
use crate::Result;
use actix_web::{get, post, web, HttpResponse, Scope};
use cardano_serialization_lib::{AssetName, PolicyID};
use serde::{Deserialize, Serialize};

#[get("")]
async fn get_all_sales(
    data: web::Data<AppState>,
    query: web::Query<WebFilter>,
) -> Result<HttpResponse> {
    let filters = query.into_inner().into_filters()?;
    let sales = data
        .project
        .holder
        .get_nfts_for_sale(&data.pool, filters)
        .await?;
    Ok(HttpResponse::Ok().json(sales))
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Buy {
    buyer_address: String,
    policy_id: String,
    asset_name: String,
}

#[post("/buy")]
async fn buy_nft(buy_details: web::Json<Buy>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let buy_details = buy_details.into_inner();

    let buyer_address = parse_address(&buy_details.buyer_address)?;
    let policy_id = PolicyID::from_bytes(hex::decode(buy_details.policy_id)?)?;
    let asset_name = AssetName::new(buy_details.asset_name.into_bytes())?;

    let tx = data
        .project
        .buy(buyer_address, policy_id, asset_name, &data.pool)
        .await?;
    Ok(respond_with_transaction(&tx))
}

pub fn create_project_service() -> Scope {
    web::scope("/projects")
        .service(buy_nft)
        .service(get_all_sales)
}
