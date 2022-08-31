
# Pallet description

Equilibrium oracle provides high speed price feeds for DeFi applications within the polkadot Ecosystem.

## Medianizer

Medianizer is a function/business-logic module which provides a reference median price and will work the following way:

- A single feeder always uses one price source per asset (e.g. a single feeder can’t feed asset price from several different sources).
- Median works well only when there are >=3 feeders (e.g. we’re able to calculate actual median). In case of a single feeder his price is used as a reference, in case of two feeders, their average price is calculated to obtain the reference price.
- There is a PriceTimeout parameter which acts as a time-rolling window and shows which data points from which feeders should be taken into account when calculating a reference (median) price. No different data points from the same feeder are used in the reference price calculation.
- There is a MedianPriceTimeout parameter - if the reference (median) price is not updated more than this timeout, anyone willing to obtain the price will receive an error.

## Feeder flow 

1. Feeder registration = transfer of a generic currencyId to the pallet’s account and add yourself to Whitelist.
2. Feeder configure its price source via offchain storage. `oracle::resource_type` - is one of the PriceSource, available on chain, e.g. `custom`. Then offchain local storage should be configured according to resource_type definition:
   - `oracle::custom_query` => `json(https://api.binance.com/api/v3/ticker/price?symbol={$}USDT).price`;
   - `oracle::source_assets` => `eth,btc,usdt`.
3. Feeder choses price_periodicity e.g. the frequency he wants to feed with. If feeder doesn’t feed prices more than NumberOfTimeoutPeriods * price_periodicity.

# Data Model

## Constants

`KEY_TYPE` - key type for signing transactions from off chain workers. 

Type is set to sp_core::crypto::KeyTypeId(*b"orac").

### Scalars

- `PriceTimeout: u64` (pallet setting) - amount of time for which price point is valid (seconds).
- `MedianPriceTimeout: u64` (pallet setting) - amount of time for which price median is valid (seconds).
- `oracle::price_periodicity: u32` (off-chain setting) - amount of blocks between price feeds.
- `oracle::resource_type: String` (off-chain setting) - type of external data source.
- `oracle::custom_query: String` (off-chain setting) - query string for fetching assets' prices with `get` http method.
- `oracle::source_assets: String` (off-chain setting) - list of assets to fetch price

### Associated types
- `Whitelist` - container with authorities allowed to feed prices
- `AssetId: AsSymbol` - generic currency for fetching price
- `AssetGetter` - container for all assets in system
- `AdditionalParamsValidator` - custom validator for `set_price`
- `Price: FixedPointNumber` - generic price type
- `PriceSource` - available built-in price sources for feeders
- `DirectPriceCorrelation` - direct correlation map between assets
- `SpecialPrices` - prices known at runtime or constant prices
- `OnPriceSet` - interface for feeding new prices into other pallets

### Traits

PriceSource - obtains assets' prices in off-chain and submits
- Custom - http based price source, allowing to fetch `application/json` http request and parse it.
 
### Maps

PricePoints: AssetId => PricePoint;
- Per asset metadata with current median price and older price points.

### Structs

PricePayload - Stores payload for unsigned transactions.
- `public: Public` - public key of transactor
- `currency: AssetId` - asset
- `price: Price` - price value

DataPoint - Stores price data from single source
- `price: Price` - price value
- `account_id: AccountId` - feeder’s account id
- `block_number: BlockNumber` - block number of price adding
- `timestamp: u64` - timestamp of price adding

PricePoint - Stores metadata with current median price and older price datas.
- `block_number: BlockNumber` - block number of median price update
- `price: Price` - median price value
- `timestamp: u64` - timestamp of median price update
- `data_points: Vec<DataPoint>` - prices from different sources

### Inner functions

#### fetch_price_from_json - Return price from json string.

Function Signature

    fn fetch_price_from_json(body: String, path: &str) -> Result<FixedI64, http::Error>

Parameters

- `body: String` - server endpoint string
- `path: String` - json path to price

Returns

- `Result<Price>` - maybe price for given asset

Events

- None

Errors

- http::Error::Unknown

Preconditions

- None

Function Sequence

1. Deserialize string to json. Return http::Error::Unknown, if string cannot be deserialized.
2. Get value from json-path.
3. Convert value to f64. Return http::Error::Unknown if value cannot be converted.
4. Return FixedI64 from f64 value.

#### fetch_price - Get price for currency from query. Query contains templated url and templated json path. Example: json(https://api.hitbtc.com/api/2/public/ticker/{$}USD).

Function Signature

    fn fetch_price(currency: &AssetId, query: &String) -> Result<Price, http::Error>

Parameters

- `asset: &AssetId` - asset for fetching price
- `query: &String` - query string

Returns

- `Result<Price>` - maybe price for given asset

Events

- None

Errors

- http::Error

Preconditions

- None

Function Sequence

