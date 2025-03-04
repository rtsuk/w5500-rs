//! Platform agnostic rust driver for the [Wiznet W5500] SPI internet offload
//! chip.
//!
//! This is a low-level (ll) crate. The scope of this crate is:
//! 1) Register accessors.
//! 2) Networking data types.
//!
//! Higher level functionality (such as socket operations) should be built
//! on-top of what is provided here.
//!
//! # Example
//!
//! Reading the VERSIONR register (a constant value).
//!
//! ```
//! # let spi = ehm1::spi::Mock::new(&[
//! #   ehm1::spi::Transaction::transaction_start(),
//! #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x39, 0x00]),
//! #   ehm1::spi::Transaction::read(0x04),
//! #   ehm1::spi::Transaction::transaction_end(),
//! # ]);
//! use w5500_ll::{eh1::vdm::W5500, Registers};
//!
//! let mut w5500 = W5500::new(spi);
//! let version: u8 = w5500.version()?;
//! assert_eq!(version, 0x04);
//! # Ok::<(), eh1::spi::ErrorKind>(())
//! ```
//!
//! # Feature Flags
//!
//! All features are disabled by default.
//!
//! * `async`: **Nightly only.** Enables the asynchronous [`aio::Registers`]
//!   trait.
//! * `defmt`: Enable formatting most types with `defmt`.
//! * `eh0`: Enables the [`eh0`] module which contains
//!   implementations of the [`Registers`] trait
//!   using the `embedded-hal` version 0.2 traits.
//! * `eh1`: Enables the [`eh1`] module which contains
//!   implementations of the [`Registers`] trait
//!   using the `embedded-hal` version 1 traits.
//! * `eha0a`: **Nightly only.**
//!   Implements the [`aio::Registers`] trait for types in the [`eh1`] module
//!   using the `embedded-hal-async` alpha traits.
//! * `ip_in_core`: **Nightly only.**
//!   Use `core::net` types for `Ipv4Addr` and `SocketAddrV4`.
//! * `std`: Enables conversion between [`std::net`] and [`w5500_ll::net`] types.
//!   This is for testing purposes only, the `std` flag will not work on
//!   embedded systems because it uses the standard library.
//!
//! [`std::net`]: https://doc.rust-lang.org/std/net/index.html
//! [Wiznet W5500]: https://www.wiznet.io/product-item/w5500/
//! [`aio::Registers`]: https://docs.rs/w5500-ll/latest/w5500_ll/aio/trait.Registers.html
//! [`aio`]: https://docs.rs/w5500-ll/latest/w5500_ll/aio/index.html
//! [`eh0`]: https://docs.rs/w5500-ll/latest/w5500_ll/eh0/index.html
//! [`eh1`]: https://docs.rs/w5500-ll/latest/w5500_ll/eh1/index.html
//! [`Registers`]: https://docs.rs/w5500-ll/latest/w5500_ll/trait.Registers.html
//! [`w5500_ll::net`]: https://docs.rs/w5500-ll/latest/w5500_ll/net/index.html
#![cfg_attr(docsrs, feature(doc_cfg), feature(doc_auto_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(
    any(feature = "eha0a", feature = "async"),
    feature(async_fn_in_trait),
    allow(incomplete_features), // async_fn_in_trait
)]
#![cfg_attr(feature = "ip_in_core", feature(ip_in_core))]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

#[cfg(feature = "async")]
pub mod aio;

#[cfg(feature = "eh0")]
pub mod eh0;
#[cfg(feature = "eh1")]
pub mod eh1;

pub mod net;
pub mod spi;

mod addr;
mod registers;
mod specifiers;
use net::{Eui48Addr, Ipv4Addr, SocketAddrV4};

pub use addr::{Reg, SnReg};
pub use registers::{Interrupt, Mode, PhyCfg, SocketInterrupt, SocketInterruptMask, SocketMode};
pub use specifiers::{
    BufferSize, DuplexStatus, LinkStatus, OperationMode, Protocol, SocketCommand, SocketStatus,
    SpeedStatus,
};

/// Common register block address offset.
pub const COMMON_BLOCK_OFFSET: u8 = 0x00;
/// Socket spacing between blocks.
const SOCKET_SPACING: u8 = 0x04;
/// Socket common block select bits offset.
const SOCKET_BLOCK_OFFSET: u8 = 0x01;
/// Socket TX block select bits offset
const SOCKET_TX_OFFSET: u8 = 0x02;
/// Socket RX block select bits offset
const SOCKET_RX_OFFSET: u8 = 0x03;

/// Value of the W5500 VERSIONR register.
///
/// This is very useful as a sanity check to ensure the W5500 is out of reset
/// and responding correctly to register reads.
///
/// # Example
///
/// ```
/// # use ehm1 as hal;
/// # let spi = ehm1::spi::Mock::new(&[
/// #   ehm1::spi::Transaction::transaction_start(),
/// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x39, 0x00]),
/// #   ehm1::spi::Transaction::read(0x04),
/// #   ehm1::spi::Transaction::transaction_end(),
/// # ]);
/// use w5500_ll::{eh1::vdm::W5500, Registers, VERSION};
///
/// let mut w5500 = W5500::new(spi);
/// let version: u8 = w5500.version()?;
/// assert_eq!(version, VERSION);
/// # Ok::<(), eh1::spi::ErrorKind>(())
/// ```
pub const VERSION: u8 = 0x04;

/// W5500 socket numbers.
#[repr(u8)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Sn {
    /// Socket 0.
    ///
    /// This is the only socket that can be used in the [`Macraw`] mode.
    ///
    /// [`Macraw`]: crate::Protocol::Macraw
    Sn0 = 0,
    /// Socket 1.
    Sn1 = 1,
    /// Socket 2.
    Sn2 = 2,
    /// Socket 3.
    Sn3 = 3,
    /// Socket 4.
    Sn4 = 4,
    /// Socket 5.
    Sn5 = 5,
    /// Socket 6.
    Sn6 = 6,
    /// Socket 7.
    Sn7 = 7,
}

impl Sn {
    /// Get the socket register block select bits.
    ///
    /// # Example
    ///
    /// ```
    /// use w5500_ll::Sn;
    ///
    /// assert_eq!(Sn::Sn0.block(), 0b00001);
    /// assert_eq!(Sn::Sn1.block(), 0b00101);
    /// assert_eq!(Sn::Sn2.block(), 0b01001);
    /// assert_eq!(Sn::Sn3.block(), 0b01101);
    /// assert_eq!(Sn::Sn4.block(), 0b10001);
    /// assert_eq!(Sn::Sn5.block(), 0b10101);
    /// assert_eq!(Sn::Sn6.block(), 0b11001);
    /// assert_eq!(Sn::Sn7.block(), 0b11101);
    /// ```
    #[inline]
    pub const fn block(self) -> u8 {
        SOCKET_SPACING * (self as u8) + SOCKET_BLOCK_OFFSET
    }

    /// Get the socket TX buffer block select bits.
    ///
    /// # Example
    ///
    /// ```
    /// use w5500_ll::Sn;
    ///
    /// assert_eq!(Sn::Sn0.tx_block(), 0b00010);
    /// assert_eq!(Sn::Sn1.tx_block(), 0b00110);
    /// assert_eq!(Sn::Sn2.tx_block(), 0b01010);
    /// assert_eq!(Sn::Sn3.tx_block(), 0b01110);
    /// assert_eq!(Sn::Sn4.tx_block(), 0b10010);
    /// assert_eq!(Sn::Sn5.tx_block(), 0b10110);
    /// assert_eq!(Sn::Sn6.tx_block(), 0b11010);
    /// assert_eq!(Sn::Sn7.tx_block(), 0b11110);
    /// ```
    #[inline]
    pub const fn tx_block(self) -> u8 {
        SOCKET_SPACING * (self as u8) + SOCKET_TX_OFFSET
    }

    /// Get the socket RX buffer block select bits.
    ///
    /// # Example
    ///
    /// ```
    /// use w5500_ll::Sn;
    ///
    /// assert_eq!(Sn::Sn0.rx_block(), 0b00011);
    /// assert_eq!(Sn::Sn1.rx_block(), 0b00111);
    /// assert_eq!(Sn::Sn2.rx_block(), 0b01011);
    /// assert_eq!(Sn::Sn3.rx_block(), 0b01111);
    /// assert_eq!(Sn::Sn4.rx_block(), 0b10011);
    /// assert_eq!(Sn::Sn5.rx_block(), 0b10111);
    /// assert_eq!(Sn::Sn6.rx_block(), 0b11011);
    /// assert_eq!(Sn::Sn7.rx_block(), 0b11111);
    /// ```
    #[inline]
    pub const fn rx_block(self) -> u8 {
        SOCKET_SPACING * (self as u8) + SOCKET_RX_OFFSET
    }

    /// Socket bitmask.
    ///
    /// This is useful for masking socket interrupts with [`set_simr`].
    ///
    /// # Examples
    ///
    /// Demonstration:
    ///
    /// ```
    /// use w5500_ll::Sn;
    ///
    /// assert_eq!(Sn::Sn0.bitmask(), 0x01);
    /// assert_eq!(Sn::Sn1.bitmask(), 0x02);
    /// assert_eq!(Sn::Sn2.bitmask(), 0x04);
    /// assert_eq!(Sn::Sn3.bitmask(), 0x08);
    /// assert_eq!(Sn::Sn4.bitmask(), 0x10);
    /// assert_eq!(Sn::Sn5.bitmask(), 0x20);
    /// assert_eq!(Sn::Sn6.bitmask(), 0x40);
    /// assert_eq!(Sn::Sn7.bitmask(), 0x80);
    /// ```
    ///
    /// As an argument of [`set_simr`]:
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x18, 0x04]),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x0A]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{
    ///     eh1::vdm::W5500,
    ///     Registers,
    ///     Sn::{Sn1, Sn3},
    /// };
    ///
    /// let mut w5500 = W5500::new(spi);
    /// // enable socket 1 and socket 3 interrupts
    /// const SOCKET_INTERRUPT_MASK: u8 = Sn1.bitmask() | Sn3.bitmask();
    /// w5500.set_simr(SOCKET_INTERRUPT_MASK)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    ///
    /// [`set_simr`]: crate::Registers::set_simr
    pub const fn bitmask(self) -> u8 {
        1 << (self as u8)
    }

    /// Iterate over all sockets.
    ///
    /// # Example
    ///
    /// Check all sockets for a pending interrupt.
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x17, 0x00]),
    /// #   ehm1::spi::Transaction::read(0),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn, SocketCommand};
    ///
    /// let mut w5500 = W5500::new(spi);
    ///
    /// let sir: u8 = w5500.sir()?;
    /// for sn in Sn::iter() {
    ///     let mask: u8 = sn.bitmask();
    ///     if sir & sn.bitmask() != 0 {
    ///         // handle socket interrupt
    ///     }
    /// }
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    #[inline]
    pub fn iter() -> core::slice::Iter<'static, Self> {
        SOCKETS.iter()
    }
}

