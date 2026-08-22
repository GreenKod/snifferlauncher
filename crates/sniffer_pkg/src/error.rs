//! Error types for the package layer.

use std::fmt;

/// All errors that can originate from the package layer.
#[derive(Debug)]
pub enum PackageError {
    /// Returned by [`super::package::LauncherPackage::on_init`] or
    /// [`super::package::WidgetPackage::on_init`] when the package cannot
    /// start (e.g. missing hardware, permission denied, thread spawn failure).
    Init(String),

    /// The requested `method` string is not recognised by this package.
    UnsupportedMethod(String),

    /// No package with the given ID is registered in the
    /// [`super::registry::PackageRegistry`].
    NotFound(String),

    /// The JSON descriptor passed to
    /// [`super::package::WidgetPackage::apply_descriptor`] could not be
    /// parsed or contains invalid values.
    InvalidDescriptor(String),

    /// A dynamic-library operation failed (only available with the
    /// `dynamic` feature).
    #[cfg(feature = "dynamic")]
    DynLoad(String),

    /// A file-system error occurred (e.g. while copying a `.so` to the
    /// internal package directory on Android).
    Io(std::io::Error),

    /// A mutex or RwLock inside the package was poisoned.
    Poisoned(String),
}

impl fmt::Display for PackageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Init(msg) => write!(f, "package init failed: {msg}"),
            Self::UnsupportedMethod(m) => write!(f, "unsupported method: '{m}'"),
            Self::NotFound(id) => write!(f, "package not found: '{id}'"),
            Self::InvalidDescriptor(msg) => write!(f, "invalid descriptor: {msg}"),
            #[cfg(feature = "dynamic")]
            Self::DynLoad(msg) => write!(f, "dynamic load error: {msg}"),
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Poisoned(ctx) => write!(f, "lock poisoned in: {ctx}"),
        }
    }
}

impl std::error::Error for PackageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let Self::Io(e) = self {
            Some(e)
        } else {
            None
        }
    }
}

impl From<std::io::Error> for PackageError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

#[cfg(feature = "dynamic")]
impl From<libloading::Error> for PackageError {
    fn from(e: libloading::Error) -> Self {
        Self::DynLoad(e.to_string())
    }
}
