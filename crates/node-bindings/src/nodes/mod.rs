//! Bindings for various nodes.

pub mod anvil;
pub mod geth;
pub mod reth;

#[cfg(test)]
mod test {
    /// Run the given function only if we are in a CI environment.
    pub(crate) fn ci_only<F>(f: F)
    where
        F: FnOnce(),
    {
        if ci_info::is_ci() {
            f();
        }
    }
}
