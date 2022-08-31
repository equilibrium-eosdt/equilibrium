#![cfg(test)]

use frame_support::{assert_err, assert_ok};
use sp_arithmetic::FixedI64;

use crate::{
    mock::*,
    price_source::json::{PriceSourceError, WithUrl},
};
use primitives::{Asset, PriceGetter};

use super::*;

pub type Sign = sp_core::sr25519::Public;

fn set_price(
    account: Sign,
    asset: Asset,
    price: f64,
    block_number: u64,
) -> DispatchResultWithPostInfo {
    let dummy_signature = sp_core::sr25519::Signature([0u8; 64]);
    let payload = PricePayload {
        public: account,
        asset,
        price: FixedI64::from_inner((price * (FixedI64::accuracy() as f64)) as i64),
        block_number,
    };
    Oracle::set_price_unsigned(
        frame_system::RawOrigin::None.into(),
        payload,
        dummy_signature,
    )
}

fn set_price_ok(account: Sign, asset: Asset, price: f64, block_number: u64) {
    assert_ok!(set_price(account, asset, price, block_number));
}

fn check_price(asset: Asset, price: f64) {
    assert_eq!(
        Oracle::get_price(asset).unwrap(),
        FixedI64::from_inner((price * (FixedI64::accuracy() as f64)) as i64)
    );
}

fn check_error(dr: DispatchResultWithPostInfo, msg: &str) {
    let a: &str = From::<
        sp_runtime::DispatchErrorWithPostInfo<frame_support::weights::PostDispatchInfo>,
    >::from(dr.expect_err(""));
    assert_eq!(a, msg);
}

fn time_move(time: &mut u64, step: u64) {
    println!(
        "timemove: time: {} sec, step: {} sec, current: {} sec.",
        *time,
        step,
        *time + step
    );

    *time = *time + step;
    Timestamp::set_timestamp(*time * 1000);
    System::set_block_number(*time / 6);
}

#[test]
///Test
fn main_test() {
    new_test_ext().execute_with(|| {
        let account_id_1 = Sign { 0: [0; 32] };
        let account_id_2 = Sign { 0: [1; 32] };

        check_error(
            set_price(account_id_1, asset::EQ, 1., 0),
            "NotAllowedToSubmitPrice",
        );
        assert_err!(
            set_price(account_id_1, asset::EQ, 2., 0),
            Error::<Test>::NotAllowedToSubmitPrice
        );

        Whitelist::add_to_whitelist(&account_id_1);
        Whitelist::add_to_whitelist(&account_id_2);

        assert_err!(
            set_price(account_id_1, asset::EQ, 0., 0),
            Error::<Test>::PriceIsZero
        );
        assert_err!(
            set_price(account_id_1, asset::EQ, -1., 0),
            Error::<Test>::PriceIsNegative
        );
        assert_err!(
            set_price(account_id_1, asset::EQD, 1., 0),
            Error::<Test>::WrongCurrency
        );
        /*assert_err!(
            set_price(account_id_1, Currency::Unknown, 1.),
            Error::<Test>::WrongCurrency
        );*/

        set_price_ok(account_id_1, asset::EQ, 100_000.17, 1);
        set_price_ok(account_id_2, asset::EQ, 200_000.13, 1);

        set_price_ok(account_id_1, asset::BTC, 10.19, 1);
        set_price_ok(account_id_2, asset::BTC, 20.23, 1);

        check_price(asset::EQ, 150_000.15);
        check_price(asset::BTC, 15.21);

        // Timestamp::set_timestamp(2000); todo check
        System::set_block_number(2);

        set_price_ok(account_id_1, asset::EQ, 10_000., 2);
        set_price_ok(account_id_2, asset::EQ, 20_000., 2);

        check_price(asset::EQ, 15_000.);
    });
}

#[test]
fn set_price_not_from_whitelist() {
    new_test_ext().execute_with(|| {
        let account_id_1 = Sign { 0: [0; 32] };

        check_error(
            set_price(account_id_1, asset::EQ, 1., 0),
            "NotAllowedToSubmitPrice",
        );
        assert_err!(
            set_price(account_id_1, asset::EQ, 2., 0),
            Error::<Test>::NotAllowedToSubmitPrice
        );
    });
}

#[test]
fn set_price_from_whitelist() {
    new_test_ext().execute_with(|| {
        let account_id_1 = Sign { 0: [0; 32] };

        Whitelist::add_to_whitelist(&account_id_1);
        set_price_ok(account_id_1, asset::EQ, 100_000., 0);
    });
}

