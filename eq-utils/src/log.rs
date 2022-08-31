pub use crate::{debug, error, info, trace, warn};
pub use log as log_ext;

#[macro_export]
macro_rules! trace {
    (target: $target:expr, $($arg:tt)*) => {
        utils::log::log_ext::trace!(
            target: $target,
            "{}:{}. {}",
            file!(),
            line!(),
            alloc::format!($($arg)*),
        )
    };
    ($($arg:tt)*) => {
        utils::log::log_ext::trace!(
            "{}:{}. {}",
            file!(),
            line!(),
            alloc::format!($($arg)*),
        )
    };
}

#[macro_export]
macro_rules! debug {
    (target: $target:expr, $($arg:tt)*) => {
        utils::log::log_ext::debug!(
            target: $target,
            "{}:{}. {}",
            file!(),
            line!(),
            alloc::format!($($arg)*),
        )
    };
    ($($arg:tt)*) => {
        utils::log::log_ext::debug!(
            "{}:{}. {}",
            file!(),
            line!(),
            alloc::format!($($arg)*),
        )
    };
}

#[macro_export]
macro_rules! info {
    (target: $target:expr, $($arg:tt)*) => {
        utils::log::log_ext::info!(
            target: $target,
            "{}:{}. {}",
            file!(),
            line!(),
            alloc::format!($($arg)*),
        )
    };
    ($($arg:tt)*) => {
        utils::log::log_ext::info!(
            "{}:{}. {}",
            file!(),
            line!(),
            alloc::format!($($arg)*),
        )
    };
}

#[macro_export]
macro_rules! warn {
    (target: $target:expr, $($arg:tt)*) => {
        utils::log::log_ext::warn!(
            target: $target,
            "{}:{}. {}",
            file!(),
            line!(),
            alloc::format!($($arg)*),
        )
    };
    ($($arg:tt)*) => {
        utils::log::log_ext::warn!(
            "{}:{}. {}",
            file!(),
            line!(),
            alloc::format!($($arg)*),
        )
    };
}

#[macro_export]
macro_rules! error {
    (target: $target:expr, $($arg:tt)*) => {
        utils::log::log_ext::error!(
            target: $target,
            "{}:{}. {}",
            file!(),
            line!(),
            alloc::format!($($arg)*),
        )
    };
    ($($arg:tt)*) => {
        utils::log::log_ext::error!(
            "{}:{}. {}",
            file!(),
            line!(),
            alloc::format!($($arg)*),
        )
    };
}
