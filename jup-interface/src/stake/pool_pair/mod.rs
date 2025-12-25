//! For PoolPairs, we assume that we do not have access to init_keyed_account, so all
//! structs must have an initial state that assumes no fetched accounts

mod common;
mod oneway;

pub use oneway::*;