#[test]
fn set_median_price() {
    new_test_ext().execute_with(|| {
        let account_id_1 = Sign { 0: [1; 32] };
        let account_id_2 = Sign { 0: [2; 32] };
        let account_id_3 = Sign { 0: [3; 32] };
        let account_id_4 = Sign { 0: [4; 32] };
        let account_id_5 = Sign { 0: [5; 32] };
        let account_id_6 = Sign { 0: [6; 32] };
        let account_id_7 = Sign { 0: [7; 32] };

        Whitelist::add_to_whitelist(&account_id_1);
        Whitelist::add_to_whitelist(&account_id_2);
        Whitelist::add_to_whitelist(&account_id_3);
        Whitelist::add_to_whitelist(&account_id_4);
        Whitelist::add_to_whitelist(&account_id_5);
        Whitelist::add_to_whitelist(&account_id_6);
        set_price_ok(account_id_1, asset::EQ, 35_000., 0);
        check_price(asset::EQ, 35_000.);
        System::set_block_number(2);
        set_price_ok(account_id_1, asset::EQ, 40_000., 2);
        check_price(asset::EQ, 40_000.);
        set_price_ok(account_id_2, asset::EQ, 50_000., 2);
        check_price(asset::EQ, 45_000.);
        set_price_ok(account_id_3, asset::EQ, 130_000., 2);
        check_price(asset::EQ, 50_000.);
        set_price_ok(account_id_4, asset::EQ, 1_000., 2);
        check_price(asset::EQ, 45_000.);
        set_price_ok(account_id_5, asset::EQ, 120_000., 2);
        check_price(asset::EQ, 50_000.);
        set_price_ok(account_id_6, asset::EQ, 2_000., 2);
        check_price(asset::EQ, 45_000.);
        System::set_block_number(3);
        set_price_ok(account_id_1, asset::EQ, 60_000., 3);
        check_price(asset::EQ, 55_000.);

        Whitelist::remove_from_whitelist(&account_id_1);
        Whitelist::remove_from_whitelist(&account_id_2);
        System::set_block_number(4);
        set_price_ok(account_id_3, asset::EQ, 5_000., 4);
        check_price(asset::EQ, 27_500.);

        Whitelist::add_to_whitelist(&account_id_7);
        System::set_block_number(5);
        set_price_ok(account_id_3, asset::EQ, 70_000., 5);
        check_price(asset::EQ, 55_000.);

        // data_point price timeout
        System::set_block_number(6);
        set_price_ok(account_id_3, asset::EQ, 30_000., 6);
        Timestamp::set_timestamp(2000);
        set_price_ok(account_id_4, asset::EQ, 40_000., 6);
        set_price_ok(account_id_5, asset::EQ, 50_000., 6);
        set_price_ok(account_id_6, asset::EQ, 60_000., 6);
        set_price_ok(account_id_7, asset::EQ, 70_000., 6);
        check_price(asset::EQ, 55_000.);
    });
}

#[test]
fn set_price_twice_block_moved() {
    new_test_ext().execute_with(|| {
        let account_id_1 = Sign { 0: [0; 32] };

        Timestamp::set_timestamp(2000);
        System::set_block_number(1);

        Whitelist::add_to_whitelist(&account_id_1);

        set_price_ok(account_id_1, asset::EQ, 10_000., 1);

        System::set_block_number(2);

        set_price_ok(account_id_1, asset::EQ, 20_000., 2);
    });
}

#[test]
fn set_price_twice_time_moved() {
    new_test_ext().execute_with(|| {
        let account_id_1 = Sign { 0: [0; 32] };

        Timestamp::set_timestamp(2000);
        System::set_block_number(1);

        Whitelist::add_to_whitelist(&account_id_1);

        set_price_ok(account_id_1, asset::EQ, 10_000., 1);

        Timestamp::set_timestamp(3000);

        check_error(
            set_price(account_id_1, asset::EQ, 20_000., 1),
            "PriceAlreadyAdded",
        );
    });
}

#[test]
fn not_set_price_twice() {
    new_test_ext().execute_with(|| {
        let account_id_1 = Sign { 0: [0; 32] };

        Timestamp::set_timestamp(2000);
        System::set_block_number(1);

        Whitelist::add_to_whitelist(&account_id_1);
        set_price_ok(account_id_1, asset::EQ, 10_000., 1);

        assert_err!(
            set_price(account_id_1, asset::EQ, 20_000., 1),
            Error::<Test>::PriceAlreadyAdded
        );
    });
}

