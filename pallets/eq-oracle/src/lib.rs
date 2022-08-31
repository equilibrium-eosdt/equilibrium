//! # Equilibrium Oracle Pallet
//!
//! 1. Various price sources are supported.
//! PriceSource - source of received price data points
//! Custom - custom data source, url template is used.
//! Pancake - price source that provides information for LP token price calculation (not our curve LP tokens!) .
//! JSON path expressions are being parsed to retrieve price data.
//! Once the price source is set up, prices for all currencies supported in the blockchain are fed from it.
//! The price source can be changed on the fly: the validator (node) who feeds the price can do it via an RPC call.

//! 2. Pancake price source gets data from pancake swap contract and calculate price for token.
//! It requires: BCS/ETH node url, contract address, asset settings in offchain storage and prices of pool tokens stored onchain.
//! It calls read methods on smart-contract and receives addresses of both pool tokens,
//! total supply of LP token, calculate LP token price and returns it.

//! 3. Adjustable frequency of price points, it may be changed on the fly. Prices may be fed no faster than once per block.

//! 4. Medianizer is a function/business-logic module which provides a reference median price and works the following way:
//! A single feeder always uses one price source per asset
//! (e.g. a single feeder can’t feed asset price from several different sources).
//! Median works well only when there are >=3 feeders (e.g. we’re able to calculate actual median).
//! In case of a single feeder his price is used as a reference, in case of two feeders,
//! their average price is calculated to obtain the reference price.
//! There is a PriceTimeout parameter which acts as a time-rolling window and shows
//! which data points from which feeders should be taken into account when calculating a reference (median) price.
//! No different data points from the same feeder are used in the reference price calculation.

//! Example:

//! if PriceTimeout is 1 minute:

//! 1. i feed price now - it is used in calculation
//! 2. someone feeds price in 40 seconds - it is used in calculation.
//! 3. someone feeds the price in 65 seconds - my price from step 1 is not used in the calculation.

//! There is a MedianPriceTimeout parameter - if the reference (median) price is not updated more than this timeout,
//! anyone willing to obtain the price will receive an error.

//! 5. Oracle is implemented using offchain workers (implements Substrate’s offchain worker).

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(warnings)]

extern crate alloc;
use alloc::string::String;
use core::convert::TryInto;

use frame_support::pallet_prelude::DispatchResultWithPostInfo;
#[cfg(feature = "std")]
use frame_support::traits::GenesisBuild;
use frame_support::{
    codec::{Decode, Encode},
    dispatch::DispatchResult,
    traits::{Contains, Get, UnixTime},
    weights::TransactionPriority,
};
use frame_system::offchain::{
    AppCrypto, CreateSignedTransaction, ForAll, SendUnsignedTransaction, SignedPayload, Signer,
    SigningTypes,
};
use sp_arithmetic::FixedPointNumber;
use sp_core::{crypto::KeyTypeId, RuntimeDebug};
use sp_runtime::{
    traits::{Convert, IdentifyAccount, TrailingZeroInput},
    RuntimeAppPublic,
};
use sp_std::{iter::Iterator, prelude::*};
use utils::log;

use codec::FullCodec;

pub use pallet::*;
use sp_runtime::traits::Zero;

pub mod weights;
pub use weights::WeightInfo;

mod regex_offsets;
use primitives::{AssetGetter, OnPriceSet, ParamsValidator};
pub mod crypto;
pub mod offchain_storage;

pub mod price_source;
use price_source::PriceSourcePeeker;

pub mod benchmarking;
mod mock;
mod tests;

pub use utils;
pub use primitives;

/// Key type for signing transactions from off chain workers
pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"orac");
const ORACLE_PREFIX: &[u8] = b"eq-orac/";

/// Payload for a price setting with an unsigned transaction
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
pub struct PricePayload<Public, BlockNumber, AssetId, Price> {
    public: Public,
    asset: AssetId,
    price: Price,
    block_number: BlockNumber,
}

impl<T: SigningTypes, AssetId: Encode, Price: Encode> SignedPayload<T>
    for PricePayload<T::Public, T::BlockNumber, AssetId, Price>
{
    fn public(&self) -> T::Public {
        self.public.clone()
    }
}

