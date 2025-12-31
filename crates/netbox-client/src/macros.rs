//! Macros for NetBoxClient trait implementations
//!
//! This module provides macros to reduce boilerplate in trait implementations.
//!
//! **NOTE: Current Limitation**
//!
//! The `impl_netbox_delegate!` macro does NOT work with `async_trait::async_trait` due to
//! lifetime parameter expansion conflicts. When `async_trait` expands async methods, it adds
//! internal lifetime parameters (`'async_trait`) that macro-generated code cannot match,
//! resulting in `E0195: lifetime parameters or bounds on method do not match the trait declaration`.
//!
//! **Native Async Traits Status:**
//!
//! Native `async fn` in traits (RFC 3185) is **stable** in Rust 1.75+ (December 2023).
//! However, `dyn Trait` support is NOT yet available, so we still need `async_trait` for
//! trait objects (`Box<dyn NetBoxClientTrait>`). Once `dyn` support lands, we could potentially
//! use native async traits and the macro might work better.
//!
//! **Workaround:**
//!
//! Use manual implementations for async trait methods. The delegation pattern is simple:
//! ```rust
//! async fn method_name(&self, params...) -> ReturnType {
//!     module::function(&self.core, params...).await
//! }
//! ```
//!
//! **Future Solutions:**
//!
//! 1. Use a proc-macro instead of `macro_rules!` (more complex but might work)
//! 2. Wait for `dyn Trait` support with native async traits (in progress)
//! 3. Accept manual implementations (current approach - works reliably)

// Macro to generate the delegation body (helper for manual implementations)
// Currently unused - kept for future reference
#[allow(unused_macros)]
#[macro_export]
macro_rules! delegate_to_module {
    ($core:expr, $module_path:path, $($param:expr),*) => {
        $module_path($core, $($param),*).await
    };
}

// NOTE: This macro does NOT work with async_trait due to lifetime expansion conflicts
// Keeping it here for reference/future investigation
// Commented out to avoid unused macro warnings
/*
#[macro_export]
macro_rules! impl_netbox_delegate {
    // Simple delegation - direct pass-through
    // DOES NOT WORK with async_trait - generates E0195 lifetime mismatch errors
    (
        $(
            $method:ident($($param:ident: $param_type:ty),*) -> $return_type:ty => $module_path:path;
        )+
    ) => {
        $(
            async fn $method(&self, $($param: $param_type),*) -> $return_type {
                $crate::delegate_to_module!(&self.core, $module_path, $($param),*)
            }
        )+
    };
}
*/

