#[macro_export]
#[allow(unused_macros)]
macro_rules! check {
    ($result:expr) => {
        match $result {
            Ok(value) => Ok(value),
            Err(err) => {
                log::error!("Failed: {:?}", err);
                Err(err)
            }
        }
    };
}

#[macro_export]
#[allow(unused_macros)]
macro_rules! log_err {
    ($result:expr) => {
        let _ = $crate::check!($result);
    };
}

#[allow(unused_imports)]
pub use check;

#[allow(unused_imports)]
pub use log_err;
