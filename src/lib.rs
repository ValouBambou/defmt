#![cfg_attr(not(target_arch = "x86_64"), no_std)]

use core::{mem::MaybeUninit, ptr::NonNull};

#[doc(hidden)]
pub mod export;
mod impls;
mod leb;
#[cfg(test)]
mod tests;

/// Creates an interned string ([`Str`]) from a string literal.
///
/// This must be called on a string literal, and will allocate the literal in the object file. At
/// runtime, only a small string index is required to refer to the string, represented as the
/// [`Str`] type.
///
/// # Example
///
/// ```
/// let interned = binfmt::intern!("long string literal taking up little space");
/// ```
///
/// [`Str`]: struct.Str.html
pub use binfmt_macros::intern;

/// Logs data at *debug* level.
pub use binfmt_macros::debug;
/// Logs data at *error* level.
pub use binfmt_macros::error;
/// Logs data at *info* level.
pub use binfmt_macros::info;
/// Logs data at *trace* level.
pub use binfmt_macros::trace;
/// Logs data at *warn* level.
pub use binfmt_macros::warn;

/// Defines the global binfmt logger.
///
/// `#[global_logger]` needs to be put on a unit struct type declaration. This struct has to
/// implement the [`Logger`] trait.
///
/// # Example
///
/// ```
/// use binfmt::{Logger, Write, global_logger};
/// use core::ptr::NonNull;
///
/// #[global_logger]
/// struct MyLogger;
///
/// unsafe impl Logger for MyLogger {
///     fn acquire() -> Option<NonNull<dyn Write>> {
/// # todo!()
///         // ...
///     }
///     unsafe fn release(writer: NonNull<dyn Write>) {
/// # todo!()
///         // ...
///     }
/// }
/// ```
///
/// [`Logger`]: trait.Logger.html
pub use binfmt_macros::global_logger;

/// Defines the global timestamp provider for binfmt.
///
/// Every message logged with binfmt will include a timestamp. The function annotated with
/// `#[timestamp]` will be used to obtain this timestamp.
///
/// The `#[timestamp]` attribute needs to be applied to a function with the signature `fn() -> u64`.
/// The returned `u64` is the current timestamp in microseconds.
///
/// Some systems might not have a timer available. In that case, a dummy implementation such as this
/// may be used:
///
/// ```
/// # use binfmt_macros::timestamp;
/// #[timestamp]
/// fn dummy_timestamp() -> u64 {
///     0
/// }
/// ```
pub use binfmt_macros::timestamp;

/// Writes binfmt-formatted data to a [`Formatter`].
///
/// This works similarly to the `write!` macro in libcore.
///
/// Usage:
///
/// ```
/// # use binfmt::{Format, Formatter};
/// # struct S;
/// # impl Format for S {
/// #     fn format(&self, formatter: &mut Formatter) {
/// #         let arguments = 0u8;
/// binfmt::write!(formatter, "format string {:?}", arguments)
/// #     }
/// # }
/// ```
///
/// [`Formatter`]: struct.Formatter.html
pub use binfmt_macros::write;

#[doc(hidden)]
pub use binfmt_macros::winfo;
#[doc(hidden)] // documented as the `Format` trait instead
pub use binfmt_macros::Format;

/// Global logger acquire-release mechanism.
///
/// # Safety contract
///
/// - `acquire` returns a handle that temporarily *owns* the global logger
/// - `acquire` must return `Some` only once, until the handle is `release`-d
/// - `acquire` is allowed to return a handle per thread or interrupt level
/// - `acquire` is a safe function therefore it must be thread-safe and interrupt-safe
/// - The value returned by `acquire` is not `Send` so it cannot be moved between threads or
/// interrupt handlers
///
/// And, not safety related, `acquire` should never be invoked from user code. The easiest way to
/// ensure this is to implement `Logger` on a *private* `struct` and mark that `struct` as the
/// `#[global_logger]`.
pub unsafe trait Logger {
    fn acquire() -> Option<NonNull<dyn Write>>;
    /// # Safety
    /// `writer` argument must be a value previously returned by `Self::acquire` and not, say,
    /// `NonNull::dangling()`
    unsafe fn release(writer: NonNull<dyn Write>);
}

/// An interned string created via [`intern!`].
///
/// [`intern!`]: macro.intern.html
#[derive(Clone, Copy)]
pub struct Str {
    // 14-bit address
    address: u16,
}

/// Handle to a binfmt logger.
pub struct Formatter {
    #[cfg(not(target_arch = "x86_64"))]
    writer: NonNull<dyn Write>,
    #[cfg(target_arch = "x86_64")]
    bytes: Vec<u8>,
}

impl Formatter {
    /// Only for testing on x86_64
    #[cfg(target_arch = "x86_64")]
    pub fn new() -> Self {
        Self { bytes: vec![] }
    }

