#[macro_use]
extern crate rocket;

pub mod api;
pub mod db;
pub mod utils;

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
#[ctor::ctor]
fn init() {
    crate::utils::logging::setup_console_log();
}
