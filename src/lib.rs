//! An embedded-hal crate for @arturo182's BlackBerry Q10 PMOD Keyboard
//!
//! Written on James' [Office Hours Stream](https://youtu.be/GQ0uzTIx9gY)


#![no_std]

use embedded_hal::blocking::i2c::{Read, Write};

// DEFAULT
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
    I2C: Read + Write
{
    i2c: I2C
}

/// The version identifier of our keyboard's firmware
#[derive(Debug, PartialEq, Eq)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
}

/// A raw key event from the management FIFO
#[derive(Debug)]
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
#[derive(Debug)]
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
#[derive(Debug)]
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
#[derive(Debug)]
pub enum FifoCount {
    Known(u8),
    EmptyOr32,
}

/// The current key status register reported by the keyboard firmware
#[derive(Debug)]
pub struct KeyStatus {
    num_lock: NumLockState,
    caps_lock: CapsLockState,
    fifo_count: FifoCount,
}

impl<I2C> Bbq10Kbd<I2C>
where
    I2C: Read + Write
{
    /// Create a new
    pub fn new(i2c: I2C) -> Self {
        Self {
            i2c
        }
    }

    pub fn release(self) -> I2C {
        self.i2c
    }

    pub fn get_version(&mut self) -> Result<Version> {
        const VERSION_REGISTER: u8 = 0x01;
        let mut buf = [0u8; 1];

        buf[0] = VERSION_REGISTER;

        self.i2c
            .write(KBD_ADDR, &buf)
            .map_err(|_| Error::I2c)?;

        buf[0] = 0;

        self.i2c
            .read(KBD_ADDR, &mut buf)
            .map_err(|_| Error::I2c)?;

        let val = buf[0];

        Ok(Version {
            major: (val & 0xF0) >> 4,
            minor: (val & 0x0F),
        })
    }

    pub fn get_fifo_key_raw(&mut self) -> Result<KeyRaw> {
        const FIFO_REGISTER: u8 = 0x09;
        let mut buf = [0u8; 2];

        buf[0] = FIFO_REGISTER;

        self.i2c
            .write(KBD_ADDR, &buf[..1])
            .map_err(|_| Error::I2c)?;

        buf[0] = 0;

        self.i2c
            .read(KBD_ADDR, &mut buf)
            .map_err(|_| Error::I2c)?;

        Ok(match buf {
            [1, n] => KeyRaw::Pressed(n),
            [2, n] => KeyRaw::Held(n),
            [3, n] => KeyRaw::Released(n),
            [_, _] => KeyRaw::Invalid,
        })
    }

    pub fn get_backlight(&mut self) -> Result<u8> {
        const BACKLIGHT_REGISTER_READ: u8 = 0x05;
        let mut buf = [0u8; 1];

        buf[0] = BACKLIGHT_REGISTER_READ;

        self.i2c
            .write(KBD_ADDR, &buf)
            .map_err(|_| Error::I2c)?;

        buf[0] = 0;

        self.i2c
            .read(KBD_ADDR, &mut buf)
            .map_err(|_| Error::I2c)?;

        Ok(buf[0])
    }

    pub fn set_backlight(&mut self, level: u8) -> Result<()> {
        const BACKLIGHT_REGISTER_WRITE: u8 = 0x85;
        let mut buf = [0u8; 2];

        buf[0] = BACKLIGHT_REGISTER_WRITE;
        buf[1] = level;

        self.i2c
            .write(KBD_ADDR, &buf)
            .map_err(|_| Error::I2c)
    }

    /// Reset the device via software
    ///
    /// WARNING: Device may take >= 10ms to reboot. It
    /// will not be responsive during this time
    pub fn sw_reset(&mut self) -> Result<()> {
        const RESET_REGISTER: u8 = 0x08;
        let mut buf = [0u8; 1];

        buf[0] = RESET_REGISTER;

        // This is enough to reset the device
        self.i2c
            .write(KBD_ADDR, &buf)
            .map_err(|_| Error::I2c)
    }

    pub fn get_key_status(&mut self) -> Result<KeyStatus> {
        const KEY_STATUS_REGISTER: u8 = 0x04;
        let mut buf = [0u8; 1];

        buf[0] = KEY_STATUS_REGISTER;

        self.i2c
            .write(KBD_ADDR, &buf)
            .map_err(|_| Error::I2c)?;

        buf[0] = 0;

        self.i2c
            .read(KBD_ADDR, &mut buf)
            .map_err(|_| Error::I2c)?;

        let mut resp = buf[0];

        let num_lock = if (resp & 0b0100_0000) != 0 {
            NumLockState::On
        } else {
            NumLockState::Off
        };
        resp = resp & 0b1011_1111;

        let capslock = (resp & 0b0010_0000) != 0;
        let fifo_ct = resp & 0b0001_1111;

        Ok(match (capslock, fifo_ct) {
            (true, 0) => {
                KeyStatus {
                    caps_lock: CapsLockState::Unknown,
                    num_lock,
                    fifo_count: FifoCount::EmptyOr32,
                }
            },
            (true, n) => {
                KeyStatus {
                    caps_lock: CapsLockState::On,
                    num_lock,
                    fifo_count: FifoCount::Known(n),
                }
            }
            (false, n) => {
                KeyStatus {
                    caps_lock: CapsLockState::Off,
                    num_lock,
                    fifo_count: FifoCount::Known(n),
                }
            }
        })
    }

}


// Interface for this keyboard
// To read the FW version register
//
// 1. WRITE (ADDRESS) 0x01
// 2. READ (ADDRESS) (read 1 byte)