/// Struct for storing added asset price data from one source
#[derive(Encode, Decode, Clone, Default, PartialEq, RuntimeDebug, scale_info::TypeInfo)]
pub struct PricePoint<AccountId, BlockNumber, Price> {
    block_number: BlockNumber,
    timestamp: u64,
    price: Price,
    account_id: AccountId,
}

/// Struct for storing aggregated asset price data
#[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, scale_info::TypeInfo)]
pub struct PriceData<AccountId, BlockNumber, Price> {
    pub block_number: BlockNumber,
    pub timestamp: u64,
    pub price: Price,
    pub price_points: Vec<PricePoint<AccountId, BlockNumber, Price>>,
}

impl<AccountId, BlockNumber: Default, Price: Default> Default
    for PriceData<AccountId, BlockNumber, Price>
{
    fn default() -> PriceData<AccountId, BlockNumber, Price> {
        PriceData {
            block_number: Default::default(),
            timestamp: Default::default(),
            price: Default::default(),
            price_points: Default::default(),
        }
    }
}

/// UnsignedPriorityPair = (TransactionPriority, MinTransactionWeight)
/// Unsigned priority = TransactionPriority + block_number % MinTransactionWeight
pub type UnsignedPriorityPair = (TransactionPriority, u64);
pub type AssetDataOf<T> = <<T as pallet::Config>::AssetGetter as AssetGetter>::AssetData;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + CreateSignedTransaction<Call<Self>> {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        /// Authority for signing calls submited from offchain
        type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
        /// Timestamp provider
        type UnixTime: UnixTime;
        /// Whitelist checks for price setters
        type Whitelist: Contains<Self::AccountId>;
        /// Asset id that could be represented as query string
        type AssetId: Parameter + Member + MaybeSerializeDeserialize + FullCodec;
        /// Used to deal with Assets
        type AssetGetter: AssetGetter<AssetId = Self::AssetId>;
        /// Additional validator for setting prices
        type AdditionalParamsValidator: ParamsValidator<
            Self::AccountId,
            Self::AssetId,
            Self::Price,
            Self::BlockNumber,
        >;
        /// Pallet setting representing amount of time for which price median is valid
        #[pallet::constant]
        type MedianPriceTimeout: Get<u64>;
        /// Pallet setting representing amount of time for which price point is valid
        #[pallet::constant]
        type PriceTimeout: Get<u64>;
        /// Type of fetched prices
        type Price: Parameter + Member + MaybeSerializeDeserialize + FixedPointNumber + FullCodec;
        /// Custom price source for assets, could be a Tuple of price sources
        type PriceSource: PriceSourcePeeker<Self::AssetId, AssetDataOf<Self>>;
        /// Direct correlation map between assets, e.g.: Price[XDOT] = 1.0 * Price[DOT]
        type DirectPriceCorrelation: for<'a> Convert<
            (&'a Self::AssetId, &'a AssetDataOf<Self>),
            Option<(Self::AssetId, Self::Price)>,
        >;
        /// Prices known at runtime or constant prices, e.g.: Price[EQD] = 1.0
        type SpecialPrices: for<'a> Convert<
            (&'a Self::AssetId, &'a AssetDataOf<Self>),
            Option<Self::Price>,
        >;
        /// Interface for feeding new prices into other pallets
        type OnPriceSet: OnPriceSet<Self::AssetId, Self::Price>;
        /// For priority calculation of an unsigned transaction
        #[pallet::constant]
        type UnsignedPriority: Get<UnsignedPriorityPair>;
        /// Lifetime in blocks for unsigned transactions
        #[pallet::constant]
        type UnsignedLifetimeInBlocks: Get<u32>;
        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight((T::WeightInfo::set_price(10), DispatchClass::Operational))]
        /// Adds and saves a new `DataPoint` containing an asset price information. It
        /// would be used for the `PricePoint` calculation. Only whitelisted
        /// accounts can add `DataPoints`
        pub fn set_price(
            origin: OriginFor<T>,
            asset: T::AssetId,
            price: T::Price,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            let current_block = frame_system::Pallet::<T>::block_number();

            Self::validate_params(&who, &asset, &price, current_block)?;
            Self::set_price_inner(who, asset, price)?;

            Ok(Pays::No.into())
        }

        #[pallet::weight((T::WeightInfo::set_price(10), DispatchClass::Operational))]
        /// Adds new `DataPoint` from an unsigned transaction
        pub fn set_price_unsigned(
            origin: OriginFor<T>,
            payload: PricePayload<T::Public, T::BlockNumber, T::AssetId, T::Price>,
            _signature: T::Signature,
        ) -> DispatchResultWithPostInfo {
            ensure_none(origin)?;
            let PricePayload {
                public,
                asset,
                price,
                ..
            } = payload;
            let who = public.into_account();
            Self::validate_params(&who, &asset, &price, payload.block_number)?;
            Self::set_price_inner(who, asset, price)?;

            Ok(().into())
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// Starts an off-chain task for a given block number
        fn offchain_worker(block_number: T::BlockNumber) {
            // collect the public keys
            let publics = <T::AuthorityId as AppCrypto<_, _>>::RuntimeAppPublic::all()
                .into_iter()
                .enumerate()
                .filter_map(|(_index, key)| {
                    let generic_public =
                        <T::AuthorityId as AppCrypto<_, _>>::GenericPublic::from(key);
                    let public: T::Public = generic_public.into();
                    let account_id = public.clone().into_account();
                    if T::Whitelist::contains(&account_id) {
                        Some(public)
                    } else {
                        None
                    }
                })
                .collect();

            let signer = Signer::<T, T::AuthorityId>::all_accounts().with_filter(publics);
            if !signer.can_sign() {
                // not in the whitelist
                return;
            }
            //acquire a lock
            let lock_res = utils::offchain::acquire_lock(ORACLE_PREFIX, || {
                // All oracles must set their own price feeding frequency
                // Oracle feeds prices every N blocks, where N = oracle::price_periodicity
                let maybe_price_periodicity = offchain_storage::get_price_periodicity();
                if maybe_price_periodicity.is_none() {
                    log::warn!("Price periodicity setting doesn't exists");
                    return;
                }

                let price_periodicity = maybe_price_periodicity.unwrap();
                if price_periodicity < 1 {
                    log::warn!(
                        "Unexpected price periodicity {:?}, should be more or equal 1",
                        price_periodicity
                    );
                    return;
                }

                let counter = offchain_storage::get_counter().unwrap_or(0_u32);
                let counter_next = counter + 1;

                if counter_next == price_periodicity {
                    offchain_storage::set_counter(0_u32);

                    // Prices source
                    if let Some(source_type_name) = offchain_storage::get_source_type() {
                        Self::update_prices(source_type_name, block_number, &signer);
                    }
                } else if counter_next > price_periodicity {
                    offchain_storage::set_counter(0_u32);
                } else {
                    offchain_storage::set_counter(counter_next);
                }
            });
            log::trace!(target: "eq_oracle", "offchain_worker:{:?}", lock_res);
        }

        fn on_initialize(_: BlockNumberFor<T>) -> Weight {
            for asset in T::AssetGetter::get_assets_data() {
                if let Some(price) = T::SpecialPrices::convert((&asset.0, &asset.1)) {
                    Self::set_the_only_price(asset.0.clone(), price);
                    continue;
                }

                if let Some((corr_asset, correlation)) =
                    T::DirectPriceCorrelation::convert((&asset.0, &asset.1))
                {
                    if let Ok(price) =
                        <Self as primitives::PriceGetter>::get_price(corr_asset.clone())
                    {
                        Self::set_the_only_price(asset.0.clone(), correlation * price);
                        continue;
                    }
                }
            }

            10_000
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new price added to the storage. The event contains: `AssetId` for the price,
        /// `Price` for the price value that was added, `Price` for a new
        /// aggregated price and `AccountId` of the price submitter
        /// \[asset, new_value, aggregated, submitter\]
        NewPrice(T::AssetId, T::Price, T::Price, T::AccountId),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Params are
        AdditionalValidatorFailed,
        /// The account is not allowed to set prices
        NotAllowedToSubmitPrice,
        /// The same price data point has been already added
        PriceAlreadyAdded,
        /// Incorrect asset
        CurrencyNotFound,
        /// Attempting to submit a new price for constant price currencies
        WrongCurrency,
        /// The price cannot be zero
        PriceIsZero,
        /// The price cannot be negative
        PriceIsNegative,
        /// The price data point is too old and cannot be used
        PriceTimeout,
    }

    /// Pallet storage for added price points
    #[pallet::storage]
    #[pallet::getter(fn price_points)]
    pub(super) type PricePoints<T: Config> = StorageMap<
        _,
        Identity,
        T::AssetId,
        PriceData<
            <T as frame_system::Config>::AccountId,
            <T as frame_system::Config>::BlockNumber,
            T::Price,
        >,
        OptionQuery,
    >;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub prices: Vec<(T::AssetId, T::Price)>,
        pub update_date: u64,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                prices: Default::default(),
                update_date: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            // with chain spec
            for asset in T::AssetGetter::get_assets() {
                <PricePoints<T>>::insert(asset, PriceData::default());
            }
            for (asset, price) in &self.prices {
                <PricePoints<T>>::mutate(asset, |maybe_price_point| {
                    let price_point = PriceData {
                        timestamp: self.update_date,
                        price: price.clone(),
                        ..Default::default()
                    };
                    *maybe_price_point = Some(price_point);
                });
            }
        }
    }

    #[pallet::validate_unsigned]
    impl<T: Config> ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;

        fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            if let Call::set_price_unsigned { payload, signature } = call {
                let signature_valid =
                    SignedPayload::<T>::verify::<T::AuthorityId>(payload, signature.clone());
                if !signature_valid {
                    return InvalidTransaction::BadProof.into();
                }

                let current_block = <frame_system::Pallet<T>>::block_number();

                if payload.block_number > current_block {
                    // transaction in future?
                    return InvalidTransaction::Stale.into();
                } else if payload.block_number + T::UnsignedLifetimeInBlocks::get().into()
                    < current_block
                {
                    // transaction was in pool for 5 blocks
                    return InvalidTransaction::Stale.into();
                }

                let account = payload.public.clone().into_account();
                Self::validate_params(
                    &account,
                    &payload.asset,
                    &payload.price,
                    payload.block_number,
                )
                .map_err(|_| InvalidTransaction::Call)?;

                let (initial_priority, min_transaction_weight) = T::UnsignedPriority::get();
                let priority = initial_priority.saturating_add(
                    (TryInto::<u64>::try_into(payload.block_number).unwrap_or(0))
                        % min_transaction_weight,
                );

                ValidTransaction::with_tag_prefix("EqOracleSetPrice")
                    .priority(priority)
                    .and_provides((payload.public.clone(), &payload.asset))
                    .longevity(5) // hotfix, transfer to config
                    .propagate(true)
                    .build()
            } else {
                InvalidTransaction::Call.into()
            }
        }
    }
}