#[test]
fn check_json_reader() {
    new_test_ext().execute_with(|| {
        assert_err!(
            JsonPriceSource::<Asset, ()>::fetch_price_from_json::<FixedI64>("".to_string(), "USD"),
            PriceSourceError::DeserializationError
        );
        assert_err!(
            JsonPriceSource::<Asset, ()>::fetch_price_from_json::<FixedI64>(
                "rtdfgfdgfdgf".to_string(),
                "USD"
            ),
            PriceSourceError::DeserializationError
        );
        assert_err!(
            JsonPriceSource::<Asset, ()>::fetch_price_from_json::<FixedI64>(
                "{USD:2.98}".to_string(),
                "USD"
            ),
            PriceSourceError::DeserializationError
        );
        assert_err!(
            JsonPriceSource::<Asset, ()>::fetch_price_from_json::<FixedI64>(
                "{\"USD\":'2.98'}".to_string(),
                "USD"
            ),
            PriceSourceError::DeserializationError
        );

        let val = FixedI64::from_inner((2.98 * (FixedI64::accuracy() as f64)) as i64);
        assert_eq!(
            JsonPriceSource::<Asset, ()>::fetch_price_from_json::<FixedI64>(
                "{\"USD\":2.98}".to_string(),
                "USD"
            ),
            Ok(val)
        );
        assert_eq!(
            JsonPriceSource::<Asset, ()>::fetch_price_from_json::<FixedI64>(
                "{\"USD\":\"2.98\"}".to_string(),
                "USD"
            ),
            Ok(val)
        );

        assert_err!(
            JsonPriceSource::<Asset, ()>::fetch_price_from_json::<FixedI64>(
                "{\"price\":\"2.98\"}".to_string(),
                "USD"
            ),
            PriceSourceError::JsonParseError
        );

        assert_err!(
            JsonPriceSource::<Asset, ()>::fetch_price_from_json::<FixedI64>(
                "{\"price\":\"2.98\"}".to_string(),
                "USD"
            ),
            PriceSourceError::JsonParseError
        );

        assert_eq!(
            JsonPriceSource::<Asset, ()>::fetch_price_from_json::<FixedI64>(
                "{\"price\": {\"last\": \"2.98\"}}".to_string(),
                "price.last"
            ),
            Ok(val)
        );

        assert_eq!(
            JsonPriceSource::<Asset, ()>::fetch_price_from_json::<FixedI64>(
                "{\"price\": [\"3.46\", \"2.98\"]}".to_string(),
                "price[1]"
            ),
            Ok(val)
        );

        assert_eq!(
            JsonPriceSource::<Asset, ()>::fetch_price_from_json::<FixedI64>(
                "{\"price\": {\"last\": [\"2.98\"]}}".to_string(),
                "price.last[0]"
            ),
            Ok(val)
        );

        assert_eq!(
            JsonPriceSource::<Asset, ()>::fetch_price_from_json::<FixedI64>(
                "[\"2.98\"]".to_string(),
                "[0]"
            ),
            Ok(val)
        );

        assert_eq!(
            JsonPriceSource::<Asset, ()>::fetch_price_from_json::<FixedI64>(
                "{\"data\": [ {\"data\": [ { \"price\": \"2.98\" } ] } ] }".to_string(),
                "data[0].data[0].price"
            ),
            Ok(val)
        );
    });
}

#[test]
fn invalid_prices() {
    new_test_ext().execute_with(|| {
        let account_id_1 = Sign { 0: [0; 32] };
        Whitelist::add_to_whitelist(&account_id_1);
        assert_eq!(Whitelist::contains(&account_id_1), true);
        assert_err!(
            set_price(account_id_1, asset::EQ, 0., 0),
            Error::<Test>::PriceIsZero
        );
        assert_err!(
            set_price(account_id_1, asset::EQ, -1., 0),
            Error::<Test>::PriceIsNegative
        );
    });
}

#[test]
fn test_timeout() {
    new_test_ext().execute_with(|| {
        let mut time: u64 = 0;
        time_move(&mut time, 10000);
        time_move(&mut time, 10000);
        time_move(&mut time, 10000);

        let account_id = Sign { 0: [0; 32] };

        Whitelist::add_to_whitelist(&account_id);

        set_price_ok(account_id, asset::EQ, 0.000_000_001, 0);
        println!("{:?}", PricePoints::<Test>::get(asset::EQ));
        check_price(asset::EQ, 0.000_000_001);

        time_move(&mut time, 7199);

        check_price(asset::EQ, 0.000_000_001);

        time_move(&mut time, 1);

        assert_err!(Oracle::get_price(asset::EQ), Error::<Test>::PriceTimeout);
    });
}