macro_rules! sn_conversion_for {
    ($ty:ident) => {
        impl From<Sn> for $ty {
            #[inline]
            fn from(s: Sn) -> Self {
                s as $ty
            }
        }

        impl TryFrom<$ty> for Sn {
            type Error = $ty;

            #[inline]
            fn try_from(val: $ty) -> Result<Sn, $ty> {
                match val {
                    0 => Ok(Sn::Sn0),
                    1 => Ok(Sn::Sn1),
                    2 => Ok(Sn::Sn2),
                    3 => Ok(Sn::Sn3),
                    4 => Ok(Sn::Sn4),
                    5 => Ok(Sn::Sn5),
                    6 => Ok(Sn::Sn6),
                    7 => Ok(Sn::Sn7),
                    x => Err(x),
                }
            }
        }
    };
}

sn_conversion_for!(u8);
sn_conversion_for!(u16);
sn_conversion_for!(u32);
sn_conversion_for!(u64);
sn_conversion_for!(u128);
sn_conversion_for!(usize);
sn_conversion_for!(i8);
sn_conversion_for!(i16);
sn_conversion_for!(i32);
sn_conversion_for!(i64);
sn_conversion_for!(i128);
sn_conversion_for!(isize);

/// Array of all sockets.
///
/// Useful for iterating over sockets.
///
/// # Example
///
/// Close all sockets.
///
/// ```
/// # use w5500_ll::Sn::*;
/// # use ehm1 as hal;
/// # let spi = ehm1::spi::Mock::new(&[
/// #   ehm1::spi::Transaction::transaction_start(),
/// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x01, (Sn0.block() << 3) | 0x04]),
/// #   ehm1::spi::Transaction::write_vec(vec![SocketCommand::Close.into()]),
/// #   ehm1::spi::Transaction::transaction_end(),
/// #   ehm1::spi::Transaction::transaction_start(),
/// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x01, (Sn1.block() << 3) | 0x04]),
/// #   ehm1::spi::Transaction::write_vec(vec![SocketCommand::Close.into()]),
/// #   ehm1::spi::Transaction::transaction_end(),
/// #   ehm1::spi::Transaction::transaction_start(),
/// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x01, (Sn2.block() << 3) | 0x04]),
/// #   ehm1::spi::Transaction::write_vec(vec![SocketCommand::Close.into()]),
/// #   ehm1::spi::Transaction::transaction_end(),
/// #   ehm1::spi::Transaction::transaction_start(),
/// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x01, (Sn3.block() << 3) | 0x04]),
/// #   ehm1::spi::Transaction::write_vec(vec![SocketCommand::Close.into()]),
/// #   ehm1::spi::Transaction::transaction_end(),
/// #   ehm1::spi::Transaction::transaction_start(),
/// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x01, (Sn4.block() << 3) | 0x04]),
/// #   ehm1::spi::Transaction::write_vec(vec![SocketCommand::Close.into()]),
/// #   ehm1::spi::Transaction::transaction_end(),
/// #   ehm1::spi::Transaction::transaction_start(),
/// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x01, (Sn5.block() << 3) | 0x04]),
/// #   ehm1::spi::Transaction::write_vec(vec![SocketCommand::Close.into()]),
/// #   ehm1::spi::Transaction::transaction_end(),
/// #   ehm1::spi::Transaction::transaction_start(),
/// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x01, (Sn6.block() << 3) | 0x04]),
/// #   ehm1::spi::Transaction::write_vec(vec![SocketCommand::Close.into()]),
/// #   ehm1::spi::Transaction::transaction_end(),
/// #   ehm1::spi::Transaction::transaction_start(),
/// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x01, (Sn7.block() << 3) | 0x04]),
/// #   ehm1::spi::Transaction::write_vec(vec![SocketCommand::Close.into()]),
/// #   ehm1::spi::Transaction::transaction_end(),
/// # ]);
/// use w5500_ll::{eh1::vdm::W5500, Registers, SocketCommand, SOCKETS};
///
/// let mut w5500 = W5500::new(spi);
/// for socket in SOCKETS.iter() {
///     w5500.set_sn_cr(*socket, SocketCommand::Close)?;
/// }
/// # Ok::<(), eh1::spi::ErrorKind>(())
/// ```
pub const SOCKETS: [Sn; 8] = [
    Sn::Sn0,
    Sn::Sn1,
    Sn::Sn2,
    Sn::Sn3,
    Sn::Sn4,
    Sn::Sn5,
    Sn::Sn6,
    Sn::Sn7,
];

/// TX socket buffer pointers.
///
/// Returned by [`Registers::sn_tx_ptrs`] and [`aio::Registers::sn_tx_ptrs`].
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct TxPtrs {
    /// Free size.
    pub fsr: u16,
    /// Write pointer.
    pub wr: u16,
}

/// RX socket buffer pointers.
///
/// Returned by [`Registers::sn_rx_ptrs`] and [`aio::Registers::sn_rx_ptrs`].
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct RxPtrs {
    /// Recieved size.
    pub rsr: u16,
    /// Read pointer.
    pub rd: u16,
}

/// W5500 register setters and getters.
///
/// * All register getters are simply the name of the register.
/// * All register setters are the name of the register prefixed with `set_`.
///
/// Most of the register documentation is taken from the data sheet.
pub trait Registers {
    /// Register accessor error type.
    type Error;

    /// Read from the W5500.
    ///
    /// # Arguments
    ///
    /// * `addr` - Starting address of the memory being read.
    /// * `block` - W5500 block select bits
    /// * `data` - Buffer to read data into. The number of bytes read is equal
    ///   to the length of this buffer.
    fn read(&mut self, addr: u16, block: u8, data: &mut [u8]) -> Result<(), Self::Error>;

    /// Write to the W5500.
    ///
    /// # Arguments
    ///
    /// * `addr` - Starting address of the memory being written.
    /// * `block` - W5500 block select bits
    /// * `data` - Buffer of data to write. The number of bytes written is equal
    ///   to the length of this buffer.
    fn write(&mut self, addr: u16, block: u8, data: &[u8]) -> Result<(), Self::Error>;

    /// Get the mode register.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x00, 0x00]),
    /// #   ehm1::spi::Transaction::read(0),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Mode, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let mode: Mode = w5500.mr()?;
    /// assert_eq!(mode, Mode::default());
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn mr(&mut self) -> Result<Mode, Self::Error> {
        let mut reg: [u8; 1] = [0];
        self.read(Reg::MR.addr(), COMMON_BLOCK_OFFSET, &mut reg)?;
        Ok(Mode::from(reg[0]))
    }