impl<T: Config> Pallet<T> {
    /// Initializes price source and gets prices
    fn get_prices(source_type_name: String) -> Vec<(T::AssetId, Result<T::Price, &'static str>)> {
        let assets_data = T::AssetGetter::get_assets_data();

        match T::PriceSource::get_prices(&source_type_name, &assets_data) {
            Ok(prices) => prices,
            Err(Some(err)) => {
                log::error!("Error while creating price source: {:?}.", err);
                Vec::new()
            }
            Err(None) => {
                log::error!("Unexpected price resource type: {:?}.", source_type_name);
                Vec::new()
            }
        }
    }

    fn update_prices(
        source_type_name: String,
        block_number: T::BlockNumber,
        signer: &Signer<T, T::AuthorityId, ForAll>,
    ) {
        for (asset, price_result) in Self::get_prices(source_type_name) {
            match price_result {
                Ok(price) => {
                    Self::submit_tx_update_price(asset, price, block_number, signer);
                }
                Err(err) => {
                    log::error!(
                        "Price source return error.  Asset: {:?}, error: {:?}",
                        asset,
                        err,
                    );

                    // skip error, try update other asset prices
                    continue;
                }
            }
        }
    }

    /// Prepares unsigned transaction with new price
    fn submit_tx_update_price(
        asset: T::AssetId,
        price: T::Price,
        block_number: T::BlockNumber,
        signer: &Signer<T, T::AuthorityId, ForAll>,
    ) {
        signer.send_unsigned_transaction(
            |account| PricePayload {
                public: account.public.clone(),
                asset: asset.clone(),
                price,
                block_number,
            },
            |payload, signature| Call::set_price_unsigned { payload, signature },
        );
    }

