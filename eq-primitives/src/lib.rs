#![cfg_attr(not(feature = "std"), no_std)]
#![deny(warnings)]

extern crate alloc;
use core::convert::TryInto;

use alloc::string::String;
use codec::{Decode, Encode, MaxEncodedLen};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{DispatchError, FixedPointNumber, RuntimeDebug};
use sp_std::vec::Vec;

pub type AssetIdInnerType = u64;

#[derive(Debug)]
pub enum AssetError {
    DebtWeightNegative,
    DebtWeightMoreThanOne,
    AssetNameWrongLength,
    AssetNameWrongSymbols,
    PriceStepNegative,
}

#[derive(
    Clone,
    Copy,
    Default,
    RuntimeDebug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Decode,
    Encode,
    Hash,
    MaxEncodedLen,
    scale_info::TypeInfo,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Asset(pub AssetIdInnerType);

impl Asset {
    const SIZE_OF_ASSET_ID_INNER: usize = sp_std::mem::size_of::<AssetIdInnerType>();

    /// Creates new `Asset` instance from str
    pub fn from_bytes(mut asset: Vec<u8>) -> Result<Self, AssetError> {
        if asset.len() > Self::SIZE_OF_ASSET_ID_INNER {
            return Err(AssetError::AssetNameWrongLength);
        }

        // only latin and 0-9 is allowed
        // convert uppercase to lowercase
        if !Self::lower_case(&mut asset) {
            return Err(AssetError::AssetNameWrongSymbols);
        }

        let saturating_zeros = vec![0_u8; Self::SIZE_OF_ASSET_ID_INNER];

        let len = asset.len();
        let saturated = [saturating_zeros, asset].concat();
        let saturated_arr = saturated[len..].try_into().expect("slice with incorrect length");
        let id = AssetIdInnerType::from_be_bytes(saturated_arr);
        Ok(Self(id))
    }

    /// Returns original bytes which can be used in from_utf8 to get str
    pub fn to_str_bytes(&self) -> Vec<u8> {
        let bytes = self.0.to_be_bytes();
        let bytes: Vec<u8> = bytes.iter().cloned().filter(|b| b != &0_u8).collect();
        bytes
    }

    fn lower_case(asset: &mut [u8]) -> bool {
        for byte in asset {
            if byte.is_ascii_uppercase() {
                *byte = byte.to_ascii_lowercase();
            }

            if !(byte.is_ascii_digit() || byte.is_ascii_lowercase()) {
                return false
            }
        }

        true
    }
}

pub mod known {
    use super::Asset;
    pub const EQ: Asset = Asset(0x6571);
}

pub trait AsSymbol {
    /// Returns a string for inner oracle filter
    fn get_symbol(&self) -> Option<String>;

    /// Returns a symbolic string for query
    /// `is_kraken` flag could be used to specify query string for kraken,
    /// since some tokens have weird representation in its api
    fn get_query_symbol(&self, is_kraken: bool) -> Option<String>;
}

impl AsSymbol for Asset {
    fn get_symbol(&self) -> Option<String> {
        String::from_utf8(self.to_str_bytes()).ok()
    }

    fn get_query_symbol(&self, is_kraken: bool) -> Option<String> {
        let symbol = self.get_symbol()?;
        match (is_kraken, &symbol[..]) {
            (true, "eth") => Some("xethz".into()),
            (true, "btc") => Some("xxbtz".into()),
            (true, "usdt") => Some("usdtz".into()),
            _ => Some(symbol),
        }
    }
}

pub trait AssetGetter {
    type AssetId;
    type AssetData;

    fn get_asset_data(asset: Self::AssetId) -> Result<Self::AssetData, DispatchError>;

    fn exists(asset: Self::AssetId) -> bool;

    fn get_assets_data() -> Vec<(Self::AssetId, Self::AssetData)>;

    fn get_assets() -> Vec<Self::AssetId>;

    fn get_main_asset() -> Self::AssetId;
}

pub trait PriceGetter {
    type AssetId;
    type Price: FixedPointNumber;

    fn get_price(asset: Self::AssetId) -> Result<Self::Price, DispatchError>;
}

#[impl_trait_for_tuples::impl_for_tuples(5)]
pub trait OnPriceSet<AssetId, Price: FixedPointNumber> {
    fn on_price_set(asset: AssetId, price: Price);
}

pub trait ParamsValidator<AccountId, AssetId, Price, BlockNumber> {
    fn validate_params(
        who: &AccountId,
        asset: &AssetId,
        price: &Price,
        block_number: &BlockNumber,
    ) -> Result<(), &'static str>;
}

#[impl_trait_for_tuples::impl_for_tuples(5)]
impl<AccountId, AssetId, Price, BlockNumber> ParamsValidator<AccountId, AssetId, Price, BlockNumber>
    for Tuple
{
    fn validate_params(
        who: &AccountId,
        asset: &AssetId,
        price: &Price,
        block_number: &BlockNumber,
    ) -> Result<(), &'static str> {
        for_tuples!(
            #( Tuple::validate_params(who, asset, price, block_number)?; )*
        );
        Ok(())
    }
}
