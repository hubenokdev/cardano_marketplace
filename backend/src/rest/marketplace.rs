use crate::error::Error;
use crate::marketplace::holder::Filters;
use crate::rest::{parse_address, respond_with_transaction, AppState};
use crate::Result;
use actix_web::{get, post, web, HttpResponse, Scope};
use cardano_serialization_lib::{AssetName, PolicyID};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct WebFilter {
    page: Option<u32>,
    policy: Option<String>,
    asset_name: Option<String>,
}

impl WebFilter {
    pub(crate) fn into_filters(self) -> Result<Filters> {
        let page = self.page.unwrap_or(1);
        let policy = match self.policy {
            Some(ps) => Some(PolicyID::from_bytes(hex::decode(ps)?)?),
            None => None,
        };
        Ok(Filters {
            page,
            policy,
            asset_name: self.asset_name,
        })
    }
}

#[get("")]
async fn get_all_sales(
    data: web::Data<AppState>,
    query: web::Query<WebFilter>,
) -> Result<HttpResponse> {
    let filters = query.into_inner().into_filters()?;
    let sales = data
        .marketplace
        .holder
        .get_nfts_for_sale(&data.pool, filters)
        .await?;
    Ok(HttpResponse::Ok().json(sales))
}

#[get("/single/{transactionHash}")]
async fn get_single_sale(
    path: web::Path<String>,
    data: web::Data<AppState>,
) -> Result<HttpResponse> {
    let hash = path.into_inner();
    let sell_data = data
        .marketplace
        .holder
        .get_single_nft_for_sale(&data.pool, &hash)
        .await?;
    Ok(HttpResponse::Ok().json(sell_data))
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Sell {
    seller_address: String,
    policy_id: String,
    asset_name: String,
    price: u64,
}

#[post("/sell")]
async fn sell_nft(
    sell_details: web::Json<Sell>,
    data: web::Data<AppState>,
) -> Result<HttpResponse> {
    let sell_details = sell_details.into_inner();
    if sell_details.price < 5_000_000 {
        return Err(Error::Message(
            "Price cannot be less than 5 ADA".to_string(),
        ));
    }
    let seller_address = parse_address(&sell_details.seller_address)?;
    let policy_id = PolicyID::from_bytes(hex::decode(sell_details.policy_id)?)?;
    let asset_name = AssetName::new(sell_details.asset_name.into_bytes())?;
    let tx = data
        .marketplace
        .sell(
            seller_address,
            policy_id,
            asset_name,
            sell_details.price,
            &data.pool,
        )
        .await?;
    Ok(respond_with_transaction(&tx))
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
        .marketplace
        .buy(buyer_address, policy_id, asset_name, &data.pool)
        .await?;
    Ok(respond_with_transaction(&tx))
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Cancel {
    seller_address: String,
    policy_id: String,
    asset_name: String,
}

#[post("/cancel")]
async fn cancel_nft(
    cancel_details: web::Json<Cancel>,
    data: web::Data<AppState>,
) -> Result<HttpResponse> {
    let cancel_details = cancel_details.into_inner();

    let seller_address = parse_address(&cancel_details.seller_address)?;
    let policy_id = PolicyID::from_bytes(hex::decode(cancel_details.policy_id)?)?;
    let asset_name = AssetName::new(cancel_details.asset_name.into_bytes())?;

    let tx = data
        .marketplace
        .cancel(seller_address, policy_id, asset_name, &data.pool)
        .await?;
    Ok(respond_with_transaction(&tx))
}

pub fn create_marketplace_service() -> Scope {
    web::scope("/marketplace")
        .service(sell_nft)
        .service(buy_nft)
        .service(cancel_nft)
        .service(get_all_sales)
        .service(get_single_sale)
}
