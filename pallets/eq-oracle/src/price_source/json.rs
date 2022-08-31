use super::{http_client, PriceSource};
use crate::offchain_storage;
use crate::regex_offsets::{get_index_offsets, get_url_offset};
use alloc::string::String;
use serde_json as json;
use sp_arithmetic::FixedPointNumber;
use sp_std::vec::Vec;

use primitives::AsSymbol;
use utils::log;

/// Json price source. Gets prices for assets from setting "oracle::source_assets"
/// or for all assets if no settings specified. Also uses price_strategy from "oracle::source_assets"
/// if specifies. Price strategy define how to interpret value from source (price, reverse)
#[derive(Debug)]
pub struct JsonPriceSource<AssetId, AssetData> {
    /// Full query, containing url template and path to price in json
    /// example: json(https://ftx.com/api/markets/{$}/USD).result.price
    query: String,
    assets_data: Vec<(AssetId, AssetData)>,
}

impl<AssetId: AsSymbol, AssetData> JsonPriceSource<AssetId, AssetData> {
    /// Fetches a price for an asset from a URL source with the query
    fn fetch_price<F: FixedPointNumber>(
        asset: &AssetId,
        query: &str,
    ) -> Result<F, PriceSourceError> {
        let (start, end) = get_url_offset(query.as_bytes()).ok_or_else(|| {
            log::error!("Incorrect query format, can't parse. Query: {}", query);
            PriceSourceError::IncorrectQueryFormat
        })?;

        // regex is \(.+\)\.
        let url_template = &query[start + 1..end - 2];
        if !url_template.contains("{$}") {
            log::error!(
                "Incorrect query format, doesn't have {{$}}. Query: {}, url template: {:?}.",
                query,
                url_template
            );
            frame_support::fail!(PriceSourceError::WrongUrlPattern)
        }

        let path_template = &query[end..];
        let (url, path) = asset.get_url(url_template, path_template)?;
        let s = http_client::get(url.as_str()).map_err(|e| {
            let e = match e {
                sp_runtime::offchain::http::Error::DeadlineReached => "DEADLINE",
                sp_runtime::offchain::http::Error::IoError => "IO_ERROR",
                sp_runtime::offchain::http::Error::Unknown => "UNKNOWN",
            };
            log::error!("Http GET {:?} error: {:?}", url, e);
            PriceSourceError::HttpError
        })?;

        Self::fetch_price_from_json::<F>(s, path.as_str())
    }

    /// Fetches a price from a collected JSON
    pub(crate) fn fetch_price_from_json<F: FixedPointNumber>(
        body: String,
        path: &str,
    ) -> Result<F, PriceSourceError> {
        let mut val: &json::Value = &json::from_str(&body).map_err(|_| {
            log::error!(
                "Cannot deserialize an instance from a string to JSON. String: {:?}.",
                body
            );
            PriceSourceError::DeserializationError
        })?;

        let indices = path.split(".");
        for index in indices {
            let offsets = get_index_offsets(index.as_bytes());
            if offsets.len() == 0 {
                val = val.get(index).ok_or_else(|| {
                    log::error!(
                        "Couldn't access a value in a map. Json: {:?}, index: {:?}.",
                        val,
                        index
                    );
                    PriceSourceError::JsonParseError
                })?;
            } else {
                // arrays
                for (start, end) in offsets {
                    if start != 0 {
                        val = val.get(&index[..start]).ok_or_else(|| {
                            log::error!(
                                "Couldn't access an element of an array. Json: {:?}, index: {:?}.",
                                val,
                                &index[..start]
                            );
                            PriceSourceError::JsonParseError
                        })?;
                    }

                    let i = &index[start + 1..end - 1]
                        .parse::<usize>()
                        .expect("Expect a number as array index");

                    val = val.get(i).ok_or_else(|| {
                        log::error!(
                            "Couldn't access an element of an array. Json: {:?}, index: {:?}.",
                            val,
                            i
                        );
                        PriceSourceError::JsonParseError
                    })?;
                }
            }
        }

        let maybe_price = match val {
            json::Value::Number(v) => v.as_f64(),
            json::Value::String(v) => v.parse::<f64>().ok(),
            _ => {
                log::error!(
                    "Value received from json not number or string. Value: {:?}.",
                    val
                );
                frame_support::fail!(PriceSourceError::JsonValueNotANumber)
            }
        };

        let price = maybe_price.ok_or_else(|| {
            log::error!("Couldn't get value as f64. Value: {:?}.", val);
            PriceSourceError::JsonPriceConversionError
        })?;

        const MAX_ACCURACY: u128 = 1_000_000_000_000;
        Ok(F::saturating_from_rational(
            (price * MAX_ACCURACY as f64) as u128,
            MAX_ACCURACY,
        ))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum PriceSourceError {
    HttpError,
    WrongUrlPattern,
    NoQueryStringInStorage,
    IncorrectQueryFormat,
    DeserializationError,
    JsonParseError,
    JsonValueNotANumber,
    JsonPriceConversionError,
    UnknownPriceStrategy,
    Symbol,
}

impl From<PriceSourceError> for &'static str {
    fn from(error: PriceSourceError) -> Self {
        match error {
            PriceSourceError::HttpError => "Http error",
            PriceSourceError::WrongUrlPattern => "Wrong url pattern",
            PriceSourceError::NoQueryStringInStorage => "No query string in storage",
            PriceSourceError::IncorrectQueryFormat => "Incorrect query format",
            PriceSourceError::DeserializationError => "Deserialization error",
            PriceSourceError::JsonParseError => "Json parse error",
            PriceSourceError::JsonValueNotANumber => "Json value not a number",
            PriceSourceError::JsonPriceConversionError => "Json price conversion error",
            PriceSourceError::UnknownPriceStrategy => "Unknown price strategy",
            PriceSourceError::Symbol => "Symbol",
        }
    }
}

impl<AssetId: AsSymbol + Clone, AssetData> PriceSource<AssetId, AssetData>
    for JsonPriceSource<AssetId, AssetData>
{
    const PRICE_SOURCE_TYPE: &'static str = "custom";

    fn new(assets_data: Vec<(AssetId, AssetData)>) -> Result<Self, &'static str> {
        Ok(JsonPriceSource {
            query: offchain_storage::get_query().ok_or("No query string in storage")?,
            assets_data,
        })
    }

