#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

pub trait WeightInfo {
    fn set_price(b: u32) -> Weight;
}

// for tests
impl crate::WeightInfo for () {
    fn set_price(_b: u32) -> Weight {
        0 as Weight
    }
}
