pub mod http_client;
pub mod json;
pub use json::JsonPriceSource;

use alloc::string::String;
use sp_runtime::FixedPointNumber;
use sp_std::vec::Vec;

/// Price source abstraction. Settings of price source stored in offchain local storage.
pub trait PriceSource<AssetId, AssetData>: Sized {
    const PRICE_SOURCE_TYPE: &'static str;

    fn new(assets_data: Vec<(AssetId, AssetData)>) -> Result<Self, &'static str>;

    /// Returns collection of (asset, price result)
    fn get_prices<F>(&self) -> Vec<(AssetId, Result<F, &'static str>)>
    where
        F: FixedPointNumber;
}

pub trait PriceSourcePeeker<AssetId, AssetData> {
    fn get_prices<F>(
        price_source_type: impl AsRef<str>,
        assets_data: &Vec<(AssetId, AssetData)>,
    ) -> Result<Vec<(AssetId, Result<F, &'static str>)>, Option<&'static str>>
    where
        F: FixedPointNumber;
}

pub struct PriceSourceStruct<P>(P);

impl<AssetId: Clone, AssetData: Clone, P> PriceSourcePeeker<AssetId, AssetData>
    for PriceSourceStruct<P>
where
    P: PriceSource<AssetId, AssetData>,
{
    fn get_prices<F>(
        price_source_type: impl AsRef<str>,
        assets_data: &Vec<(AssetId, AssetData)>,
    ) -> Result<Vec<(AssetId, Result<F, &'static str>)>, Option<&'static str>>
    where
        F: FixedPointNumber,
    {
        if price_source_type.as_ref() == P::PRICE_SOURCE_TYPE {
            let price_source = P::new(assets_data.clone()).map_err(Some)?;
            Ok(price_source.get_prices::<F>())
        } else {
            Err(None)
        }
    }
}

#[impl_trait_for_tuples::impl_for_tuples(5)]
impl<AssetId: Clone, AssetData: Clone> PriceSourcePeeker<AssetId, AssetData> for Tuple {
    fn get_prices<F>(
        price_source_type: impl AsRef<str>,
        assets_data: &Vec<(AssetId, AssetData)>,
    ) -> Result<Vec<(AssetId, Result<F, &'static str>)>, Option<&'static str>>
    where
        F: FixedPointNumber,
    {
        for_tuples!( #(
            match Tuple::get_prices::<F>(price_source_type.as_ref(), assets_data) {
                Err(None) => {},
                res => return res,
            }
        )* );

        Err(None)
    }
}