    /// Validates the parameters fot setting price
    fn validate_params(
        who: &T::AccountId,
        asset: &T::AssetId,
        price: &T::Price,
        block_number: T::BlockNumber,
    ) -> DispatchResult {
        if let Err(err) =
            T::AdditionalParamsValidator::validate_params(who, asset, price, &block_number)
        {
            log::error!(
                target: "eq_oracle",
                "Additional validator failed. Error: {:?}.",
                err
            );
            frame_support::fail!(Error::<T>::AdditionalValidatorFailed)
        }

        if !T::Whitelist::contains(who) {
            log::error!(
                target: "eq_oracle",
                "Account not in whitelist. Who: {:?}.",
                who
            );
            frame_support::fail!(Error::<T>::NotAllowedToSubmitPrice)
        }

        let asset_data = match T::AssetGetter::get_asset_data(asset.clone()) {
            Ok(asset_data) => asset_data,
            Err(err) => {
                log::error!(
                    target: "eq_oracle",
                    "Asset not found. Who: {:?}, asset: {:?}.",
                    who,
                    asset
                );
                frame_support::fail!(err)
            }
        };
        if T::SpecialPrices::convert((&asset, &asset_data)).is_some()
            || T::DirectPriceCorrelation::convert((&asset, &asset_data)).is_some()
        {
            log::error!(
                target: "eq_oracle",
                "Asset is not allowed to set price. Who: {:?}, price: {:?}, asset: {:?}.",
                who,
                price,
                asset
            );
            frame_support::fail!(Error::<T>::WrongCurrency)
        }

        if price.is_negative() {
            log::error!(
                target: "eq_oracle",
                "Price is negative. Who: {:?}, price: {:?}, asset: {:?}.",
                who,
                price,
                asset
            );
            frame_support::fail!(Error::<T>::PriceIsNegative)
        }

        if price.is_zero() {
            log::error!(
                target: "eq_oracle",
                "Price is equal to zero. Who: {:?}, price: {:?}, asset: {:?}.",
                who,
                price,
                asset,
            );
            frame_support::fail!(Error::<T>::PriceIsZero)
        }

        if !T::AssetGetter::exists(asset.clone()) {
            log::error!(
                target: "eq_oracle",
                "'Unknown' is not allowed to set price. Who: {:?}, price: {:?}, asset: {:?}.",
                who,
                price,
                asset,
            );
            frame_support::fail!(Error::<T>::WrongCurrency)
        }

        return Ok(());
    }

