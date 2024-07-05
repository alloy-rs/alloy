cfg_if::cfg_if! {
    if #[cfg(feature = "kzg")] {
        pub use c_kzg::Error as KzgError;
    } else if #[cfg(feature = "kzg-rs")] {
        /// An error returned by [`kzg-rs`].
        pub use kzg_rs::KzgError,
    }
}
