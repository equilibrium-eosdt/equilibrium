extern crate alloc;
use alloc::string::String;
use core::str::FromStr;
use sp_runtime::offchain::{
    storage::{StorageRetrievalError, StorageValueRef},
    StorageKind,
};

const ID_KEY: &[u8] = b"exec_id";
const LOCK_KEY: &[u8] = b"lock";
const EXEC_ID_KEY: &[u8] = b"execution-id/";

#[derive(Debug)]
pub enum LockedExecResult {
    Locked,
    Executed,
}

pub fn acquire_lock<F>(prefix: &[u8], f: F) -> LockedExecResult
where
    F: Fn(),
{
    let lock_key = [prefix, LOCK_KEY].concat();
    let mut lock_storage = StorageValueRef::persistent(&lock_key);

    let exec_id_opt = StorageValueRef::persistent(EXEC_ID_KEY).get();
    if let Ok(Some(exec_id)) = exec_id_opt {
        let id_key = [prefix, ID_KEY].concat();
        let id_storage = StorageValueRef::persistent(&id_key);
        let need_to_clear_lock = id_storage.mutate(
            |id: Result<Option<[u8; 32]>, StorageRetrievalError>| match id {
                Ok(Some(val)) => {
                    if val != exec_id {
                        // new id we need to clear lock because of first launch
                        Ok(exec_id)
                    } else {
                        Err(())
                    }
                }
                _ => {
                    // no id we need to clear lock because of first launch
                    Ok(exec_id)
                }
            },
        );

        if need_to_clear_lock.is_ok() {
            lock_storage.clear();
        }
    }

    let can_process = lock_storage.mutate(
        |is_locked: Result<Option<bool>, StorageRetrievalError>| match is_locked {
            Ok(Some(true)) => Err(()),
            _ => Ok(true),
        },
    );

    match can_process {
        Ok(true) => {
            f();
            lock_storage.clear();
            LockedExecResult::Executed
        }
        _ => LockedExecResult::Locked,
    }
}

/// Gets a value by the key
pub fn get_local_storage_val<R: FromStr>(key: &[u8]) -> Option<R> {
    let raw_val = sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, key);
    match raw_val {
        Some(val_bytes) => match String::from_utf8(val_bytes.clone()) {
            Ok(val_decoded) => match val_decoded.parse::<R>() {
                Ok(val) => Some(val),
                Err(_e) => {
                    log::warn!("Can't parse local storage value {:?}", val_decoded);
                    None
                }
            },
            Err(_e) => {
                log::warn!("Can't decode local storage key {:?}: {:?}", key, val_bytes);
                None
            }
        },
        None => {
            log::warn!("Uninitialized local storage key: {:?}", key);
            None
        }
    }
}
