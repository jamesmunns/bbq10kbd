//! An embedded-hal crate for @arturo182's BlackBerry Q10 PMOD Keyboard
//!
//! Written on James' [Office Hours Stream](https://youtu.be/GQ0uzTIx9gY)
//!

// Developer's note:
//
// Interface for this keyboard
// To read the FW version register
//
// 1. WRITE (ADDRESS) 0x01
// 2. READ (ADDRESS) (read 1 byte)

#![no_std]

use embedded_hal::blocking::i2c::{Read, Write};

#[cfg(feature = "embedded-hal-async")]
mod r#async;
#[cfg(feature = "embedded-hal-async")]
pub use r#async::AsyncBbq10Kbd;

// DEFAULT ADDRESS, not currently changeable
const KBD_ADDR: u8 = 0x1F;

/// The Error type for this crate
#[derive(Debug)]
pub enum Error {
    /// A generic embedded-hal I2C error
    I2c,
}

/// The Result type for this crate
pub type Result<T> = core::result::Result<T, Error>;

/// A struct representing our BlackBerry Q10 PMOD Keyboard
pub struct Bbq10Kbd<I2C>
where
    I2C: Read + Write,
{
    i2c: I2C,
}

/// The version identifier of our keyboard's firmware
#[derive(Debug, PartialEq, Eq)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
}

/// A raw key event from the management FIFO
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum KeyRaw {
    Invalid,
    Pressed(u8),
    Held(u8),
    Released(u8),
}

/// The current state of NumLock
///
/// Numlock is enabled by pressing `alt` + `Left Shift`, and
/// disabled by double clicking either Shift key
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NumLockState {
    Off,
    On,
}

/// The current state of CapsLock
///
/// Capslock is enabled by pressing `alt` + `Right Shift`, and
/// disabled by double clicking either Shift key.
///
/// NOTE: Due to a firmware bug, the Fifo count may roll over
/// into the CapsLock bit. In this case, an `Unknown` value will
/// be returned.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CapsLockState {
    Off,
    On,
    Unknown,
}

/// The current number of key events waiting in the FIFO queue
///
/// Each event contains a state + key id
///
/// NOTE: Due to a firmware bug, the Fifo count may roll over
/// into the CapsLock bit. In this case, an `EmptyOr32` value will
/// be returned, and there are either zero or 32 elements in the
/// fifo currently.

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FifoCount {
    Known(u8),
    EmptyOr32,
}

/// The current key status register reported by the keyboard firmware
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct KeyStatus {
    pub num_lock: NumLockState,
    pub caps_lock: CapsLockState,
    pub fifo_count: FifoCount,
}

pub(crate) mod register {
    pub(crate) const WRITE: u8 = 0x80;

    pub(crate) const VERSION: u8 = 0x01;

    pub(crate) const KEY_STATUS: u8 = 0x04;
    pub(crate) const BACKLIGHT: u8 = 0x05;
    pub(crate) const RESET: u8 = 0x08;
    pub(crate) const FIFO: u8 = 0x09;
}

impl<I2C> Bbq10Kbd<I2C>
where
    I2C: Read + Write,
{
    /// Create a new BBQ10 Keyboard instance
    pub fn new(i2c: I2C) -> Self {
        Self { i2c }
    }

    /// Consume self, returning the inner I2C device
    pub fn release(self) -> I2C {
        self.i2c
    }

    /// Get the version reported by the keyboard's firmware
    pub fn get_version(&mut self) -> Result<Version> {
        let mut buf = [0u8; 1];

        buf[0] = register::VERSION;

        self.i2c.write(KBD_ADDR, &buf).map_err(|_| Error::I2c)?;

        buf[0] = 0;

        self.i2c.read(KBD_ADDR, &mut buf).map_err(|_| Error::I2c)?;

        let val = buf[0];

        Ok(Version::from_byte(val))
    }

    /// Obtain a single fifo item from the keyboard's firmware
    pub fn get_fifo_key_raw(&mut self) -> Result<KeyRaw> {
        let mut buf = [0u8; 2];

        buf[0] = register::FIFO;

        self.i2c
            .write(KBD_ADDR, &buf[..1])
            .map_err(|_| Error::I2c)?;

        buf[0] = 0;

        self.i2c.read(KBD_ADDR, &mut buf).map_err(|_| Error::I2c)?;

        Ok(KeyRaw::from_bytes(buf))
    }

    /// Get the current level of backlight. All u8 values are valid
    pub fn get_backlight(&mut self) -> Result<u8> {
        let mut buf = [0u8; 1];

        buf[0] = register::BACKLIGHT;

        self.i2c.write(KBD_ADDR, &buf).map_err(|_| Error::I2c)?;

        buf[0] = 0;

        self.i2c.read(KBD_ADDR, &mut buf).map_err(|_| Error::I2c)?;

        Ok(buf[0])
    }

    /// Set the current level of backlight. All u8 values are valid
    pub fn set_backlight(&mut self, level: u8) -> Result<()> {
        let mut buf = [0u8; 2];

        buf[0] = register::BACKLIGHT | register::WRITE;
        buf[1] = level;

        self.i2c.write(KBD_ADDR, &buf).map_err(|_| Error::I2c)
    }

    /// Reset the device via software
    ///
    /// WARNING: Device may take >= 10ms to reboot. It
    /// will not be responsive during this time
    pub fn sw_reset(&mut self) -> Result<()> {
        let mut buf = [0u8; 1];

        buf[0] = register::RESET;

        // This is enough to reset the device
        self.i2c.write(KBD_ADDR, &buf).map_err(|_| Error::I2c)
    }

    /// Get the reported status of the keyboard
    pub fn get_key_status(&mut self) -> Result<KeyStatus> {
        let mut buf = [0u8; 1];

        buf[0] = register::KEY_STATUS;

        self.i2c.write(KBD_ADDR, &buf).map_err(|_| Error::I2c)?;

        buf[0] = 0;

        self.i2c.read(KBD_ADDR, &mut buf).map_err(|_| Error::I2c)?;

        Ok(KeyStatus::from_byte(buf[0]))
    }
}

impl Version {
    pub(crate) fn from_byte(byte: u8) -> Self {
        Self {
            major: (byte & 0xF0) >> 4,
            minor: (byte & 0x0F),
        }
    }
}

impl KeyRaw {
    pub(crate) fn from_bytes(buf: [u8; 2]) -> Self {
        match buf {
            [1, n] => Self::Pressed(n),
            [2, n] => Self::Held(n),
            [3, n] => Self::Released(n),
            [_, _] => Self::Invalid,
        }
    }
}

impl KeyStatus {
    pub(crate) fn from_byte(mut byte: u8) -> Self {
        let num_lock = if (byte & 0b0100_0000) != 0 {
            NumLockState::On
        } else {
            NumLockState::Off
        };
        byte = byte & 0b1011_1111;

        let capslock = (byte & 0b0010_0000) != 0;
        let fifo_ct = byte & 0b0001_1111;

        match (capslock, fifo_ct) {
            (true, 0) => Self {
                caps_lock: CapsLockState::Unknown,
                num_lock,
                fifo_count: FifoCount::EmptyOr32,
            },
            (true, n) => Self {
                caps_lock: CapsLockState::On,
                num_lock,
                fifo_count: FifoCount::Known(n),
            },
            (false, n) => Self {
                caps_lock: CapsLockState::Off,
                num_lock,
                fifo_count: FifoCount::Known(n),
            },
        }
    }
}
