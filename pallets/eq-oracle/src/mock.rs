#![cfg(test)]

use super::*;
use crate as eq_oracle;
pub use crate::price_source::{json::JsonPriceSource, PriceSourceStruct};
use core::cell::RefCell;
use frame_support::parameter_types;
use frame_support::traits::Everything;
use primitives::Asset;
use sp_core::{sr25519::Signature, H256};
use sp_runtime::traits::One;
use sp_runtime::{
    testing::{Header, TestXt},
    traits::{BlakeTwo256, Extrinsic as ExtrinsicT, IdentifyAccount, IdentityLookup, Verify},
};
use sp_runtime::{DispatchError, FixedI64};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

use core::convert::{TryFrom, TryInto};

pub mod asset {
    use primitives::Asset;

    pub const EQ: Asset = Asset(0x6571);
    pub const EQD: Asset = Asset(0x657164);
    pub const BTC: Asset = Asset(0x627463);
    pub const ETH: Asset = Asset(0x657468);

    pub const DOT: Asset = Asset(0x646F74);
    pub const XDOT: Asset = Asset(0x78646F74);
    pub const HDOT: Asset = Asset(0x68646f74);

    pub const LP_XDOT: Asset = Asset(0x786C707430);
    pub const LP_CURVE: Asset = Asset(0x6C707430);
}

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Oracle: eq_oracle::{Pallet, Call, Storage, Event<T>},
        Timestamp: pallet_timestamp::{Pallet, Call, Storage},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(1024);
    pub const MinimumPeriod: u64 = 1;
    pub const UnsignedPriority: UnsignedPriorityPair = (0, 1_000_000);
}
impl frame_system::Config for Test {
    type BaseCallFilter = Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

thread_local! {
    pub static WHITELIST: RefCell<Vec<AccountId>> = RefCell::new(vec![]);
}

pub struct Whitelist;

impl Whitelist {
    pub fn add_to_whitelist(who: &AccountId) {
        WHITELIST
            .try_with(|whitelist| {
                let mut whitelist = whitelist.borrow_mut();
                if let Err(pos) = whitelist.binary_search(who) {
                    whitelist.insert(pos, who.clone());
                }
            })
            .unwrap();
    }

    pub fn remove_from_whitelist(who: &AccountId) {
        WHITELIST
            .try_with(|whitelist| {
                let mut whitelist = whitelist.borrow_mut();
                if let Ok(pos) = whitelist.binary_search(who) {
                    whitelist.remove(pos);
                }
            })
            .unwrap()
    }
}

impl Contains<AccountId> for Whitelist {
    fn contains(who: &AccountId) -> bool {
        WHITELIST
            .try_with(|whitelist| whitelist.borrow().contains(who))
            .unwrap_or(false)
    }
}

thread_local! {
    pub static ASSETS: RefCell<Vec<Asset>> = RefCell::new(vec![]);
}

pub struct AssetGetterMock;

impl AssetGetterMock {
    pub fn add_asset(asset: Asset) {
        ASSETS
            .try_with(|assets| {
                let mut assets = assets.borrow_mut();
                if let Err(pos) = assets.binary_search(&asset) {
                    assets.insert(pos, asset);
                }
            })
            .unwrap()
    }
}

impl AssetGetter for AssetGetterMock {
    type AssetId = Asset;
    type AssetData = ();

    fn get_asset_data(asset: Self::AssetId) -> Result<Self::AssetData, DispatchError> {
        ASSETS
            .try_with(|assets| {
                let assets = assets.borrow();
                match assets.binary_search(&asset) {
                    Ok(_) => Some(()),
                    Err(_) => None,
                }
            })
            .ok()
            .flatten()
            .ok_or(DispatchError::Other("No asset"))
    }

    fn get_assets() -> Vec<Self::AssetId> {
        ASSETS
            .try_with(|assets| assets.borrow().clone())
            .unwrap_or_default()
    }