1. Extract url from query. Return http::Error::Unknown if error.
2. Check if the url contains "{$}". If not, return http::Error::Unknown.
3. Extract path_template.
4. Call exec_query. Check for error and return it, if there is one.
5. Call fetch_price_from_json and return it's result.

#### get_local_storage_val - Helper function for getting values from local storage.

Function Signature

    fn get_local_storage_val<R: FromStr>(key: &[u8]) -> Option<R>

Parameters

- `key: &[u8]` - raw key to extract the value from

Returns

- `Option<R>` - maybe value from local storage

Events

- None

Errors

- None

Preconditions

- None

Function Sequence

1. Extract raw value with sp_io::offchain::local_storage_get.
2. If the value is “none”, return it.
3. Convert value to string. If value is not converted return None.
4. Parse string to generic type R. If string is not parsed return None.
5. Return parsed value within Some.

### Outer functions

#### get_price - Gets current reference price for given currency.

Function Signature

    fn get_price(asset: &AssetId) -> Result<Price, sp_runtime::DispatchError>

Parameters

- `asset: &AssetId` - given asset

Returns

- `Result<Price>` - maybe price for given asset

Events

- None

Errors

- `CurrencyNotFound` - Incorrect currency
- `PriceIsZero` - Price cannot be zero
- `PriceIsNegative` - Price cannot be negative
- `PriceTimeout` - Reference price is too old and cannot be used

Preconditions

- None

Function Sequence
1. Check if currency is Usd. If it is, return 1.
2. Check if PricePoints contains currency. If it doesn’t, return CurrencyNotFound.
3. Get price from PricePoints.
4. Check if the price is zero. If it is, return PriceIsZero.
5. Check if the price is negative. If it is, return PriceIsNegative.
6. Check if the price point timestamp is expired. If it is, return PriceTimeout.
7. Return price within Ok.

### Extrinsics

#### set_price - Setting price manually by feeder.

Function Signature

    fn set_price(origin, currency: AssetId, price: Price) -> DispatchResult

Parameters

- `who: AccountId` - feeder's account id
- `asset: AssetId` - currency for which the price is set
- `price: Price` - new price value

Returns

- `DispatchResult`

Events

- `NewPrice(AssetId, Price, Price, AccountId)` - Signals the new reference price and feeded price when it is updated.

Errors

- `NotAllowedToSubmitPrice` - `who` is not a feeder;  
- `WrongCurrency` - currency not available to set prices;
- `PriceIsNegative`, `PriceIsZero` - non valid price value;
- `PriceAlreadyAdded` - the same price data point was already added;

Preconditions

- None

Function Sequence
1. Check if the last DataPoint for given currency from a given user was setted in the current block. If it was, return PriceAlreadyAdded.
2. Update PricePoint timestamp and block number.
3. Filter DataPoints from PricePoint for not expired.
4. Insert new DataPoint with new price.
5. Calculate median.
6. Update PricePoint with new median price.
7. Emit the NewPrice event.
8. Return Ok(()).

#### set_price_unsigned - Setting price automatically by offchain.

Function Signature

    pub fn set_price_unsigned(origin, payload: PricePayload<T::Public>, _signature: T::Signature) -> DispatchResult

Parameters

- `payload: PricePayload` - payload with new price
- `_signature: Signature` - unused

Returns

- `DispatchResult`

Events

- `NewPrice(AssetId, Price, Price, AccountId)` - Signals the new reference price and feeded price when it is updated.

Errors

- `NotAllowedToSubmitPrice` - `who` is not a feeder;  
- `WrongCurrency` - currency not available to set prices;
- `PriceIsNegative`, `PriceIsZero` - non valid price value;
- `PriceAlreadyAdded` - the same price data point was already added;

Preconditions

- None

Function Sequence

1. Ensure that the origin represents an unsigned extrinsic. If not, return sp_runtime::traits::BadOrigin.
2. Call validate_params. Check for error and return it, if there is one.
3. Call _set_price and return the result.

### offchain_worker - Main logic - periodicity check, price feed, price set. Implements Substrate's off chain worker.

Function Signature

    fn offchain_worker(_block_number: T::BlockNumber)

Parameters

- `_block_number` - number of current block

Returns

- None

Events

- `NewPrice(AssetId, Price, Price, AccountId)` - Signals the new reference price and feeded price when it is updated.

Errors

- `NotAllowedToSubmitPrice` - `who` is not a feeder;  
- `WrongCurrency` - currency not available to set prices;
- `PriceIsNegative`, `PriceIsZero` - non valid price value;
- `PriceAlreadyAdded` - the same price data point was already added;

Preconditions

- Off-chain workers must be enabled on nodes.

Function Sequence

1. Check if the caller of the function is enabled. If not, return.
2. Get price_periodicity from local storage. If not or if price_periodicity < 1, return.
3. Get resource_type from local storage. If not, return.
4. For every asset call fetch_price.
5. Call wrong_price() and check it. Call slash() if true, or call send_unsigned_transaction with set_price_unsigned otherwise.
6. Return.