    fn get_prices<F>(&self) -> Vec<(AssetId, Result<F, &'static str>)>
    where
        F: FixedPointNumber,
    {
        let asset_settings = offchain_storage::get_asset_settings();
        let empty_settings = asset_settings.is_empty();
        let mut asset_prices: Vec<(AssetId, Result<F, &'static str>)> =
            Vec::with_capacity(self.assets_data.len());

        for asset in &self.assets_data {
            let (asset, _) = asset;

            // If specified, do not fetch non available currencies
            let price = if empty_settings {
                offchain_storage::clear_asset_settings();
                Self::fetch_price(asset, &self.query)
            } else {
                if let Some(symbol) = asset.get_symbol() {
                    match asset_settings.get(&symbol) {
                        Some(price_strategy) => Self::fetch_price::<F>(&asset, &self.query)
                            .and_then(|price| match price_strategy.as_str() {
                                "price" => Ok(price),
                                "reverse" => {
                                    Ok(price.reciprocal().expect("Price should be more than 0"))
                                }
                                _ => Err(PriceSourceError::UnknownPriceStrategy),
                            }),
                        _ => continue, // skip asset
                    }
                } else {
                    Err(PriceSourceError::Symbol)
                }
            };

            if let Err(err) = &price {
                log::error!(
                    "{}:{} Custom price source return error. Asset: {:?}, error: {:?}",
                    file!(),
                    line!(),
                    asset.get_symbol(),
                    err,
                );
            };
            asset_prices.push((asset.clone(), price.map_err(From::from)));
        }

        asset_prices
    }
}

/// Getter of a URL for an asset price
pub(crate) trait WithUrl {
    /// Gets a URL and JSON path for an asset price
    fn get_url(
        &self,
        url_template: &str,
        path_template: &str,
    ) -> Result<(String, String), PriceSourceError>;
}

impl<AssetId: AsSymbol> WithUrl for AssetId {
    /// Gets a URL
    ///
    /// Put self string identifier in `url_template` and `path_template` instead of `{$}`
    fn get_url(
        &self,
        url_template: &str,
        path_template: &str,
    ) -> Result<(String, String), PriceSourceError> {
        let is_upper_case = url_template.find("USD").is_some();
        let symbol = {
            let is_kraken = url_template.contains("api.kraken.com");
            let symbol = self
                .get_query_symbol(is_kraken)
                .ok_or(PriceSourceError::Symbol)?;

            if is_upper_case {
                symbol.to_uppercase()
            } else {
                symbol.to_lowercase()
            }
        };

        Ok((
            url_template.replace("{$}", &symbol),
            path_template.replace("{$}", &symbol),
        ))
    }
}

// /// Returns a symbolic ticker
// impl AsQuerySymbol for Asset {
//     fn get_symbol(&self, is_kraken: bool) -> Result<String, PriceSourceError> {
//         match (is_kraken, str_asset!(self)) {
//             (true, Ok("eth")) => Ok("xethz".into()),
//             (true, Ok("btc")) => Ok("xxbtz".into()),
//             (true, Ok("kbtc")) => Ok("xxbtz".into()),
//             (true, Ok("ibtc")) => Ok("xxbtz".into()),
//             (true, Ok("usdt")) => Ok("usdtz".into()),
//             (_, Ok("kbtc")) => Ok("btc".into()),
//             (_, Ok("ibtc")) => Ok("btc".into()),
//             (_, Ok(s)) => Ok(s.into()),
//             (_, Err(_)) => Err(PriceSourceError::Symbol),
//         }
//     }
// }