#[test]
fn invalid_currencies() {
    new_test_ext().execute_with(|| {
        let account_id_1 = Sign { 0: [0; 32] };
        Whitelist::add_to_whitelist(&account_id_1);

        assert_err!(
            set_price(account_id_1, asset::EQD, 1., 0),
            Error::<Test>::WrongCurrency
        );
        /*assert_err!(
            set_price(account_id_1, Currency::Unknown, 1.),
            Error::<Test>::WrongCurrency
        );*/
    });
}

#[test]
fn should_build_on_genesis_price_points() {
    new_test_ext().execute_with(|| {
        let default_price_point = PriceData {
            block_number: frame_system::Pallet::<Test>::block_number(),
            timestamp: 0,
            price: FixedI64::saturating_from_integer(0i32),
            price_points: Vec::<
                PricePoint<
                    <mock::Test as frame_system::Config>::AccountId,
                    <mock::Test as frame_system::Config>::BlockNumber,
                    <mock::Test as crate::Config>::Price,
                >,
            >::new(),
        };

        assert_eq!(<PricePoints<Test>>::contains_key(asset::EQ), true);
        assert_eq!(<PricePoints<Test>>::contains_key(asset::BTC), true);
        assert_eq!(<PricePoints<Test>>::contains_key(asset::ETH), true);

        assert_eq!(
            Oracle::price_points(asset::EQ).unwrap(),
            default_price_point
        );
        assert_eq!(
            Oracle::price_points(asset::ETH).unwrap(),
            default_price_point
        );
        assert_eq!(
            Oracle::price_points(asset::BTC).unwrap(),
            default_price_point
        );
    });
}

#[test]
fn set_price_when_stored_price_newer_should_fail() {
    new_test_ext().execute_with(|| {
        let account_id_1 = Sign { 0: [0; 32] };
        let account_id_2 = Sign { 0: [2; 32] };
        Whitelist::add_to_whitelist(&account_id_1);
        Whitelist::add_to_whitelist(&account_id_2);

        System::set_block_number(2);

        set_price_ok(account_id_1, asset::EQ, 100_000.17, 2);

        set_price_ok(account_id_2, asset::EQ, 100_000., 1);

        assert_err!(
            set_price(account_id_1, asset::EQ, 100_100., 1),
            Error::<Test>::PriceAlreadyAdded
        );

        assert_err!(
            set_price(account_id_1, asset::EQ, 100_100., 2),
            Error::<Test>::PriceAlreadyAdded
        );
    });
}

#[test]
fn filter_prices_from_test() {
    new_test_ext().execute_with(|| {
        let account_id_1 = Sign { 0: [1; 32] };
        let account_id_2 = Sign { 0: [2; 32] };
        let account_id_3 = Sign { 0: [3; 32] };
        Whitelist::add_to_whitelist(&account_id_1);
        Whitelist::add_to_whitelist(&account_id_2);
        Whitelist::add_to_whitelist(&account_id_3);

        set_price_ok(account_id_1, asset::EQ, 80_000., 1);
        set_price_ok(account_id_2, asset::EQ, 90_000., 1);
        set_price_ok(account_id_3, asset::EQ, 100_000., 1);

        let price_point = <PricePoints<Test>>::get(asset::EQ).unwrap();

        assert_eq!(price_point.price, FixedI64::saturating_from_integer(90_000));
        Oracle::filter_prices_from(&account_id_1);

        let price_point = <PricePoints<Test>>::get(asset::EQ).unwrap();

        assert_eq!(price_point.price, FixedI64::saturating_from_integer(95_000));

        for data_point in price_point.price_points {
            assert!(data_point.account_id != account_id_1);
        }

        set_price_ok(account_id_1, asset::EQ, 110_000., 1);

        let price_point = <PricePoints<Test>>::get(asset::EQ).unwrap();

        assert_eq!(
            price_point.price,
            FixedI64::saturating_from_integer(100_000)
        );
    });
}

#[test]
fn url_symbol_case() {
    let huobi_url_template = "https://api.huobi.pro/market/history/trade?symbol={$}usdt&size=1";
    let huobi_url = asset::BTC.get_url(huobi_url_template, "");

    assert!(huobi_url.is_ok());

    assert_eq!(
        huobi_url.unwrap().0,
        "https://api.huobi.pro/market/history/trade?symbol=btcusdt&size=1"
    );

    let kraken_url_template = "https://api.kraken.com/0/public/Ticker?pair={$}USD";
    let kraken_url = asset::BTC.get_url(kraken_url_template, "");

    assert!(kraken_url.is_ok());

    assert_eq!(
        kraken_url.unwrap().0,
        "https://api.kraken.com/0/public/Ticker?pair=XXBTZUSD"
    );
}