    fn get_assets_data() -> Vec<(Self::AssetId, Self::AssetData)> {
        let assets = Self::get_assets();
        assets.into_iter().map(|a| (a, ())).collect()
    }

    fn exists(asset: Self::AssetId) -> bool {
        Self::get_asset_data(asset).is_ok()
    }

    fn get_main_asset() -> Self::AssetId {
        asset::EQ
    }
}

type Extrinsic = TestXt<Call, ()>;
type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

impl frame_system::offchain::SigningTypes for Test {
    type Public = <Signature as Verify>::Signer;
    type Signature = Signature;
}

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test
where
    Call: From<LocalCall>,
{
    type OverarchingCall = Call;
    type Extrinsic = Extrinsic;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Test
where
    Call: From<LocalCall>,
{
    fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
        call: Call,
        _public: <Signature as Verify>::Signer,
        _account: AccountId,
        nonce: u64,
    ) -> Option<(Call, <Extrinsic as ExtrinsicT>::SignaturePayload)> {
        Some((call, (nonce, ())))
    }
}

parameter_types! {
    pub const PriceTimeout: u64 = 1;
    pub const MedianPriceTimeout: u64 = 60 * 60 * 2;
}

pub struct FinancialMock;
impl OnPriceSet<Asset, FixedI64> for FinancialMock {
    fn on_price_set(_asset: Asset, _value: FixedI64) {}
}

parameter_types! {
    pub const LpPriceBlockTimeout: u64 = 10u64;
    pub const UnsignedLifetimeInBlocks: u32 = 5;
    pub const FinancialRecalcPeriodBlocks: u64  = (1000 * 60 * 60 * 4) as u64 / 6000;
}

pub struct DirectPriceCorrelation;
impl<'a> Convert<(&'a Asset, &'a ()), Option<(Asset, FixedI64)>> for DirectPriceCorrelation {
    fn convert((a, _): (&'a Asset, &'a ())) -> Option<(Asset, FixedI64)> {
        match *a {
            // asset::EQ => Some((asset::DOT, FixedI64::saturating_from_rational(7, 10))),
            asset::XDOT | asset::HDOT => Some((asset::DOT, FixedI64::one())),
            _ => None,
        }
    }
}

pub struct SpecialPrices;
impl<'a> Convert<(&'a Asset, &'a ()), Option<FixedI64>> for SpecialPrices {
    fn convert((a, _): (&'a Asset, &'a ())) -> Option<FixedI64> {
        match *a {
            asset::EQD => Some(FixedI64::one()),
            asset::LP_CURVE | asset::LP_XDOT => Some(FixedI64::one() + FixedI64::one()),
            _ => None,
        }
    }
}

impl eq_oracle::Config for Test {
    type Event = Event;
    type AuthorityId = crypto::TestAuthId;
    type AssetId = Asset;
    type UnixTime = pallet_timestamp::Pallet<Self>;
    type Whitelist = Whitelist;
    type MedianPriceTimeout = MedianPriceTimeout;
    type PriceTimeout = PriceTimeout;
    type OnPriceSet = FinancialMock;
    type UnsignedPriority = UnsignedPriority;
    type AssetGetter = AssetGetterMock;
    type WeightInfo = ();
    type UnsignedLifetimeInBlocks = UnsignedLifetimeInBlocks;
    type AdditionalParamsValidator = ();
    type Price = FixedI64;
    type PriceSource = (PriceSourceStruct<JsonPriceSource<Asset, ()>>,);
    type DirectPriceCorrelation = DirectPriceCorrelation;
    type SpecialPrices = SpecialPrices;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    AssetGetterMock::add_asset(asset::EQD);
    AssetGetterMock::add_asset(asset::BTC);
    AssetGetterMock::add_asset(asset::ETH);
    AssetGetterMock::add_asset(asset::DOT);
    AssetGetterMock::add_asset(asset::EQ);

    eq_oracle::GenesisConfig::<Test> {
        prices: vec![],
        update_date: 0,
    }
    .assimilate_storage(&mut t)
    .unwrap();

    t.into()
}