    /// A variant when a price is a single value
    fn set_the_only_price(asset: T::AssetId, price: T::Price) {
        let block_number = frame_system::Pallet::<T>::block_number();
        let timestamp = T::UnixTime::now().as_secs();
        let account_id = T::AccountId::decode(&mut TrailingZeroInput::new(b"oracle::price_setter"))
            .expect("Correct default account");

        let price_point = PriceData {
            block_number,
            timestamp,
            price,
            price_points: vec![PricePoint {
                price,
                account_id: account_id.clone(),
                block_number,
                timestamp,
            }],
        };

        <PricePoints<T>>::insert(&asset, price_point);
        T::OnPriceSet::on_price_set(asset.clone(), price);
        Self::deposit_event(Event::NewPrice(asset, price, price, account_id));
    }

    /// Calculate a median over **sorted** price points
    fn calc_median_price(
        data_points: &[PricePoint<T::AccountId, T::BlockNumber, T::Price>],
    ) -> T::Price {
        let len = data_points.len();
        if len % 2 == 0 {
            (data_points[len / 2 - 1].price + data_points[len / 2].price)
                / T::Price::saturating_from_integer(2)
        } else {
            data_points[len / 2].price
        }
    }

    fn set_price_inner(who: T::AccountId, asset: T::AssetId, price: T::Price) -> DispatchResult {
        let mut median_price = price;

        // mutate a price point in the storage by the asset
        <PricePoints<T>>::try_mutate(&asset, |maybe_price_data| -> DispatchResult {
            let mut price_data = maybe_price_data.clone().unwrap_or_default();
            let block_number = frame_system::Pallet::<T>::block_number();
            let timestamp = T::UnixTime::now().as_secs(); // always same within block

            if price_data.block_number == block_number
                && price_data
                    .price_points
                    .iter()
                    .any(|pp| pp.account_id == who && pp.block_number == block_number)
            {
                log::error!(
                        "Account already set price. Who: {:?}, price: {:?}, block: {:?}, timestamp: {:?}.",
                        who,
                        price_data.price,
                        price_data.block_number,
                        price_data.timestamp
                    );
                frame_support::fail!(Error::<T>::PriceAlreadyAdded)
            }

            // clear outdated price points
            price_data.price_points.retain(|pp| {
                pp.timestamp + T::PriceTimeout::get() > timestamp && pp.account_id != who
            });
            price_data.block_number = block_number;
            price_data.timestamp = timestamp;

            // add price point to price_point preserving order by price
            let data_point = PricePoint {
                account_id: who.clone(),
                price,
                block_number,
                timestamp,
            };
            match price_data
                .price_points
                .binary_search_by(|dp| dp.price.cmp(&price))
            {
                Ok(pos) | Err(pos) => price_data.price_points.insert(pos, data_point),
            }

            // calculate a median over price points for the moment
            median_price = Self::calc_median_price(&price_data.price_points);
            price_data.price = median_price;

            log::info!(
                target: "eq_oracle",
                "Median calc. price: {:?} median_price: {:?} asset: {:?}",
                price,
                median_price,
                asset
            );
            *maybe_price_data = Some(price_data);
            Ok(())
        })?;

        T::OnPriceSet::on_price_set(asset.clone(), price);
        Self::deposit_event(Event::NewPrice(asset, price, median_price, who));
        Ok(())
    }

