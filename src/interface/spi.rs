use crate::interface::Interface;
use crate::interface::{calculate_bitshift, Result};
use embedded_hal::spi::SpiDevice;

#[cfg(feature = "std")]
use std::boxed::Box;

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

pub(crate) struct SpiInterface<SPI> {
    spi: SPI,
}

impl<SPI> SpiInterface<SPI>
where
    SPI: SpiDevice,
{
    pub(crate) fn new(spi: SPI) -> Result<Self> {
        Ok(Self { spi })
    }
}

impl<SPI> Interface for SpiInterface<SPI>
where
    SPI: SpiDevice + Send,
{
    fn read(&mut self, register: Box<dyn crate::device::registers::Register>) -> Result<u8> {
        // SPI read command: 0b01AAAAAA (read bit set, followed by 6-bit address)
        let cmd = 0x40 | (register.address() & 0x3F);
        let mut buffer = [cmd, 0x00];

        self.spi
            .transfer_in_place(&mut buffer)
            .map_err(|_| crate::interface::Error::Spi)?;

        let value = (buffer[1] & register.mask()) >> calculate_bitshift(register.mask());
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

        // Read current value first
        let cmd_read = 0x40 | (register.address() & 0x3F);
        let mut buffer = [cmd_read, 0x00];

        self.spi
            .transfer_in_place(&mut buffer)
            .map_err(|_| crate::interface::Error::Spi)?;

        let current_data = buffer[1];

        // SPI write command: 0b00AAAAAA (write bit clear, followed by 6-bit address)
        let cmd_write = register.address() & 0x3F;
        let new_value = (current_data ^ register.mask()) | (payload << bitshift);
        let write_buffer = [cmd_write, new_value];

        self.spi
            .write(&write_buffer)
            .map_err(|_| crate::interface::Error::Spi)?;

        Ok(())
    }
}
