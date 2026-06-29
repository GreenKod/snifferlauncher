pub mod color;
pub mod layout;
pub mod parser;
pub mod props;

// Re-export everything so existing code using `crate::core::style::*` keeps working.
pub use color::*;
pub use layout::*;
pub use props::Style;
