#![cfg(feature = "runtime-benchmarks")]
#![allow(warnings)]

use super::*;
use crate::Call;
use frame_benchmarking::{account, benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use sp_runtime::{traits::One, FixedI64, FixedPointNumber};

pub struct AssetGetterMock;
impl AssetGetter for AssetGetterMock {
    type AssetId = u64;
    type AssetData = ();

    fn get(asset: &u64) -> Option<()> {
        (asset == &0x01234567).then(|| ())
    }

    fn get_assets() -> Vec<Self::AssetId> {
        vec![0x01234567]
    }

    fn get_assets_data() -> Vec<(Self::AssetId, Self::AssetData)> {
        vec![(0x01234567, ())]
    }
}

pub struct Module<T: Config>(crate::Pallet<T>);
pub trait Config:
    crate::Config<
    Whitelist = Everything,
    AssetId = u64,
    Price = FixedI64,
    AssetGetter = AssetGetterMock,
>
{
}

benchmarks! {
    set_price {
        let b in 1 .. 20;

        for i in 0..b {
            let price_setter: T::AccountId = account("price_setter", i, 0);
            Pallet::<T>::set_price(
                RawOrigin::Signed(price_setter).into(),
                0x01234567,
                FixedI64::one()
            ).unwrap();
        }

        let caller: T::AccountId = whitelisted_caller();
    }: _ (
        RawOrigin::Signed(caller),
        0x01234567,
        FixedI64::one()
    )
    verify {}
}