    /// Remove prices from `who` and recalc median price for each asset
    pub fn filter_prices_from(who: &T::AccountId) {
        for asset in T::AssetGetter::get_assets() {
            <PricePoints<T>>::mutate_exists(asset, |maybe_price_data| {
                if let Some(PriceData {
                    price,
                    price_points,
                    ..
                }) = maybe_price_data.as_mut()
                {
                    let initial_len = price_points.len();
                    price_points.retain(|pp| &pp.account_id != who);
                    if price_points.len() == 0 {
                        *maybe_price_data = None;
                    } else if price_points.len() != initial_len {
                        *price = Self::calc_median_price(price_points);
                    }
                };
            });
        }
    }
}

impl<T: Config> primitives::PriceGetter for Pallet<T> {
    type AssetId = T::AssetId;
    type Price = T::Price;

    fn get_price(asset: T::AssetId) -> Result<T::Price, sp_runtime::DispatchError> {
        // SpecialPrices and DirectCorrelatedPrices already in PricePoints, no need to check
        let price_point = <PricePoints<T>>::get(&asset).ok_or_else(|| {
            log::error!(
                target: "eq_oracle",
                "Currency not found in PricePoints. asset: {:?}.",
                asset
            );
            Error::<T>::CurrencyNotFound
        })?;

        let current_time = T::UnixTime::now().as_secs();
        if current_time >= price_point.timestamp + T::MedianPriceTimeout::get() {
            log::error!(
                target: "eq_oracle",
                "{:?} Price received after time is out. Current time: {:?}, price_point timestamp + {:?} seconds: {:?}.",
                asset,
                current_time,
                T::MedianPriceTimeout::get(),
                price_point.timestamp + T::MedianPriceTimeout::get(),
            );
            frame_support::fail!(Error::<T>::PriceTimeout);
        }

        let price = price_point.price;

        if price.is_zero() {
            log::error!(
                target: "eq_oracle",
                "Price is equal to zero. Price: {:?}, asset: {:?}.",
                price,
                asset,
            );
            frame_support::fail!(Error::<T>::PriceIsZero);
        }

        if price.is_negative() {
            log::error!(
                target: "eq_oracle",
                "Price is negative. Price: {:?}, asset: {:?}.",
                price,
                asset,
            );
            frame_support::fail!(Error::<T>::PriceIsNegative);
        }

        Ok(price)
    }
}