    /// Only for testing on x86_64
    #[cfg(target_arch = "x86_64")]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    #[doc(hidden)]
    #[cfg(target_arch = "x86_64")]
    pub fn write(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes)
    }

    #[doc(hidden)]
    #[cfg(not(target_arch = "x86_64"))]
    pub fn write(&mut self, bytes: &[u8]) {
        unsafe { self.writer.as_mut().write(bytes) }
    }

    /// Implementation detail
    #[cfg(target_arch = "x86_64")]
    #[doc(hidden)]
    pub unsafe fn from_raw(_: NonNull<dyn Write>) -> Self {
        unreachable!()
    }

    /// Implementation detail
    #[cfg(not(target_arch = "x86_64"))]
    #[doc(hidden)]
    pub unsafe fn from_raw(writer: NonNull<dyn Write>) -> Self {
        Self { writer }
    }

    /// Implementation detail
    #[cfg(target_arch = "x86_64")]
    #[doc(hidden)]
    pub unsafe fn into_raw(self) -> NonNull<dyn Write> {
        unreachable!()
    }

    /// Implementation detail
    #[cfg(not(target_arch = "x86_64"))]
    #[doc(hidden)]
    pub unsafe fn into_raw(self) -> NonNull<dyn Write> {
        self.writer
    }

    // TODO turn these public methods in `export` free functions
    /// Implementation detail
    #[doc(hidden)]
    pub fn fmt(&mut self, f: &impl Format) {
        f.format(self)
    }

    /// Implementation detail
    #[doc(hidden)]
    pub fn leb64(&mut self, x: u64) {
        let mut buf: [u8; 10] = unsafe { MaybeUninit::uninit().assume_init() };
        let i = unsafe { leb::leb64(x, &mut buf) };
        self.write(unsafe { buf.get_unchecked(..i) })
    }

    /// Implementation detail
    #[doc(hidden)]
    pub fn i8(&mut self, b: &i8) {
        self.write(&b.to_le_bytes())
    }

    /// Implementation detail
    #[doc(hidden)]
    pub fn i16(&mut self, b: &i16) {
        self.write(&b.to_le_bytes())
    }

    /// Implementation detail
    #[doc(hidden)]
    pub fn i32(&mut self, b: &i32) {
        self.write(&b.to_le_bytes())
    }

    // TODO remove
    /// Implementation detail
    #[doc(hidden)]
    pub fn prim(&mut self, s: &Str) {
        self.write(&[s.address as u8])
    }

    /// Implementation detail
    #[doc(hidden)]
    pub fn u8(&mut self, b: &u8) {
        self.write(&[*b])
    }

    /// Implementation detail
    #[doc(hidden)]
    pub fn u16(&mut self, b: &u16) {
        self.write(&b.to_le_bytes())
    }

    /// Implementation detail
    #[doc(hidden)]
    pub fn u24(&mut self, b: &u32) {
        self.write(&b.to_le_bytes()[..3])
    }

    /// Implementation detail
    #[doc(hidden)]
    pub fn u32(&mut self, b: &u32) {
        self.write(&b.to_le_bytes())
    }

    #[doc(hidden)]
    pub fn str(&mut self, s: &str) {
        self.leb64(s.len() as u64);
        self.write(s.as_bytes());
    }

    /// Implementation detail
    #[doc(hidden)]
    pub fn istr(&mut self, s: &Str) {
        // LEB128 encoding
        if s.address < 128 {
            self.write(&[s.address as u8])
        } else {
            self.write(&[s.address as u8 | (1 << 7), (s.address >> 7) as u8])
        }
    }
}

/// Trait for binfmt logging targets.
pub trait Write {
    /// Writes `bytes` to the destination.
    ///
    /// This will be called by the binfmt logging macros to transmit encoded data. The write
    /// operation must not fail.
    fn write(&mut self, bytes: &[u8]);
}

/// Derivable trait for binfmt output.
///
/// This trait is used by the `{:?}` format specifier and can format a wide range of types.
/// User-defined types can `#[derive(Format)]` to get an auto-generated implementation of this
/// trait.
///
/// # Example
///
/// It is recommended to `#[derive]` implementations of this trait:
///
/// ```
/// use binfmt::Format;
///
/// #[derive(Format)]
/// struct Header {
///     source: u8,
///     destination: u8,
///     sequence: u16,
/// }
/// ```
///
/// If necessary, implementations can also be written manually:
///
/// ```
/// use binfmt::{Format, Formatter};
///
/// struct Header {
///     source: u8,
///     destination: u8,
///     sequence: u16,
/// }
///
/// impl Format for Header {
///     fn format(&self, fmt: &mut Formatter) {
///         binfmt::write!(
///             fmt,
///             "Header {{ source: {:u8}, destination: {:u8}, sequence: {:u16} }}",
///             self.source,
///             self.destination,
///             self.sequence
///         )
///     }
/// }
/// ```
pub trait Format {
    /// Writes the binfmt representation of `self` to `fmt`.
    fn format(&self, fmt: &mut Formatter);
}
