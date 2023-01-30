use crate::coin::TransactionWitnessSetParams;
use crate::config::Config;
use crate::marketplace::holder::{MarketplaceHolder, SellMetadata};
use crate::{
    cardano_db_sync::{get_protocol_params, get_slot_number, query_user_address_utxo},
    coin::build_transaction_body,
    convert_to_testnet, Error, Result,
};
use cardano_serialization_lib::address::Address;
use cardano_serialization_lib::crypto::Vkeywitnesses;
use cardano_serialization_lib::utils::{
    hash_transaction, to_bignum, TransactionUnspentOutput, Value,
};
use cardano_serialization_lib::{
    AssetName, Assets, MultiAsset, PolicyID, Transaction, TransactionOutput, TransactionWitnessSet,
};
use sqlx::PgPool;

const ONE_HOUR: u32 = 3600;

#[derive(Clone)]
pub struct Projects {
    pub(crate) holder: MarketplaceHolder,
    revenue_address: Address,
}

impl Projects {
    pub fn from_config(config: &Config) -> Result<Projects> {
        let holder =
            MarketplaceHolder::from_key_file(&config.projects_private_key_file, config.is_testnet)?;

        let mut revenue_address = Address::from_bech32(&config.projects_revenue_address)?;

        if config.is_testnet {
            revenue_address = convert_to_testnet(revenue_address);
        }

        Ok(Self {
            holder,
            revenue_address,
        })
    }

    pub async fn buy(
        &self,
        buyer_address: Address,
        policy_id: PolicyID,
        asset_name: AssetName,
        pool: &PgPool,
    ) -> Result<Transaction> {
        let buyer_utxos = query_user_address_utxo(pool, &buyer_address).await?;
        let sell_metadata = self.get_sell_details(pool, &policy_id, &asset_name).await?;

        let holder_utxos = query_user_address_utxo(pool, &self.holder.address).await?;
        let (nft_utxo, _) = find_nft(holder_utxos, &policy_id, &asset_name)?;

        let (revenue_cut, seller_cut) = calculate_cuts(sell_metadata.price);

        let revenue_output =
            TransactionOutput::new(&self.revenue_address, &Value::new(&to_bignum(revenue_cut)));

        let seller_output = TransactionOutput::new(
            &sell_metadata.seller_address,
            &Value::new(&to_bignum(seller_cut)),
        );

        let mut nft = Value::new(&to_bignum(2_000_000));
        let multiasset = {
            let mut ma = MultiAsset::new();
            let mut assets = Assets::new();
            assets.insert(&asset_name, &to_bignum(1));
            ma.insert(&policy_id, &assets);
            ma
        };
        nft.set_multiasset(&multiasset);
        let buyer_nft_output = TransactionOutput::new(&buyer_address, &nft);

        let return_asset = {
            let mut nfts = nft_utxo.output().amount();
            let mut mas = nfts.multiasset().unwrap_or(MultiAsset::new());
            mas = mas.sub(&multiasset);
            mas
        };
        let return_value = {
            let mut val = nft_utxo.output().amount();
            val.set_multiasset(&return_asset);
            val
        };
        let return_output = TransactionOutput::new(&self.holder.address, &return_value);

        let outputs = vec![
            revenue_output,
            seller_output,
            buyer_nft_output,
            return_output,
        ];
        let inputs = vec![nft_utxo];

        let tx_witness_params = TransactionWitnessSetParams {
            vkey_count: 2,
            ..Default::default()
        };
        let slot = get_slot_number(pool).await?;
        let protocol_params = get_protocol_params(pool).await?;

        let aux_data = if return_asset.len() > 0 {
            Some(sell_metadata.create_sell_nft_metadata()?)
        } else {
            None
        };

        let tx_body = build_transaction_body(
            buyer_utxos,
            inputs,
            outputs,
            slot + ONE_HOUR,
            &protocol_params,
            None,
            None,
            &tx_witness_params,
            aux_data.clone(),
        )?;

        let tx_hash = hash_transaction(&tx_body);
        let vkey = self.holder.sign_transaction_hash(&tx_hash);
        let mut tx_witness_set = TransactionWitnessSet::new();
        let mut vkeys = Vkeywitnesses::new();
        vkeys.add(&vkey);
        tx_witness_set.set_vkeys(&vkeys);

        let tx = Transaction::new(&tx_body, &tx_witness_set, aux_data);
        Ok(tx)
    }

    async fn get_sell_details(
        &self,
        pool: &PgPool,
        policy_id: &PolicyID,
        asset_name: &AssetName,
    ) -> Result<SellMetadata> {
        self.holder
            .get_nft_details(pool, &policy_id, &asset_name)
            .await?
            .ok_or_else(|| Error::Message("No such NFT is for sale".to_string()))
    }
}

const ONE_ADA: u64 = 1_000_000;

fn calculate_cuts(price: u64) -> (u64, u64) {
    let revenue_cut = 1_500_000;
    // The seller put in 2 ADA as deposit
    let seller_cut = price - revenue_cut;
    (revenue_cut, seller_cut)
}

fn create_value_with_single_nft(policy_id: &PolicyID, asset_name: &AssetName) -> Value {
    let mut value = Value::new(&to_bignum(0));
    value.set_multiasset(&{
        let mut ma = MultiAsset::new();
        ma.insert(policy_id, &{
            let mut assets = Assets::new();
            assets.insert(asset_name, &to_bignum(1));
            assets
        });
        ma
    });
    value
}

pub fn find_nft(
    utxos: Vec<TransactionUnspentOutput>,
    policy_id: &PolicyID,
    asset_name: &AssetName,
) -> Result<(TransactionUnspentOutput, Vec<TransactionUnspentOutput>)> {
    let mut remaining_utxos = Vec::with_capacity(utxos.len());
    let mut nft_utxo = None;

    for utxo in utxos {
        if utxo
            .output()
            .amount()
            .multiasset()
            .and_then(|ma| ma.get(policy_id))
            .and_then(|assets| assets.get(asset_name))
            .is_some()
        {
            nft_utxo = Some(utxo);
        } else {
            remaining_utxos.push(utxo);
        }
    }

    nft_utxo
        .ok_or_else(|| Error::Message("No such NFT is for sale".to_string()))
        .map(|nft| (nft, remaining_utxos))
}
