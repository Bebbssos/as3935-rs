use crate::interface::Interface;
use crate::interface::{calculate_bitshift, Result};
use embedded_hal::i2c::I2c;

#[cfg(feature = "std")]
use std::boxed::Box;

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

pub const DEFAULT_I2C_ADDRESS: u8 = 0x03;

pub struct I2cAddress(pub(crate) u8);

impl I2cAddress {
    pub fn new(address: u8) -> Self {
        if address > 127 {
            panic!("invalid I2C address")
        }

        Self(address)
    }

    pub fn default() -> Self {
        Self::new(DEFAULT_I2C_ADDRESS)
    }
}

pub(crate) struct I2cInterface<I2C> {
    i2c: I2C,
    address: u8,
}

impl<I2C> I2cInterface<I2C>
where
    I2C: I2c,
{
    pub(crate) fn new(i2c: I2C, i2c_address: I2cAddress) -> Result<Self> {
        Ok(Self {
            i2c,
            address: i2c_address.0,
        })
    }
}

impl<I2C> Interface for I2cInterface<I2C>
where
    I2C: I2c + Send,
{
    fn read(&mut self, register: Box<dyn crate::device::registers::Register>) -> Result<u8> {
        let mut data: [u8; 1] = [0];

        self.i2c
            .write_read(self.address, &[register.address()], &mut data)
            .map_err(|_| crate::interface::Error::I2c)?;

        let value = (data[0] & register.mask()) >> calculate_bitshift(register.mask());
        debug!("read {} = {:#b}", register.name(), value);

        Ok(value)
    }

    fn write(
        &mut self,
        register: Box<dyn crate::device::registers::Register>,
        payload: u8,
    ) -> Result<()> {
        debug!("setting {} = {:#b}", register.name(), payload);

        let bitshift = calculate_bitshift(register.mask());
        assert!(payload <= (register.mask() >> bitshift));

        let mut current_data: [u8; 1] = [0];
        self.i2c
            .write_read(self.address, &[register.address()], &mut current_data)
            .map_err(|_| crate::interface::Error::I2c)?;

        self.i2c
            .write(
                self.address,
                &[
                    register.address(),
                    (current_data[0] ^ register.mask()) | (payload << bitshift),
                ],
            )
            .map_err(|_| crate::interface::Error::I2c)?;

        Ok(())
    }
}
