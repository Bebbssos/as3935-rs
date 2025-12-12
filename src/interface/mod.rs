use core::fmt::{Display, Formatter};
use core::time::Duration;

#[cfg(feature = "std")]
use std::boxed::Box;

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

pub(crate) mod conversion;
pub mod i2c;
pub mod spi;

pub const CLOCK_GENERATION_DELAY: Duration = Duration::from_millis(2);
pub const IRQ_TRIGGER_TO_READY_DELAY: Duration = Duration::from_millis(2);
pub const LIGHTNING_CALCULATION_DELAY: Duration = Duration::from_millis(2);
pub const DISTURBER_DEACTIVATION_PERIOD: Duration = Duration::from_millis(1500);
pub const APPROXIMATE_MINIMUM_LIGHTNING_INTERVAL: Duration = Duration::from_secs(1);

pub(crate) type Result<T> = ::core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Spi,
    I2c,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> ::core::result::Result<(), ::core::fmt::Error> {
        match self {
            Error::Spi => write!(f, "SPI communication error"),
            Error::I2c => write!(f, "I2C communication error"),
        }
    }
}

pub(crate) enum Irq {
    DistanceEstimationChanged,
    /// INT_NH
    NoiseLevelTooHigh,
    /// INT_D
    DisturberDetected,
    /// INT_L
    Lightning,
}

pub(crate) trait Interface: Send {
    fn read(&mut self, register: Box<dyn crate::device::registers::Register>) -> Result<u8>;
    fn write(
        &mut self,
        register: Box<dyn crate::device::registers::Register>,
        payload: u8,
    ) -> Result<()>;
}

pub(crate) fn calculate_bitshift(mask: u8) -> u8 {
    for i in 0..7 {
        if (mask & (1 << i)) == 1 {
            return i;
        }
    }

    0
}
