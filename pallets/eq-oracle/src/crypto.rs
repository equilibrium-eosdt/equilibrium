//! Module for signing operations

use core::convert::TryFrom;
use sp_core::sr25519::Signature as Sr25519Signature;
use sp_runtime::app_crypto::{app_crypto, sr25519};
use sp_runtime::traits::Verify;
use sp_runtime::MultiSignature;

use super::KEY_TYPE;

app_crypto!(sr25519, KEY_TYPE);

/// Struct for implementation of AppCrypto
pub struct AuthId;
impl frame_system::offchain::AppCrypto<<MultiSignature as Verify>::Signer, MultiSignature>
    for AuthId
{
    type RuntimeAppPublic = Public;
    type GenericPublic = sp_core::sr25519::Public;
    type GenericSignature = sp_core::sr25519::Signature;
}

/// Struct for implementation of AppCrypto, used in unit tests
pub struct TestAuthId;
impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
    for TestAuthId
{
    type RuntimeAppPublic = Public;
    type GenericPublic = sp_core::sr25519::Public;
    type GenericSignature = sp_core::sr25519::Signature;
}
