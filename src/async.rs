use super::{register, KeyRaw, KeyStatus, Version, KBD_ADDR};
use embedded_hal_async::i2c::{I2c, Operation::*};

/// A struct representing an asynchronous driver for the BlackBerry Q10 PMOD
/// Keyboard
pub struct AsyncBbq10Kbd<I2C>
where
    I2C: I2c,
{
    i2c: I2C,
}

impl<I2C> AsyncBbq10Kbd<I2C>
where
    I2C: I2c,
{
    /// Create a new async BBQ10 Keyboard instance
    #[must_use]
    pub fn new(i2c: I2C) -> Self {
        Self { i2c }
    }

    /// Consume self, returning the inner I2C device
    #[must_use]
    pub fn release(self) -> I2C {
        self.i2c
    }

    /// Get the version reported by the keyboard's firmware
    pub async fn get_version(&mut self) -> Result<Version, I2C::Error> {
        let mut buf = [0u8; 1];

        self.i2c
            .transaction(
                KBD_ADDR,
                &mut [Write(&[register::VERSION]), Read(&mut buf[..])],
            )
            .await?;

        Ok(Version::from_byte(buf[0]))
    }

    /// Obtain a single fifo item from the keyboard's firmware
    pub async fn get_fifo_key_raw(&mut self) -> Result<KeyRaw, I2C::Error> {
        let mut buf = [0u8; 2];

        self.i2c
            .transaction(
                KBD_ADDR,
                &mut [Write(&[register::FIFO]), Read(&mut buf[..])],
            )
            .await?;

        Ok(KeyRaw::from_bytes(buf))
    }

    /// Get the current level of backlight. All u8 values are valid
    pub async fn get_backlight(&mut self) -> Result<u8, I2C::Error> {
        let mut buf = [0u8; 1];

        self.i2c
            .transaction(
                KBD_ADDR,
                &mut [Write(&[register::BACKLIGHT]), Read(&mut buf[..])],
            )
            .await?;

        Ok(buf[0])
    }

    /// Set the current level of backlight. All u8 values are valid
    pub async fn set_backlight(&mut self, level: u8) -> Result<(), I2C::Error> {
        let mut buf = [0u8; 2];

        buf[0] = register::BACKLIGHT | register::WRITE;
        buf[1] = level;

        self.i2c.write(KBD_ADDR, &buf).await
    }

    /// Reset the device via software
    ///
    /// WARNING: Device may take >= 10ms to reboot. It
    /// will not be responsive during this time
    pub async fn sw_reset(&mut self) -> Result<(), I2C::Error> {
        let mut buf = [0u8; 1];

        buf[0] = register::RESET;

        // This is enough to reset the device
        self.i2c.write(KBD_ADDR, &buf).await
    }

    /// Get the reported status of the keyboard
    pub async fn get_key_status(&mut self) -> Result<KeyStatus, I2C::Error> {
        let mut buf = [0u8; 1];

        self.i2c
            .transaction(
                KBD_ADDR,
                &mut [Write(&[register::KEY_STATUS]), Read(&mut buf[..])],
            )
            .await?;

        Ok(KeyStatus::from_byte(buf[0]))
    }
}
