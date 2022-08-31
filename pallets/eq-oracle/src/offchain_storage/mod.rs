//! Offchain storage accessor

use alloc::string::{String, ToString};
use sp_io::offchain;
use sp_runtime::offchain::StorageKind;
use sp_std::collections::btree_map::BTreeMap;
use utils::offchain::get_local_storage_val;

mod storage_keys;

/// Gets query for price requests
pub fn get_query() -> Option<String> {
    get_local_storage_val(storage_keys::CUSTOM_QUERY)
}

/// Get counter
pub fn get_counter() -> Option<u32> {
    get_local_storage_val(storage_keys::COUNTER)
}

/// Get periodicity of price update
pub fn get_price_periodicity() -> Option<u32> {
    get_local_storage_val(storage_keys::PRICE_PERIODICITY)
}

/// Update counter value
pub fn set_counter(value: u32) {
    offchain::local_storage_set(
        StorageKind::PERSISTENT,
        storage_keys::COUNTER,
        value.to_string().as_bytes(),
    );
}

/// Get source type value
pub fn get_source_type() -> Option<String> {
    get_local_storage_val(storage_keys::RESOURCE_TYPE)
}

/// Returns collection of pairs (asset, price_strategy) available values for price_strategy is: "price", "reverse".
/// Price_strategy defines how to serve value from price source for particular asset.
/// If price_strategy == "price" then value recieved from price source is price.
/// if price_strategy == "reverse" then price = 1 / value
pub fn get_asset_settings() -> BTreeMap<String, String> {
    // List of assets that require price setting
    // example USDC:price, USDT:price, BTC:price, DAI:reverse
    // example USDC, USDT, DAI:reverse, BTC
    get_local_storage_val::<String>(storage_keys::SOURCE_ASSETS)
        .map(|assets_str| {
            assets_str
                .split(',')
                .map(|pair_str| {
                    let mut split_pair = pair_str.split(':');

                    (
                        split_pair.next().unwrap().trim().to_lowercase(),
                        split_pair
                            .next()
                            .map(|v| v.trim().to_lowercase())
                            .unwrap_or(String::from("price")),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn clear_asset_settings() {
    offchain::local_storage_clear(StorageKind::PERSISTENT, storage_keys::SOURCE_ASSETS);
}
