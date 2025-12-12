use core::fmt::{Display, Formatter};
use core::time::Duration;

#[cfg(feature = "std")]
use std::boxed::Box;

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

pub(crate) mod conversion;
pub mod i2c;
pub mod spi;

/// Internal timing constant for clock generation delay (2ms)
#[allow(dead_code)]
pub(crate) const CLOCK_GENERATION_DELAY: Duration = Duration::from_millis(2);
/// Internal timing constant for IRQ trigger to ready delay (2ms)
#[allow(dead_code)]
pub(crate) const IRQ_TRIGGER_TO_READY_DELAY: Duration = Duration::from_millis(2);
/// Internal timing constant for lightning calculation delay (2ms)
#[allow(dead_code)]
pub(crate) const LIGHTNING_CALCULATION_DELAY: Duration = Duration::from_millis(2);
/// Recommended minimum time to wait after disturber detection before re-enabling
pub const DISTURBER_DEACTIVATION_PERIOD: Duration = Duration::from_millis(1500);
/// Approximate minimum interval between lightning events
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
    for i in 0..8 {
        if (mask & (1 << i)) != 0 {
            return i;
        }
    }

    0
}
