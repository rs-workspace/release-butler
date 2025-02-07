#[cfg(feature = "tests")]
pub mod tests_utils;
pub mod webhook;

pub struct State {
    pub webhook_secret: String,
}