    /// Set the mode register.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x00, 0x04]),
    /// #   ehm1::spi::Transaction::write(w5500_ll::Mode::WOL_MASK),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Mode, Registers};
    ///
    /// const MODE: Mode = Mode::DEFAULT.enable_wol();
    /// let mut w5500 = W5500::new(spi);
    /// w5500.set_mr(MODE)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_mr(&mut self, mode: Mode) -> Result<(), Self::Error> {
        self.write(Reg::MR.addr(), COMMON_BLOCK_OFFSET, &[mode.into()])
    }

    /// Get the gateway IP address.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x01, 0x00]),
    /// #   ehm1::spi::Transaction::read_vec(vec![0, 0, 0, 0]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, net::Ipv4Addr, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let gar = w5500.gar()?;
    /// assert_eq!(gar, Ipv4Addr::UNSPECIFIED);
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn gar(&mut self) -> Result<Ipv4Addr, Self::Error> {
        let mut gar: [u8; 4] = [0; 4];
        self.read(Reg::GAR0.addr(), COMMON_BLOCK_OFFSET, &mut gar)?;
        Ok(gar.into())
    }

    /// Set the gateway IP address.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x01, 0x04]),
    /// #   ehm1::spi::Transaction::write_vec(vec![192, 168, 0, 1]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, net::Ipv4Addr, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// w5500.set_gar(&Ipv4Addr::new(192, 168, 0, 1))?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_gar(&mut self, gar: &Ipv4Addr) -> Result<(), Self::Error> {
        self.write(Reg::GAR0.addr(), COMMON_BLOCK_OFFSET, &gar.octets())
    }

    /// Get the subnet mask.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x05, 0x00]),
    /// #   ehm1::spi::Transaction::read_vec(vec![0, 0, 0, 0]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, net::Ipv4Addr, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let subr = w5500.subr()?;
    /// assert_eq!(subr, Ipv4Addr::UNSPECIFIED);
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn subr(&mut self) -> Result<Ipv4Addr, Self::Error> {
        let mut subr: [u8; 4] = [0; 4];
        self.read(Reg::SUBR0.addr(), COMMON_BLOCK_OFFSET, &mut subr)?;
        Ok(subr.into())
    }

    /// Set the subnet mask.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x05, 0x04]),
    /// #   ehm1::spi::Transaction::write_vec(vec![255, 255, 255, 0]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, net::Ipv4Addr, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// w5500.set_subr(&Ipv4Addr::new(255, 255, 255, 0))?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_subr(&mut self, subr: &Ipv4Addr) -> Result<(), Self::Error> {
        self.write(Reg::SUBR0.addr(), COMMON_BLOCK_OFFSET, &subr.octets())
    }

    /// Get the source hardware address.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x09, 0x00]),
    /// #   ehm1::spi::Transaction::read_vec(vec![0, 0, 0, 0, 0, 0]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, net::Eui48Addr, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let shar = w5500.shar()?;
    /// assert_eq!(shar, Eui48Addr::UNSPECIFIED);
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn shar(&mut self) -> Result<Eui48Addr, Self::Error> {
        let mut shar = Eui48Addr::UNSPECIFIED;
        self.read(Reg::SHAR0.addr(), COMMON_BLOCK_OFFSET, &mut shar.octets)?;
        Ok(shar)
    }

    /// Set the source hardware address.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x09, 0x04]),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x12, 0x34, 0, 0, 0, 0]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, net::Eui48Addr, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// w5500.set_shar(&Eui48Addr::new(0x12, 0x34, 0x00, 0x00, 0x00, 0x00))?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_shar(&mut self, shar: &Eui48Addr) -> Result<(), Self::Error> {
        self.write(Reg::SHAR0.addr(), COMMON_BLOCK_OFFSET, &shar.octets)
    }

    /// Get the source (client) IP address.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x0F, 0x00]),
    /// #   ehm1::spi::Transaction::read_vec(vec![0, 0, 0, 0]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, net::Ipv4Addr, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let sipr = w5500.sipr()?;
    /// assert_eq!(sipr, Ipv4Addr::UNSPECIFIED);
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn sipr(&mut self) -> Result<Ipv4Addr, Self::Error> {
        let mut sipr: [u8; 4] = [0; 4];
        self.read(Reg::SIPR0.addr(), COMMON_BLOCK_OFFSET, &mut sipr)?;
        Ok(sipr.into())
    }

    /// Set the source (client) IP address.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x0F, 0x04]),
    /// #   ehm1::spi::Transaction::write_vec(vec![192, 168, 0, 150]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, net::Ipv4Addr, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// w5500.set_sipr(&Ipv4Addr::new(192, 168, 0, 150))?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_sipr(&mut self, sipr: &Ipv4Addr) -> Result<(), Self::Error> {
        self.write(Reg::SIPR0.addr(), COMMON_BLOCK_OFFSET, &sipr.octets())
    }

    /// Get the interrupt low level time.
    ///
    /// INTLEVEL configures the assert wait time (I<sub>AWT</sub>).
    ///
    /// When the  next interrupt occurs, the interrupt in (INTn) will assert
    /// to low after INTLEVEL time.
    ///
    /// The equation is:
    ///
    /// I<sub>AWT</sub> = (INTLEVEL + 1) * PLL<sub>CLK</sub> * 4
    ///
    /// When INTLEVEL > 0.
    ///
    /// You might want to take a look at the data sheet, there is a handy
    /// timing diagram there.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x13, 0x00]),
    /// #   ehm1::spi::Transaction::read_vec(vec![0, 0]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let intlevel: u16 = w5500.intlevel()?;
    /// assert_eq!(intlevel, 0x00);
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn intlevel(&mut self) -> Result<u16, Self::Error> {
        let mut buf: [u8; 2] = [0; 2];
        self.read(Reg::INTLEVEL0.addr(), COMMON_BLOCK_OFFSET, &mut buf)?;
        Ok(u16::from_be_bytes(buf))
    }

    /// Set the interrupt low level time.
    ///
    /// See [`Registers::intlevel`] for more information.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x13, 0x04]),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x12, 0x34]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// w5500.set_intlevel(0x1234)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_intlevel(&mut self, intlevel: u16) -> Result<(), Self::Error> {
        self.write(
            Reg::INTLEVEL0.addr(),
            COMMON_BLOCK_OFFSET,
            &intlevel.to_be_bytes(),
        )
    }

    /// Get the interrupt status.
    ///
    /// `1` indicates the interrupt is raised.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x15, 0x00]),
    /// #   ehm1::spi::Transaction::read(0),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Interrupt, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let ir: Interrupt = w5500.ir()?;
    /// assert_eq!(ir, Interrupt::default());
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn ir(&mut self) -> Result<Interrupt, Self::Error> {
        let mut reg: [u8; 1] = [0];
        self.read(Reg::IR.addr(), COMMON_BLOCK_OFFSET, &mut reg)?;
        Ok(Interrupt::from(reg[0]))
    }

    /// Set the interrupt status.
    ///
    /// Setting an interrupt bit to `1` will clear the interrupt.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x15, 0x00]),
    /// #   ehm1::spi::Transaction::read(0),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x15, 0x04]),
    /// #   ehm1::spi::Transaction::write(0),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Interrupt, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let ir: Interrupt = w5500.ir()?;
    /// w5500.set_ir(ir)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_ir(&mut self, interrupt: Interrupt) -> Result<(), Self::Error> {
        self.write(Reg::IR.addr(), COMMON_BLOCK_OFFSET, &[interrupt.into()])
    }

    /// Get the interrupt mask.
    ///
    /// `0` indicates the interrupt is masked.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x16, 0x00]),
    /// #   ehm1::spi::Transaction::read(0),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Interrupt, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let imr: Interrupt = w5500.imr()?;
    /// assert_eq!(imr, Interrupt::default());
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn imr(&mut self) -> Result<Interrupt, Self::Error> {
        let mut reg: [u8; 1] = [0];
        self.read(Reg::IMR.addr(), COMMON_BLOCK_OFFSET, &mut reg)?;
        Ok(Interrupt::from(reg[0]))
    }

    /// Set the interrupt mask.
    ///
    /// Setting an interrupt bit to `1` will mask the interrupt.
    /// When a bit of IMR is `1` and the corresponding interrupt is `1` an
    /// interrupt will be issued.
    /// If a bit of IMR is `0`, and interrupt will not be issued even if the
    /// corresponding IR bit is `1`.
    ///
    /// # Example
    ///
    /// ```
    /// use w5500_ll::{eh1::vdm::W5500, Interrupt, Registers};
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x16, 0x04]),
    /// #   ehm1::spi::Transaction::write_vec(vec![Interrupt::MP_MASK]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    ///
    /// // enable the magic packet interrupt
    /// const IMR: Interrupt = Interrupt::DEFAULT.set_mp();
    ///
    /// let mut w5500 = W5500::new(spi);
    /// w5500.set_imr(IMR)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_imr(&mut self, mask: Interrupt) -> Result<(), Self::Error> {
        self.write(Reg::IMR.addr(), COMMON_BLOCK_OFFSET, &[mask.into()])
    }

    /// Get the socket interrupt status.
    ///
    /// SIMR indicates the interrupt status of all sockets.
    /// Each bit of SIR will be `1` until [`sn_ir`] is cleared.
    /// If [`sn_ir`] is not equal to `0x00` the n<sub>th</sub>
    /// bit of `sir` is `1` and the INTn pin is asserted until SIR is `0x00`.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x17, 0x00]),
    /// #   ehm1::spi::Transaction::read(0),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers, SOCKETS};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let sir = w5500.sir()?;
    /// // clear all socket interrupts
    /// for socket in SOCKETS.iter() {
    ///     if 1 << (*socket as u8) & sir != 0 {
    ///         let sn_ir = w5500.sn_ir(*socket)?;
    ///         w5500.set_sn_ir(*socket, sn_ir)?;
    ///     }
    /// }
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    ///
    /// [`sn_ir`]: Registers::sn_ir
    fn sir(&mut self) -> Result<u8, Self::Error> {
        let mut reg: [u8; 1] = [0];
        self.read(Reg::SIR.addr(), COMMON_BLOCK_OFFSET, &mut reg)?;
        Ok(reg[0])
    }

    /// Get the socket interrupt mask.
    ///
    /// Each bit of SIMR corresponds to each bit of [`Registers::sir`].
    /// When a bit of SIMR is `1` and the corresponding bit of SIR is `1`
    /// and interrupt will be issued.
    /// If a bit of SIMR is `0` an interrupt will be not issued even if the
    /// corresponding bit of SIR is `1`.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x18, 0x00]),
    /// #   ehm1::spi::Transaction::read(0),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let simr: u8 = w5500.simr()?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn simr(&mut self) -> Result<u8, Self::Error> {
        let mut reg: [u8; 1] = [0];
        self.read(Reg::SIMR.addr(), COMMON_BLOCK_OFFSET, &mut reg)?;
        Ok(reg[0])
    }

    /// Set the socket interrupt mask.
    ///
    /// See [`Registers::simr`] for more information.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x18, 0x04]),
    /// #   ehm1::spi::Transaction::write(0xFF),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// // enable all socket interrupts
    /// w5500.set_simr(0xFF)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_simr(&mut self, simr: u8) -> Result<(), Self::Error> {
        self.write(Reg::SIMR.addr(), COMMON_BLOCK_OFFSET, &[simr])
    }

    /// Get the retry time.
    ///
    /// RTR configures the re-transmission timeout period.
    /// The unit of timeout period is 100 us and the default of RTR is `0x07D0`
    /// or `2000`.
    /// And so the default timeout period is 200 ms (100 us X 2000).
    /// During the time configured by RTR, the W5500 waits for the peer response
    /// to the packet that is transmitted by Sn_CR (CONNECT, DISCON, CLOSE,
    /// SEND, SEND_MAC, SEND_KEEP command).
    /// If the peer does not respond within the RTR time, the W5500 re-transmits
    /// the packet or issues a timeout.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x19, 0x00]),
    /// #   ehm1::spi::Transaction::read_vec(vec![0x07, 0xD0]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let rtr: u16 = w5500.rtr()?;
    /// assert_eq!(rtr, 0x07D0);
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn rtr(&mut self) -> Result<u16, Self::Error> {
        let mut buf: [u8; 2] = [0; 2];
        self.read(Reg::RTR0.addr(), COMMON_BLOCK_OFFSET, &mut buf)?;
        Ok(u16::from_be_bytes(buf))
    }

    /// Set the retry time.
    ///
    /// See [`Registers::rtr`] for more information.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x19, 0x04]),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x12, 0x34]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// w5500.set_rtr(0x1234)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_rtr(&mut self, rtr: u16) -> Result<(), Self::Error> {
        self.write(Reg::RTR0.addr(), COMMON_BLOCK_OFFSET, &rtr.to_be_bytes())
    }

    /// Get the retry count.
    ///
    /// RCR configured the number of re-transmission attempts.
    /// When the number of re-transmission equals RCR + 1 the socket timeout
    /// interrupt is raised.
    ///
    /// There is a LOT more information in the data sheet,
    /// including some equations that would be very annoying to input.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x1B, 0x00]),
    /// #   ehm1::spi::Transaction::read(0x08),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let rcr: u8 = w5500.rcr()?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn rcr(&mut self) -> Result<u8, Self::Error> {
        let mut reg: [u8; 1] = [0];
        self.read(Reg::RCR.addr(), COMMON_BLOCK_OFFSET, &mut reg)?;
        Ok(reg[0])
    }

    /// Set the retry count.
    ///
    /// See [`Registers::rcr`] for more information.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x1B, 0x04]),
    /// #   ehm1::spi::Transaction::write(0x0A),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// w5500.set_rcr(0x0A)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_rcr(&mut self, rcr: u8) -> Result<(), Self::Error> {
        self.write(Reg::RCR.addr(), COMMON_BLOCK_OFFSET, &[rcr])
    }

    /// Get the PPP link control protocol request timer.
    ///
    /// PTIMER configures the time for sending LCP echo request.
    ///
    /// The unit of time is 25 milliseconds, for a register value of 200 the
    /// timer is 5 seconds.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x1C, 0x00]),
    /// #   ehm1::spi::Transaction::read(0x08),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let ptimer: u8 = w5500.ptimer()?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn ptimer(&mut self) -> Result<u8, Self::Error> {
        let mut reg: [u8; 1] = [0];
        self.read(Reg::PTIMER.addr(), COMMON_BLOCK_OFFSET, &mut reg)?;
        Ok(reg[0])
    }

    /// Set the PPP link control protocol request timer.
    ///
    /// See [`Registers::ptimer`] for more information.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x1C, 0x04]),
    /// #   ehm1::spi::Transaction::write(0xC8),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// w5500.set_ptimer(200)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_ptimer(&mut self, ptimer: u8) -> Result<(), Self::Error> {
        self.write(Reg::PTIMER.addr(), COMMON_BLOCK_OFFSET, &[ptimer])
    }

    /// Get the PPP link control protocol magic number.
    ///
    /// PMAGIC configures the 4 byte magic number used in the LCP echo request.
    /// For a register value of `0x01` the magic number is `0x01010101`.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x1D, 0x00]),
    /// #   ehm1::spi::Transaction::read(0x08),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let pmagic: u8 = w5500.pmagic()?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn pmagic(&mut self) -> Result<u8, Self::Error> {
        let mut reg: [u8; 1] = [0];
        self.read(Reg::PMAGIC.addr(), COMMON_BLOCK_OFFSET, &mut reg)?;
        Ok(reg[0])
    }

    /// Set the PPP link control protocol magic number.
    ///
    /// See [`Registers::pmagic`] for more information.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x1D, 0x04]),
    /// #   ehm1::spi::Transaction::write(0x01),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// w5500.set_pmagic(0x01)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_pmagic(&mut self, pmagic: u8) -> Result<(), Self::Error> {
        self.write(Reg::PMAGIC.addr(), COMMON_BLOCK_OFFSET, &[pmagic])
    }

    /// Get the destination hardware address in PPPoE mode.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x1E, 0x00]),
    /// #   ehm1::spi::Transaction::read_vec(vec![0, 0, 0, 0, 0, 0]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, net::Eui48Addr, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let phar = w5500.phar()?;
    /// assert_eq!(phar, Eui48Addr::UNSPECIFIED);
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn phar(&mut self) -> Result<Eui48Addr, Self::Error> {
        let mut phar = Eui48Addr::UNSPECIFIED;
        self.read(Reg::PHAR0.addr(), COMMON_BLOCK_OFFSET, &mut phar.octets)?;
        Ok(phar)
    }

    /// Set the destination hardware address in PPPoE mode.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x1E, 0x04]),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x12, 0x34, 0, 0, 0, 0]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, net::Eui48Addr, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// w5500.set_phar(&Eui48Addr::new(0x12, 0x34, 0x00, 0x00, 0x00, 0x00))?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_phar(&mut self, phar: &Eui48Addr) -> Result<(), Self::Error> {
        self.write(Reg::PHAR0.addr(), COMMON_BLOCK_OFFSET, &phar.octets)
    }

    /// Get the session ID in PPPoE mode.
    ///
    /// PSID should be written to the PPPoE server session ID acquired in the
    /// PPPoE connection process.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x24, 0x00]),
    /// #   ehm1::spi::Transaction::read_vec(vec![0, 0]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let psid: u16 = w5500.psid()?;
    /// assert_eq!(psid, 0x0000);
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn psid(&mut self) -> Result<u16, Self::Error> {
        let mut buf: [u8; 2] = [0; 2];
        self.read(Reg::PSID0.addr(), COMMON_BLOCK_OFFSET, &mut buf)?;
        Ok(u16::from_be_bytes(buf))
    }

    /// Set the session ID in PPPoE mode.
    ///
    /// See [`Registers::psid`] for more information.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x24, 0x04]),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x12, 0x34]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// w5500.set_psid(0x1234)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_psid(&mut self, psid: u16) -> Result<(), Self::Error> {
        self.write(Reg::PSID0.addr(), COMMON_BLOCK_OFFSET, &psid.to_be_bytes())
    }

    /// Get the maximum receive unit in PPPoE mode.
    ///
    /// PMRU configures the maximum receive unit of PPPoE.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x26, 0x00]),
    /// #   ehm1::spi::Transaction::read_vec(vec![0, 0]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let pmru: u16 = w5500.pmru()?;
    /// assert_eq!(pmru, 0x0000);
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn pmru(&mut self) -> Result<u16, Self::Error> {
        let mut buf: [u8; 2] = [0; 2];
        self.read(Reg::PMRU0.addr(), COMMON_BLOCK_OFFSET, &mut buf)?;
        Ok(u16::from_be_bytes(buf))
    }

    /// Set the maximum receive unit in PPPoE mode.
    ///
    /// See [`Registers::pmru`] for more information.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x26, 0x04]),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x12, 0x34]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// w5500.set_pmru(0x1234)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_pmru(&mut self, pmru: u16) -> Result<(), Self::Error> {
        self.write(Reg::PMRU0.addr(), COMMON_BLOCK_OFFSET, &pmru.to_be_bytes())
    }

    /// Get the unreachable IP address.
    ///
    /// This awkward wording is taken directly from the data-sheet:
    ///
    /// W5500 receives an ICMP packet (destination port unreachable)
    /// when data is sent to a port number which socket is not open and
    /// the UNREACH bit of [`Registers::ir`] becomes `1` and UIPR and UPORTR
    /// indicates the destination IP address and port number respectively.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x28, 0x00]),
    /// #   ehm1::spi::Transaction::read_vec(vec![0, 0, 0, 0]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, net::Ipv4Addr, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let uipr = w5500.uipr()?;
    /// assert_eq!(uipr, Ipv4Addr::UNSPECIFIED);
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn uipr(&mut self) -> Result<Ipv4Addr, Self::Error> {
        let mut uipr: [u8; 4] = [0; 4];
        self.read(Reg::UIPR0.addr(), COMMON_BLOCK_OFFSET, &mut uipr)?;
        Ok(uipr.into())
    }

    /// Get the unreachable port.
    ///
    /// See [`Registers::uipr`] for more information.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x2C, 0x00]),
    /// #   ehm1::spi::Transaction::read_vec(vec![0, 0]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let uportr = w5500.uportr()?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn uportr(&mut self) -> Result<u16, Self::Error> {
        let mut buf: [u8; 2] = [0; 2];
        self.read(Reg::UPORTR0.addr(), COMMON_BLOCK_OFFSET, &mut buf)?;
        Ok(u16::from_be_bytes(buf))
    }

    /// Get the PHY configuration.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x2E, 0x00]),
    /// #   ehm1::spi::Transaction::read(0b10111000),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, PhyCfg, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let phy_cfg: PhyCfg = w5500.phycfgr()?;
    /// assert_eq!(phy_cfg, PhyCfg::default());
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn phycfgr(&mut self) -> Result<PhyCfg, Self::Error> {
        let mut reg: [u8; 1] = [0];
        self.read(Reg::PHYCFGR.addr(), COMMON_BLOCK_OFFSET, &mut reg)?;
        Ok(PhyCfg::from(reg[0]))
    }

    /// Set the PHY configuration.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x2E, 0x04]),
    /// #   ehm1::spi::Transaction::write(0b11111000),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, OperationMode, PhyCfg, Registers};
    ///
    /// const PHY_CFG: PhyCfg = PhyCfg::DEFAULT.set_opmdc(OperationMode::Auto);
    /// let mut w5500 = W5500::new(spi);
    /// w5500.set_phycfgr(PHY_CFG)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_phycfgr(&mut self, phycfg: PhyCfg) -> Result<(), Self::Error> {
        self.write(Reg::PHYCFGR.addr(), COMMON_BLOCK_OFFSET, &[phycfg.into()])
    }

    /// Get the version.
    ///
    /// The value returned is always `0x04`.
    ///
    /// This register is extremely useful as a sanity check to ensure SPI
    /// communications are working with the W5500.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x39, 0x00]),
    /// #   ehm1::spi::Transaction::read(0x04),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let version = w5500.version()?;
    /// assert_eq!(version, 0x04);
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn version(&mut self) -> Result<u8, Self::Error> {
        let mut reg: [u8; 1] = [0];
        self.read(Reg::VERSIONR.addr(), COMMON_BLOCK_OFFSET, &mut reg)?;
        Ok(reg[0])
    }

    /// Get the socket mode.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x00, 0x08]),
    /// #   ehm1::spi::Transaction::read(0),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn, SocketMode};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let socket_mode = w5500.sn_mr(Sn::Sn0)?;
    /// assert_eq!(socket_mode, SocketMode::default());
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn sn_mr(&mut self, sn: Sn) -> Result<SocketMode, Self::Error> {
        let mut reg: [u8; 1] = [0];
        self.read(SnReg::MR.addr(), sn.block(), &mut reg)?;
        Ok(SocketMode::from(reg[0]))
    }

    /// Set the socket mode.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x00, 0x08 | 0x04]),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x01]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Protocol, Registers, Sn, SocketMode};
    ///
    /// const SOCKET_MODE: SocketMode = SocketMode::DEFAULT.set_protocol(Protocol::Tcp);
    /// let mut w5500 = W5500::new(spi);
    /// w5500.set_sn_mr(Sn::Sn0, SOCKET_MODE)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_sn_mr(&mut self, sn: Sn, mode: SocketMode) -> Result<(), Self::Error> {
        self.write(SnReg::MR.addr(), sn.block(), &[mode.into()])
    }

    /// Get the socket command.
    ///
    /// The only use for reading this register is to check if a socket command
    /// has been accepted.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x01, 0x08 | 0x04]),
    /// #   ehm1::spi::Transaction::write(0x01),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x01, 0x08]),
    /// #   ehm1::spi::Transaction::read(1),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x01, 0x08]),
    /// #   ehm1::spi::Transaction::read(0),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn, SocketCommand};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// w5500.set_sn_cr(Sn::Sn0, SocketCommand::Open)?;
    /// while w5500.sn_cr(Sn::Sn0)? != SocketCommand::Accepted.into() {}
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn sn_cr(&mut self, sn: Sn) -> Result<u8, Self::Error> {
        let mut reg: [u8; 1] = [0];
        self.read(SnReg::CR.addr(), sn.block(), &mut reg)?;
        Ok(reg[0])
    }

    /// Set the socket command.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x01, 0x08 | 0x04]),
    /// #   ehm1::spi::Transaction::write(0x01),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn, SocketCommand};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// w5500.set_sn_cr(Sn::Sn0, SocketCommand::Open)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_sn_cr(&mut self, sn: Sn, cmd: SocketCommand) -> Result<(), Self::Error> {
        self.write(SnReg::CR.addr(), sn.block(), &[cmd.into()])
    }

    /// Get the socket interrupt status.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x02, 0x08]),
    /// #   ehm1::spi::Transaction::read(0),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let socket_interrupts = w5500.sn_ir(Sn::Sn0)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn sn_ir(&mut self, sn: Sn) -> Result<SocketInterrupt, Self::Error> {
        let mut reg: [u8; 1] = [0];
        self.read(SnReg::IR.addr(), sn.block(), &mut reg)?;
        Ok(SocketInterrupt::from(reg[0]))
    }

    /// Set the socket interrupt status.
    ///
    /// This is a write 1 to clear register.
    ///
    /// # Examples
    ///
    /// Clearing all raised interrupts.
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x02, 0x08]),
    /// #   ehm1::spi::Transaction::read(0),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x02, 0x08 | 0x04]),
    /// #   ehm1::spi::Transaction::write(0),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn, SocketInterrupt};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let socket_interrupts: SocketInterrupt = w5500.sn_ir(Sn::Sn0)?;
    /// w5500.set_sn_ir(Sn::Sn0, socket_interrupts)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    ///
    /// Clearing only the SENDOK interrupt.
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x02, 0x08 | 0x04]),
    /// #   ehm1::spi::Transaction::write(SocketInterrupt::SENDOK_MASK),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn, SocketInterrupt};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// w5500.set_sn_ir(Sn::Sn0, SocketInterrupt::SENDOK_MASK)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_sn_ir<T: Into<u8>>(&mut self, sn: Sn, sn_ir: T) -> Result<(), Self::Error> {
        self.write(SnReg::IR.addr(), sn.block(), &[sn_ir.into()])
    }

    /// Get the socket status.
    ///
    /// **Note:** This method returns a nested [`core::result::Result`].
    ///
    /// The outermost `Result` is for handling bus errors, similar to most of
    /// the other methods in this trait.
    ///
    /// The innermost `Result<SocketStatus, u8>` is the result of a `u8` to
    /// [`SocketStatus`] conversion because not every value of `u8` corresponds
    /// to a valid [`SocketStatus`].
    /// * `u8` values that have a corresponding [`SocketStatus`] will be
    ///   converted and returned in the [`Ok`] variant of the inner `Result`.
    /// * `u8` values that do not corresponding [`SocketStatus`] will have the
    ///   raw `u8` byte returned in the [`Err`] variant of the inner `Result`.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x03, 0x08]),
    /// #   ehm1::spi::Transaction::read(0),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn, SocketStatus};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let sn_sr = w5500.sn_sr(Sn::Sn0)?;
    /// assert_eq!(sn_sr, Ok(SocketStatus::Closed));
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    ///
    /// [`Ok`]: https://doc.rust-lang.org/core/result/enum.Result.html#variant.Ok
    /// [`Err`]: https://doc.rust-lang.org/core/result/enum.Result.html#variant.Err
    fn sn_sr(&mut self, sn: Sn) -> Result<Result<SocketStatus, u8>, Self::Error> {
        let mut reg: [u8; 1] = [0];
        self.read(SnReg::SR.addr(), sn.block(), &mut reg)?;
        Ok(SocketStatus::try_from(reg[0]))
    }

    /// Get the socket source port.
    ///
    /// This is only valid in TCP/UDP mode.
    /// This should be set before sending the OPEN command.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x04, 0x08]),
    /// #   ehm1::spi::Transaction::read_vec(vec![0, 0]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn, SocketMode};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let socket_port: u16 = w5500.sn_port(Sn::Sn0)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn sn_port(&mut self, sn: Sn) -> Result<u16, Self::Error> {
        let mut reg: [u8; 2] = [0; 2];
        self.read(SnReg::PORT0.addr(), sn.block(), &mut reg)?;
        Ok(u16::from_be_bytes(reg))
    }

    /// Set the socket source port.
    ///
    /// See [`Registers::sn_port`] for more information.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x04, 0x08 | 0x04]),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 68]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// w5500.set_sn_port(Sn::Sn0, 68)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_sn_port(&mut self, sn: Sn, port: u16) -> Result<(), Self::Error> {
        self.write(SnReg::PORT0.addr(), sn.block(), &u16::to_be_bytes(port))
    }

    /// Get the socket destination hardware address.
    ///
    /// Sn_DHAR configures the destination hardware address of Socket n when
    /// using SEND_MAC command in UDP mode or it indicates that it is acquired
    /// in ARP-process by CONNECT/SEND command.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x06, 0x08]),
    /// #   ehm1::spi::Transaction::read_vec(vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let dhar = w5500.sn_dhar(Sn::Sn0)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn sn_dhar(&mut self, sn: Sn) -> Result<Eui48Addr, Self::Error> {
        let mut dhar: Eui48Addr = Eui48Addr::UNSPECIFIED;
        self.read(SnReg::DHAR0.addr(), sn.block(), &mut dhar.octets)?;
        Ok(dhar)
    }

    /// Set the socket destination hardware address.
    ///
    /// See [`Registers::sn_dhar`] for more information.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x06, 0x08 | 0x04]),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x12, 0x34, 0x00, 0x00, 0x00, 0x00]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, net::Eui48Addr, Registers, Sn};
    ///
    /// let dhar = Eui48Addr::new(0x12, 0x34, 0x00, 0x00, 0x00, 0x00);
    /// let mut w5500 = W5500::new(spi);
    /// w5500.set_sn_dhar(Sn::Sn0, &dhar)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_sn_dhar(&mut self, sn: Sn, dhar: &Eui48Addr) -> Result<(), Self::Error> {
        self.write(SnReg::DHAR0.addr(), sn.block(), &dhar.octets)
    }

    /// Get the socket destination IP address.
    ///
    /// This register configures or indicates the destination IP address.
    /// It it valid when the socket is in TCP/UDP mode.
    ///
    /// In TCP client mode it configures the TCP server address before the
    /// [`SocketCommand::Connect`] command.
    ///
    /// In TCP server mode it indicates the IP address of the TCP client after
    /// successfully establishing a connection.
    ///
    /// In UDP mode it configures an IP address of the peer to receive the UDP
    /// packet send by the [`SocketCommand::Send`] or [`SocketCommand::SendMac`]
    /// command.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x0C, 0x08]),
    /// #   ehm1::spi::Transaction::read_vec(vec![0, 0, 0, 0]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, net::Ipv4Addr, Registers, Sn};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let dipr = w5500.sn_dipr(Sn::Sn0)?;
    /// assert_eq!(dipr, Ipv4Addr::UNSPECIFIED);
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn sn_dipr(&mut self, sn: Sn) -> Result<Ipv4Addr, Self::Error> {
        let mut dipr: [u8; 4] = [0; 4];
        self.read(SnReg::DIPR0.addr(), sn.block(), &mut dipr)?;
        Ok(dipr.into())
    }

    /// Set the socket destination IP address.
    ///
    /// See [`Registers::sn_dipr`] for more information.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x0C, 0x08 | 0x04]),
    /// #   ehm1::spi::Transaction::write_vec(vec![192, 168, 0, 11]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, net::Ipv4Addr, Registers, Sn};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// w5500.set_sn_dipr(Sn::Sn0, &Ipv4Addr::new(192, 168, 0, 11))?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_sn_dipr(&mut self, sn: Sn, dipr: &Ipv4Addr) -> Result<(), Self::Error> {
        self.write(SnReg::DIPR0.addr(), sn.block(), &dipr.octets())
    }

    /// Get the socket destination port.
    ///
    /// This register configures or indicates the destination port number of
    /// the socket.
    /// It is valid when the socket is used in TCP/UDP mode.
    ///
    /// In TCP client mode, it configures the listen port number of the TCP
    /// server before the [`SocketCommand::Send`] command.
    ///
    /// In TCP server mode, it indicates the port number of the TCP client
    /// after successfully establishing connection.
    ///
    /// In UDP mode, it configures the port number of the peer to be transmitted
    /// in the UDP packet by the [`SocketCommand::Send`] or
    /// [`SocketCommand::SendMac`] command.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x10, 0x08]),
    /// #   ehm1::spi::Transaction::read_vec(vec![0, 0]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn, SocketMode};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let socket_destination_port: u16 = w5500.sn_dport(Sn::Sn0)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn sn_dport(&mut self, sn: Sn) -> Result<u16, Self::Error> {
        let mut reg: [u8; 2] = [0; 2];
        self.read(SnReg::DPORT0.addr(), sn.block(), &mut reg)?;
        Ok(u16::from_be_bytes(reg))
    }

    /// Set the socket destination port.
    ///
    /// See [`Registers::sn_dport`] for more information.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x10, 0x08 | 0x04]),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 67]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// w5500.set_sn_dport(Sn::Sn0, 67)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_sn_dport(&mut self, sn: Sn, port: u16) -> Result<(), Self::Error> {
        self.write(SnReg::DPORT0.addr(), sn.block(), &u16::to_be_bytes(port))
    }

    /// Get the socket destination IPv4 and port.
    ///
    /// This is equivalent to [`Registers::sn_dipr`] and [`Registers::sn_dport`]
    /// in a single read transaction.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x0C, 0x08]),
    /// #   ehm1::spi::Transaction::read_vec(vec![0, 0, 0, 0, 0, 0]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{
    ///     eh1::vdm::W5500,
    ///     net::{Ipv4Addr, SocketAddrV4},
    ///     Registers, Sn,
    /// };
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let addr = w5500.sn_dest(Sn::Sn0)?;
    /// assert_eq!(addr, SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 0));
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn sn_dest(&mut self, sn: Sn) -> Result<SocketAddrV4, Self::Error> {
        let mut buf: [u8; 6] = [0; 6];
        self.read(SnReg::DIPR0.addr(), sn.block(), &mut buf)?;
        Ok(SocketAddrV4::new(
            Ipv4Addr::new(buf[0], buf[1], buf[2], buf[3]),
            u16::from_be_bytes([buf[4], buf[5]]),
        ))
    }

    /// Set the socket destination IPv4 and port.
    ///
    /// This is equivalent to [`Registers::set_sn_dipr`] and
    /// [`Registers::set_sn_dport`] in a single writer transaction.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x0C, 0x08 | 0x04]),
    /// #   ehm1::spi::Transaction::write_vec(vec![192, 168, 0, 11, 0, 67]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{
    ///     eh1::vdm::W5500,
    ///     net::{Ipv4Addr, SocketAddrV4},
    ///     Registers, Sn,
    /// };
    ///
    /// let addr: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new(192, 168, 0, 11), 67);
    /// let mut w5500 = W5500::new(spi);
    /// w5500.set_sn_dest(Sn::Sn0, &addr)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_sn_dest(&mut self, sn: Sn, addr: &SocketAddrV4) -> Result<(), Self::Error> {
        let buf: [u8; 6] = [
            addr.ip().octets()[0],
            addr.ip().octets()[1],
            addr.ip().octets()[2],
            addr.ip().octets()[3],
            (addr.port() >> 8) as u8,
            addr.port() as u8,
        ];
        self.write(SnReg::DIPR0.addr(), sn.block(), &buf)
    }

    /// Get the socket maximum segment size.
    ///
    /// This register configures or indicates the MTU (Maximum Transfer Unit)
    /// of the socket.
    ///
    /// The default MTU is valid when the socket is used in TCP / UDP mode.
    /// However, when used in PPPoE mode it is determined within the PPPoE MTU.
    ///
    /// | Mode   | Normal Default | Normal Range | PPPoE Default | PPPoE Range |
    /// |--------|----------------|--------------|---------------|-------------|
    /// | TCP    | 1460           | 1 - 1460     | 1452          | 1 - 1452    |
    /// | UDP    | 1472           | 1 - 1472     | 1464          | 1 - 1464    |
    /// | MACRAW | 1514           | 1514         | 1514          | 1514        |
    ///
    /// When socket n is used in MACRAW mode, the default MTU is applied
    /// because the MTU is not processed internally.
    /// Therefore, when transmitting the data bigger than default MTU, the host
    /// should manually divide the data into the unit of default MTU.
    /// When socket n is used in TCP/UDP mode, and transmitting data bigger than
    /// the MTU, the data is automatically divided into the unit of MTU.
    ///
    /// In UDP mode, the configured MTU is used.
    /// When transmitting data to a peer with the different MTU size,
    /// the ICMP (Fragment MTU) packet might be received.
    /// In this case, IR(FMTU) becomes `1` and the peer information such as the
    /// MTU size and IP address is indicated from FMTUR and UIPR respectively.
    /// If IR\[MTU\] = `1`, the user cannot transmit data to the peer.
    ///
    /// To resume the communication with peer, do as followed.
    /// 1. Close socket n with the [`SocketCommand::Close`] command.
    /// 2. Set Sn_MSS to the indicated MTU from FMTUR
    /// 3. Open socket n with the [`SocketCommand::Open`] command.
    /// 4. Resume the communication with the peer.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x12, 0x08]),
    /// #   ehm1::spi::Transaction::read_vec(vec![0x00, 0x00]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let sn__mssr: u16 = w5500.sn_mssr(Sn::Sn0)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn sn_mssr(&mut self, sn: Sn) -> Result<u16, Self::Error> {
        let mut reg: [u8; 2] = [0; 2];
        self.read(SnReg::MSSR0.addr(), sn.block(), &mut reg)?;
        Ok(u16::from_be_bytes(reg))
    }

    /// Set the socket maximum segment size.
    ///
    /// See [`Registers::sn_mssr`] for lots more information.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x12, 0x08 | 0x04]),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x05, 0xB4]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// w5500.set_sn_mssr(Sn::Sn0, 1460)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_sn_mssr(&mut self, sn: Sn, mssr: u16) -> Result<(), Self::Error> {
        self.write(SnReg::MSSR0.addr(), sn.block(), &u16::to_be_bytes(mssr))
    }

    /// Get the IP type of service.
    ///
    /// This register configures the TOS (Type of service field in IP header)
    /// for socket n.
    /// Configure this field before sending the [`SocketCommand::Open`] command.
    ///
    /// For more details see [iana.org/assignments/ip-parameters].
    ///
    /// [iana.org/assignments/ip-parameters]: https://www.iana.org/assignments/ip-parameters
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x15, 0x08]),
    /// #   ehm1::spi::Transaction::read(0),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let tos: u8 = w5500.sn_tos(Sn::Sn0)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn sn_tos(&mut self, sn: Sn) -> Result<u8, Self::Error> {
        let mut reg: [u8; 1] = [0];
        self.read(SnReg::TOS.addr(), sn.block(), &mut reg)?;
        Ok(reg[0])
    }

    /// Set the IP type of service.
    ///
    /// For more information see [`Registers::sn_tos`].
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x15, 0x08 | 0x04]),
    /// #   ehm1::spi::Transaction::write(0x01),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// w5500.set_sn_tos(Sn::Sn0, 1)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_sn_tos(&mut self, sn: Sn, tos: u8) -> Result<(), Self::Error> {
        self.write(SnReg::TOS.addr(), sn.block(), &[tos])
    }

    /// Get the time to live.
    ///
    /// This register configures the TTL (Time to Live field in the IP header)
    /// for socket n.
    ///
    /// For more details see <https://www.iana.org/assignments/ip-parameters>.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x16, 0x08]),
    /// #   ehm1::spi::Transaction::read(0x80),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let ttl: u8 = w5500.sn_ttl(Sn::Sn0)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn sn_ttl(&mut self, sn: Sn) -> Result<u8, Self::Error> {
        let mut reg: [u8; 1] = [0];
        self.read(SnReg::TTL.addr(), sn.block(), &mut reg)?;
        Ok(reg[0])
    }

    /// Set the time to live.
    ///
    /// For more information see [`Registers::sn_ttl`].
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x16, 0x08 | 0x04]),
    /// #   ehm1::spi::Transaction::write(0x80),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// w5500.set_sn_ttl(Sn::Sn0, 0x80)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_sn_ttl(&mut self, sn: Sn, ttl: u8) -> Result<(), Self::Error> {
        self.write(SnReg::TTL.addr(), sn.block(), &[ttl])
    }

    /// Get the socket RX buffer size.
    ///
    /// The buffer size can be configured to any of the sizes in [`BufferSize`].
    ///
    /// The sum of all the socket RX buffers cannot exceed 16 KiB.
    ///
    /// **Note:** This method returns a nested [`core::result::Result`].
    ///
    /// The outermost `Result` is for handling bus errors, similar to most of
    /// the other methods in this trait.
    ///
    /// The innermost `Result<BufferSize, u8>` is the result of a `u8` to
    /// [`BufferSize`] conversion because not every value of `u8` corresponds
    /// to a valid [`BufferSize`].
    /// * `u8` values that have a corresponding [`BufferSize`] will be
    ///   converted and returned in the [`Ok`] variant of the inner `Result`.
    /// * `u8` values that do not corresponding [`BufferSize`] will have the
    ///   raw `u8` byte returned in the [`Err`] variant of the inner `Result`.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x1E, 0x08]),
    /// #   ehm1::spi::Transaction::read(0x02),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, BufferSize, Registers, Sn};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let sn_rxbuf_size = w5500.sn_rxbuf_size(Sn::Sn0)?;
    /// assert_eq!(sn_rxbuf_size, Ok(BufferSize::KB2));
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    ///
    /// [`Ok`]: https://doc.rust-lang.org/core/result/enum.Result.html#variant.Ok
    /// [`Err`]: https://doc.rust-lang.org/core/result/enum.Result.html#variant.Err
    fn sn_rxbuf_size(&mut self, sn: Sn) -> Result<Result<BufferSize, u8>, Self::Error> {
        let mut reg: [u8; 1] = [0];
        self.read(SnReg::RXBUF_SIZE.addr(), sn.block(), &mut reg)?;
        Ok(BufferSize::try_from(reg[0]))
    }

    /// Set the socket RX buffer size.
    ///
    /// See [`Registers::sn_rxbuf_size`] for more information.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x1E, 0x08 | 0x04]),
    /// #   ehm1::spi::Transaction::write(1),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, BufferSize, Registers, Sn};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// w5500.set_sn_rxbuf_size(Sn::Sn0, BufferSize::KB1)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_sn_rxbuf_size(&mut self, sn: Sn, size: BufferSize) -> Result<(), Self::Error> {
        self.write(SnReg::RXBUF_SIZE.addr(), sn.block(), &[size.into()])
    }

    /// Get the socket TX buffer size.
    ///
    /// The buffer size can be configured to any of the sizes in [`BufferSize`].
    ///
    /// The sum of all the socket TX buffers cannot exceed 16 KiB.
    ///
    /// **Note:** This method returns a nested [`core::result::Result`].
    ///
    /// The outermost `Result` is for handling bus errors, similar to most of
    /// the other methods in this trait.
    ///
    /// The innermost `Result<BufferSize, u8>` is the result of a `u8` to
    /// [`BufferSize`] conversion because not every value of `u8` corresponds
    /// to a valid [`BufferSize`].
    /// * `u8` values that have a corresponding [`BufferSize`] will be
    ///   converted and returned in the [`Ok`] variant of the inner `Result`.
    /// * `u8` values that do not corresponding [`BufferSize`] will have the
    ///   raw `u8` byte returned in the [`Err`] variant of the inner `Result`.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x1F, 0x08]),
    /// #   ehm1::spi::Transaction::read(0x02),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, BufferSize, Registers, Sn};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let sn_txbuf_size = w5500.sn_txbuf_size(Sn::Sn0)?;
    /// assert_eq!(sn_txbuf_size, Ok(BufferSize::KB2));
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    ///
    /// [`Ok`]: https://doc.rust-lang.org/core/result/enum.Result.html#variant.Ok
    /// [`Err`]: https://doc.rust-lang.org/core/result/enum.Result.html#variant.Err
    fn sn_txbuf_size(&mut self, sn: Sn) -> Result<Result<BufferSize, u8>, Self::Error> {
        let mut reg: [u8; 1] = [0];
        self.read(SnReg::TXBUF_SIZE.addr(), sn.block(), &mut reg)?;
        Ok(BufferSize::try_from(reg[0]))
    }

    /// Set the socket TX buffer size.
    ///
    /// See [`Registers::sn_rxbuf_size`] for more information.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x1F, 0x08 | 0x04]),
    /// #   ehm1::spi::Transaction::write(1),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, BufferSize, Registers, Sn};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// w5500.set_sn_txbuf_size(Sn::Sn0, BufferSize::KB1)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_sn_txbuf_size(&mut self, sn: Sn, size: BufferSize) -> Result<(), Self::Error> {
        self.write(SnReg::TXBUF_SIZE.addr(), sn.block(), &[size.into()])
    }

    /// Get transmit buffer free size.
    ///
    /// This register indicates the free size of socket n TX buffer.
    /// It is initialized to the configured size by [`Registers::sn_txbuf_size`].
    /// Data bigger than Sn_TX_FSR should not be written to the TX buffer to
    /// prevent buffer overflow.
    ///
    /// Check this register before writing data to the socket TX buffer,
    /// and if data is equal or smaller than its checked size, transmit the data
    /// with the [`SocketCommand::Send`] or [`SocketCommand::SendMac`] command
    /// after saving the data in Socket n TX buffer.
    ///
    /// If data is bigger than its checked size, transmit the data after
    /// dividing into the checked size and saving in the socket TX buffer.
    ///
    /// If [`Registers::sn_mr`] is not in TCP mode, this register is
    /// automatically calculated as the difference between
    /// [`Registers::sn_tx_wr`] and [`Registers::sn_tx_rd`].
    ///
    /// If [`Registers::sn_mr`] is in TCP mode, this register is automatically
    /// calculated as the difference between the internal ACK pointer which
    /// indicates the point of data is received already by the connected
    /// peer.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x20, 0x08]),
    /// #   ehm1::spi::Transaction::read_vec(vec![0x08, 0x00]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn, SocketMode};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let sn_tx_fsr: u16 = w5500.sn_tx_fsr(Sn::Sn0)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn sn_tx_fsr(&mut self, sn: Sn) -> Result<u16, Self::Error> {
        let mut reg: [u8; 2] = [0; 2];
        self.read(SnReg::TX_FSR0.addr(), sn.block(), &mut reg)?;
        Ok(u16::from_be_bytes(reg))
    }

    /// Get the socket TX read pointer.
    ///
    /// This register is initialized by the [`SocketCommand::Open`] command.
    /// However, in TCP mode, this is re-initialized while connecting with TCP.
    ///
    /// After initialization, this is auto-increased by the
    /// [`SocketCommand::Send`] command.
    ///
    /// The [`SocketCommand::Send`] command transmits the saved data from the
    /// current [`Registers::sn_tx_rd`] to the [`Registers::sn_tx_wr`] in the
    /// socket n TX buffer.
    /// After transmitting the saved data, the [`SocketCommand::Send`] command
    /// increases [`Registers::sn_tx_rd`] the as same as
    /// [`Registers::sn_tx_wr`].
    ///
    /// If its increment value exceeds the maximum value `0xFFFF`,
    /// (greater than `0x10000` and the carry bit occurs), then the carry bit is
    /// ignored and will automatically update with the lower 16 bits value.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x22, 0x08]),
    /// #   ehm1::spi::Transaction::read_vec(vec![0, 0]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn, SocketMode};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let sn_tx_rd: u16 = w5500.sn_tx_rd(Sn::Sn0)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn sn_tx_rd(&mut self, sn: Sn) -> Result<u16, Self::Error> {
        let mut reg: [u8; 2] = [0; 2];
        self.read(SnReg::TX_RD0.addr(), sn.block(), &mut reg)?;
        Ok(u16::from_be_bytes(reg))
    }

    /// Get the socket TX write pointer.
    ///
    /// This register is initialized by the [`SocketCommand::Open`] command.
    /// However, in TCP mode, this is re-initialized while connecting with TCP.
    ///
    /// This should be updated as follows:
    /// 1. Read the starting address for transmitting data.
    /// 2. Write data to the socket TX buffer buffer.
    /// 3. Update this register by the number of bytes written to the TX buffer.
    ///    Allow wrapping to occur upon `u16` overflow.
    /// 4. Transmit the saved data in the socket TX buffer by using the
    ///    [`SocketCommand::Send`] command.
    ///
    /// See [`Registers::set_sn_tx_buf`] for an additional example.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x24, 0x08]),
    /// #   ehm1::spi::Transaction::read_vec(vec![0, 0]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let sn_tx_wr: u16 = w5500.sn_tx_wr(Sn::Sn0)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn sn_tx_wr(&mut self, sn: Sn) -> Result<u16, Self::Error> {
        let mut reg: [u8; 2] = [0; 2];
        self.read(SnReg::TX_WR0.addr(), sn.block(), &mut reg)?;
        Ok(u16::from_be_bytes(reg))
    }

    /// Set the socket TX write pointer.
    ///
    /// See [`Registers::sn_tx_wr`] for more information.
    ///
    /// See [`Registers::set_sn_tx_buf`] for an example.
    fn set_sn_tx_wr(&mut self, sn: Sn, ptr: u16) -> Result<(), Self::Error> {
        self.write(SnReg::TX_WR0.addr(), sn.block(), &ptr.to_be_bytes())
    }

    /// Get the socket TX free size and write pointer
    ///
    /// This is equivalent to [`Registers::sn_tx_fsr`] and
    /// [`Registers::sn_tx_wr`] in a single read transaction.
    fn sn_tx_ptrs(&mut self, sn: Sn) -> Result<TxPtrs, Self::Error> {
        let mut buf: [u8; 6] = [0; 6];
        self.read(SnReg::TX_FSR0.addr(), sn.block(), &mut buf)?;
        Ok(TxPtrs {
            fsr: u16::from_be_bytes(buf[..2].try_into().unwrap()),
            wr: u16::from_be_bytes(buf[4..].try_into().unwrap()),
        })
    }

    /// Get the socket received data size.
    ///
    /// This register indicates the data size received and saved in the socket
    /// RX buffer.
    /// This register does not exceed the configured size
    /// ([`Registers::set_sn_rxbuf_size`]) and is calculated as the difference
    /// between [`Registers::sn_rx_wr`] and [`Registers::sn_rx_rd`].
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x26, 0x08]),
    /// #   ehm1::spi::Transaction::read_vec(vec![0, 0]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let sn_rx_rsr: u16 = w5500.sn_rx_rsr(Sn::Sn0)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn sn_rx_rsr(&mut self, sn: Sn) -> Result<u16, Self::Error> {
        let mut reg: [u8; 2] = [0; 2];
        self.read(SnReg::RX_RSR0.addr(), sn.block(), &mut reg)?;
        Ok(u16::from_be_bytes(reg))
    }

    /// Get the socket read data pointer.
    ///
    /// This register is initialized by the [`SocketCommand::Open`] command.
    ///
    /// This should be updated as follows:
    /// 1. Read the starting address for reading data.
    /// 2. Read from the socket RX buffer.
    /// 3. Update this register by the number of bytes read from the RX buffer.
    ///    Allow wrapping to occur upon `u16` overflow.
    /// 4. Send a [`SocketCommand::Recv`] command to notify the W5500 of the
    ///    retrieved data.
    ///
    /// See [`Registers::sn_rx_buf`] for an additional example.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x28, 0x08]),
    /// #   ehm1::spi::Transaction::read_vec(vec![0, 0]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let sn_rx_rd: u16 = w5500.sn_rx_rd(Sn::Sn0)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn sn_rx_rd(&mut self, sn: Sn) -> Result<u16, Self::Error> {
        let mut reg: [u8; 2] = [0; 2];
        self.read(SnReg::RX_RD0.addr(), sn.block(), &mut reg)?;
        Ok(u16::from_be_bytes(reg))
    }

    /// Set the socket read data pointer.
    ///
    /// See [`Registers::sn_rx_rd`] for more information.
    /// See [`Registers::sn_rx_buf`] for an example.
    fn set_sn_rx_rd(&mut self, sn: Sn, ptr: u16) -> Result<(), Self::Error> {
        self.write(SnReg::RX_RD0.addr(), sn.block(), &ptr.to_be_bytes())
    }

    /// Get the socket RX write pointer.
    ///
    /// This register is initialized by the [`SocketCommand::Open`] command, and
    /// it is auto-incremented by hardware.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x2A, 0x08]),
    /// #   ehm1::spi::Transaction::read_vec(vec![0, 0]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let sn_rx_wr: u16 = w5500.sn_rx_wr(Sn::Sn0)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn sn_rx_wr(&mut self, sn: Sn) -> Result<u16, Self::Error> {
        let mut reg: [u8; 2] = [0; 2];
        self.read(SnReg::RX_WR0.addr(), sn.block(), &mut reg)?;
        Ok(u16::from_be_bytes(reg))
    }

    /// Get the socket RX recieved size size and write pointer
    ///
    /// This is equivalent to [`Registers::sn_rx_rsr`] and
    /// [`Registers::sn_rx_rd`] in a single read transaction.
    fn sn_rx_ptrs(&mut self, sn: Sn) -> Result<RxPtrs, Self::Error> {
        let mut buf: [u8; 4] = [0; 4];
        self.read(SnReg::RX_RSR0.addr(), sn.block(), &mut buf)?;
        Ok(RxPtrs {
            rsr: u16::from_be_bytes(buf[..2].try_into().unwrap()),
            rd: u16::from_be_bytes(buf[2..].try_into().unwrap()),
        })
    }

    /// Get the socket interrupt mask.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x2C, 0x08]),
    /// #   ehm1::spi::Transaction::read(0xFF),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn, SocketInterruptMask};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let sn_imr: SocketInterruptMask = w5500.sn_imr(Sn::Sn0)?;
    /// assert_eq!(sn_imr, SocketInterruptMask::default());
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn sn_imr(&mut self, sn: Sn) -> Result<SocketInterruptMask, Self::Error> {
        let mut reg: [u8; 1] = [0];
        self.read(SnReg::IMR.addr(), sn.block(), &mut reg)?;
        Ok(SocketInterruptMask::from(reg[0]))
    }

    /// Set the socket interrupt mask.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x2C, 0x08 | 0x04]),
    /// #   ehm1::spi::Transaction::write(0xE0),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn, SocketInterruptMask};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// w5500.set_sn_imr(Sn::Sn0, SocketInterruptMask::ALL_MASKED)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_sn_imr(&mut self, sn: Sn, mask: SocketInterruptMask) -> Result<(), Self::Error> {
        self.write(SnReg::IMR.addr(), sn.block(), &[mask.into()])
    }

    /// Get the socket fragment.
    ///
    /// This configures the fragment field in the IP header.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x2D, 0x08]),
    /// #   ehm1::spi::Transaction::read_vec(vec![0x40, 0x00]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let frag: u16 = w5500.sn_frag(Sn::Sn0)?;
    /// assert_eq!(frag, 0x4000);
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn sn_frag(&mut self, sn: Sn) -> Result<u16, Self::Error> {
        let mut buf: [u8; 2] = [0; 2];
        self.read(SnReg::FRAG0.addr(), sn.block(), &mut buf)?;
        Ok(u16::from_be_bytes(buf))
    }

    /// Set the socket fragment.
    ///
    /// See [`Registers::sn_frag`] for more information.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x2D, 0x08 | 0x04]),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x00]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// w5500.set_sn_frag(Sn::Sn0, 0)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_sn_frag(&mut self, sn: Sn, frag: u16) -> Result<(), Self::Error> {
        self.write(SnReg::FRAG0.addr(), sn.block(), &u16::to_be_bytes(frag))
    }

    /// Get the socket keep alive time.
    ///
    /// This register configures the transmitting timer of the keep alive (KA)
    /// packet for the socket.  This is valid only in TCP mode, and is ignored
    /// in all other modes.
    ///
    /// The time unit is 5 seconds.
    ///
    /// The KA packet is transmittable after [`Registers::sn_sr`] is changed to
    /// [`SocketStatus::Established`] and after the data is transmitted or
    /// received to/from a peer at least once.
    ///
    /// In the case of a non-zero keep alive value the W5500 automatically
    /// transmits a KA packet after time-period for checking the TCP connection
    /// (automatic-keepalive-process).
    ///
    /// In case of a zero keep alive value, the keep alive packet can be
    /// transmitted with [`SocketCommand::SendKeep`].  This command is ignored
    /// for non-zero keep alive values.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x2F, 0x08]),
    /// #   ehm1::spi::Transaction::read(0),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// let sn_kpalvtr: u8 = w5500.sn_kpalvtr(Sn::Sn0)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn sn_kpalvtr(&mut self, sn: Sn) -> Result<u8, Self::Error> {
        let mut buf: [u8; 1] = [0];
        self.read(SnReg::KPALVTR.addr(), sn.block(), &mut buf)?;
        Ok(buf[0])
    }

    /// Set the socket keep alive time.
    ///
    /// See [`Registers::sn_kpalvtr`] for more information.
    ///
    /// # Example
    ///
    /// ```
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x2F, 0x08 | 0x04]),
    /// #   ehm1::spi::Transaction::write(0x0A),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn};
    ///
    /// let mut w5500 = W5500::new(spi);
    /// // 50s keep alive timer
    /// w5500.set_sn_kpalvtr(Sn::Sn0, 10)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_sn_kpalvtr(&mut self, sn: Sn, kpalvtr: u8) -> Result<(), Self::Error> {
        self.write(SnReg::KPALVTR.addr(), sn.block(), &[kpalvtr])
    }

    /// Write the socket TX buffer.
    ///
    /// # Example
    ///
    /// ```
    /// use core::cmp::min;
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn, SocketCommand};
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   // sn_tx_fsr
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, w5500_ll::SnReg::TX_FSR0.addr() as u8, 0x08]),
    /// #   ehm1::spi::Transaction::read_vec(vec![0x08, 0x00]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// #   // sn_tx_write
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, w5500_ll::SnReg::TX_WR0.addr() as u8, 0x08]),
    /// #   ehm1::spi::Transaction::read_vec(vec![0, 0]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// #   // set_sn_tx_buf
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x00, (Sn::Sn0.tx_block() as u8) << 3 | 0x04]),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x12, 0x34, 0x56, 0x78, 0x9A]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// #   // set_sn_tx_wr
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x24, 0x08 | 0x04]),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 5]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// #   // set_sn_cr
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x01, 0x08 | 0x04]),
    /// #   ehm1::spi::Transaction::write_vec(vec![SocketCommand::Send.into()]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// # let mut w5500 = W5500::new(spi);
    ///
    /// // the socket should already be opened at this point
    /// const THE_SOCKET: Sn = Sn::Sn0;
    ///
    /// let buf: [u8; 5] = [0x12, 0x34, 0x56, 0x78, 0x9A];
    ///
    /// // transmit as many bytes as possible
    /// // for large buffers this may not transmit all the available data
    /// let tx_bytes: u16 = {
    ///     min(w5500.sn_tx_fsr(THE_SOCKET)?, u16::try_from(buf.len()).unwrap_or(u16::MAX))
    /// };
    /// if tx_bytes == 0 {
    ///     return Ok(());
    /// }
    ///
    /// let ptr: u16 = w5500.sn_tx_wr(THE_SOCKET)?;
    /// w5500.set_sn_tx_buf(THE_SOCKET, ptr, &buf[..usize::from(tx_bytes)])?;
    /// w5500.set_sn_tx_wr(THE_SOCKET, ptr.wrapping_add(tx_bytes))?;
    /// w5500.set_sn_cr(THE_SOCKET, SocketCommand::Send)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_sn_tx_buf(&mut self, sn: Sn, ptr: u16, buf: &[u8]) -> Result<(), Self::Error> {
        self.write(ptr, sn.tx_block(), buf)
    }

    /// Read the socket TX buffer.
    ///
    /// This method is typically unused; there are very few use cases that
    /// require reading the TX buffer.
    ///
    /// # Example
    ///
    /// ```
    /// use core::cmp::min;
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn, SocketCommand};
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x00, (Sn::Sn0.tx_block() as u8) << 3]),
    /// #   ehm1::spi::Transaction::read(0),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// # let mut w5500 = W5500::new(spi);
    /// let mut buf: [u8; 1] = [0];
    /// w5500.sn_tx_buf(Sn::Sn0, 0, &mut buf)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn sn_tx_buf(&mut self, sn: Sn, ptr: u16, buf: &mut [u8]) -> Result<(), Self::Error> {
        self.read(ptr, sn.tx_block(), buf)
    }

    /// Read the socket RX buffer.
    ///
    /// # Example
    ///
    /// ```
    /// use core::cmp::min;
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn, SocketCommand};
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, w5500_ll::SnReg::RX_RSR0.addr() as u8, 0x08]),
    /// #   ehm1::spi::Transaction::read_vec(vec![0, 4]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, w5500_ll::SnReg::RX_RD0.addr() as u8, 0x08]),
    /// #   ehm1::spi::Transaction::read_vec(vec![0, 0]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x00, (Sn::Sn0.rx_block() as u8) << 3]),
    /// #   ehm1::spi::Transaction::read_vec(vec![0, 0, 0, 0]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, w5500_ll::SnReg::RX_RD0.addr() as u8, 0x08 | 0x04]),
    /// #   ehm1::spi::Transaction::write_vec(vec![0, 4]),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, w5500_ll::SnReg::CR.addr() as u8, 0x08 | 0x04]),
    /// #   ehm1::spi::Transaction::write(SocketCommand::Recv.into()),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// # let mut w5500 = W5500::new(spi);
    ///
    /// // the socket should already be opened at this point
    /// // a socket interrupt will indicate there is data to be retrieved
    /// const THE_SOCKET: Sn = Sn::Sn0;
    ///
    /// // in reality you will need a larger buffer for most protocols
    /// const BUF_LEN: usize = 16;
    /// let mut buf: [u8; BUF_LEN] = [0; BUF_LEN];
    ///
    /// let rx_bytes: u16 = {
    ///     min(w5500.sn_rx_rsr(THE_SOCKET)?, u16::try_from(buf.len()).unwrap_or(u16::MAX))
    /// };
    /// if rx_bytes == 0 {
    ///     return Ok(());
    /// }
    ///
    /// let ptr: u16 = w5500.sn_rx_rd(THE_SOCKET)?;
    /// w5500.sn_rx_buf(THE_SOCKET, ptr, &mut buf[..usize::from(rx_bytes)])?;
    /// w5500.set_sn_rx_rd(THE_SOCKET, ptr.wrapping_add(rx_bytes))?;
    /// w5500.set_sn_cr(THE_SOCKET, SocketCommand::Recv)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn sn_rx_buf(&mut self, sn: Sn, ptr: u16, buf: &mut [u8]) -> Result<(), Self::Error> {
        self.read(ptr, sn.rx_block(), buf)
    }

    /// Write the socket RX buffer.
    ///
    /// This method is typically unused; there are very few use cases that
    /// require writing the RX buffer.
    ///
    /// # Example
    ///
    /// ```
    /// use core::cmp::min;
    /// use w5500_ll::{eh1::vdm::W5500, Registers, Sn, SocketCommand};
    /// # let spi = ehm1::spi::Mock::new(&[
    /// #   ehm1::spi::Transaction::transaction_start(),
    /// #   ehm1::spi::Transaction::write_vec(vec![0x00, 0x00, (Sn::Sn0.rx_block() as u8) << 3 | 0x04]),
    /// #   ehm1::spi::Transaction::write(0),
    /// #   ehm1::spi::Transaction::transaction_end(),
    /// # ]);
    /// # let mut w5500 = W5500::new(spi);
    /// let buf: [u8; 1] = [0];
    /// w5500.set_sn_rx_buf(Sn::Sn0, 0, &buf)?;
    /// # Ok::<(), eh1::spi::ErrorKind>(())
    /// ```
    fn set_sn_rx_buf(&mut self, sn: Sn, ptr: u16, buf: &[u8]) -> Result<(), Self::Error> {
        self.write(ptr, sn.rx_block(), buf)
    }
}
