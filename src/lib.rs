#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "std")]
use std::boxed::Box;

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

#[macro_use]
extern crate log;

use crate::device::registers::{
    AfeGainBoost, CalibrateOscillators, DisplayTrcoOnIrqPin, DistanceEstimation, Interrupt,
    MaskDisturber, MinimumNumberOfLightning, NoiseFloorLevel, PowerDown, PresetDefault,
    WatchdogThreshold,
};
use crate::interface::i2c::{I2cAddress, I2cInterface};
use crate::interface::spi::SpiInterface;
use crate::interface::{Interface, Irq};

pub(crate) mod device;
pub mod interface;

#[derive(Debug)]
pub enum Error {
    Deadlock,
    InterfaceError(interface::Error),
    InvalidState,
}

pub type Result<T> = core::result::Result<T, Error>;

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        match self {
            Error::Deadlock => write!(f, "Deadlock occurred"),
            Error::InterfaceError(e) => write!(f, "Interface error: {}", e),
            Error::InvalidState => write!(f, "Invalid state"),
        }
    }
}

impl From<crate::interface::Error> for Error {
    fn from(error: crate::interface::Error) -> Self {
        Error::InterfaceError(error)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SensorPlacing {
    Indoor,
    Outdoor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MinimumLightningThreshold {
    One,
    Five,
    Nine,
    Sixteen,
}

/// Larger values correspond to more robust disturber rejection, with a decrease of the detection efficiency,
/// Refer to Figure 20 in the datasheet for the relationship between this threshold and its impact.
/// Defaults to 2.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SignalVerificationThreshold(pub(crate) u8);

impl SignalVerificationThreshold {
    pub fn new(value: u8) -> core::result::Result<Self, &'static str> {
        if value > 10 {
            return Err("Signal verification threshold must be in range 0-10");
        }

        Ok(Self(value))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NoiseFloorThreshold(pub(crate) u8);

impl NoiseFloorThreshold {
    pub fn new(value: u8) -> core::result::Result<Self, &'static str> {
        if value > 11 {
            return Err("Noise level threshold must be in range 0-11");
        }

        Ok(Self(value))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IgnoreDisturbances {
    Yes,
    No,
}

/// Estimated distance to the head of storm, in kilometers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HeadOfStormDistance {
    /// the storm is within 5-40 km range
    Kilometers(u8),
    /// the storm is out of range (>40 km)
    OutOfRange,
    /// the storm is overhead (<5 km)
    Overhead,
}

pub enum InterfaceSelection<I2C, SPI> {
    I2c(I2C, I2cAddress),
    Spi(SPI),
}

pub enum Event {
    Disturbance,
    Lightning(HeadOfStormDistance),
    Noise,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum State {
    Listening,
    PoweredDown,
    StandingBy,
}

#[derive(Default)]
pub struct ListeningParameters {
    pub(crate) sensor_placing: Option<SensorPlacing>,
    pub(crate) minimum_lightning_threshold: Option<MinimumLightningThreshold>,
    pub(crate) noise_floor_threshold: Option<NoiseFloorThreshold>,
    pub(crate) signal_verification_threshold: Option<SignalVerificationThreshold>,
    pub(crate) ignore_disturbances: Option<IgnoreDisturbances>,
}

impl ListeningParameters {
    pub fn with_sensor_placing(mut self, sensor_placing: SensorPlacing) -> Self {
        self.sensor_placing = Some(sensor_placing);
        self
    }

    pub fn with_minimum_lightning_threshold(
        mut self,
        minimum_lightning_threshold: MinimumLightningThreshold,
    ) -> Self {
        self.minimum_lightning_threshold = Some(minimum_lightning_threshold);
        self
    }

    pub fn with_noise_floor_threshold(
        mut self,
        noise_floor_threshold: NoiseFloorThreshold,
    ) -> Self {
        self.noise_floor_threshold = Some(noise_floor_threshold);
        self
    }

    pub fn with_signal_verification_threshold(
        mut self,
        signal_verification_threshold: SignalVerificationThreshold,
    ) -> Self {
        self.signal_verification_threshold = Some(signal_verification_threshold);
        self
    }

    pub fn with_ignore_disturbances(mut self, ignore_disturbances: IgnoreDisturbances) -> Self {
        self.ignore_disturbances = Some(ignore_disturbances);
        self
    }
}

pub struct AS3935<INTERFACE>
where
    INTERFACE: Interface,
{
    interface: INTERFACE,
    state: State,
}

impl<I2C> AS3935<I2cInterface<I2C>>
where
    I2C: embedded_hal::i2c::I2c + Send,
{
    /// Create a new AS3935 driver with an I2C interface
    pub fn new_i2c(i2c: I2C, i2c_address: I2cAddress) -> Result<Self> {
        Ok(Self {
            interface: I2cInterface::new(i2c, i2c_address)?,
            state: State::StandingBy,
        })
    }
}

impl<SPI> AS3935<SpiInterface<SPI>>
where
    SPI: embedded_hal::spi::SpiDevice + Send,
{
    /// Create a new AS3935 driver with an SPI interface
    pub fn new_spi(spi: SPI) -> Result<Self> {
        Ok(Self {
            interface: SpiInterface::new(spi)?,
            state: State::StandingBy,
        })
    }
}

impl<INTERFACE> AS3935<INTERFACE>
where
    INTERFACE: Interface,
{
    /// Initialize the sensor and start listening for events
    /// 
    /// This configures the sensor with the specified parameters.
    /// Users must poll `check_irq()` to detect events, as embedded-hal
    /// does not provide interrupt abstractions.
    pub fn listen(&mut self, parameters: ListeningParameters) -> Result<()> {
        self.assert_state(&self.state, &[State::StandingBy, State::PoweredDown])?;

        info!("starting listen sequence");

        debug!("powering up");
        self.power_up()?;

        debug!("calibrating clock");
        self.calibrate_clock()?;

        debug!("resetting to defaults");
        self.configure_defaults()?;

        debug!("configuring listen parameters");
        self.configure_listen_parameters(parameters)?;

        self.state = State::Listening;

        Ok(())
    }

    /// Check if an interrupt has occurred and return the event
    /// 
    /// This should be called when the IRQ pin goes high.
    /// Users are responsible for monitoring the IRQ pin in their platform-specific code.
    pub fn check_irq(&mut self) -> Result<Option<Event>> {
        if self.state != State::Listening {
            return Ok(None);
        }

        let irq = Irq::from(self.interface.read(Box::new(Interrupt))?);

        let event = match irq {
            Irq::DistanceEstimationChanged => return Ok(None),
            Irq::DisturberDetected => Event::Disturbance,
            Irq::Lightning => {
                // In a real implementation, the user would need to wait LIGHTNING_CALCULATION_DELAY
                // before calling this or handle it externally
                Event::Lightning(HeadOfStormDistance::from(
                    self.interface.read(Box::new(DistanceEstimation))?,
                ))
            }
            Irq::NoiseLevelTooHigh => Event::Noise,
        };

        Ok(Some(event))
    }

    /// Stop listening and power down the sensor
    pub fn terminate(&mut self) -> Result<()> {
        self.assert_state(&self.state, &[State::Listening])?;

        self.power_down()?;
        self.state = State::PoweredDown;

        Ok(())
    }

    /// Check if the sensor is currently listening
    pub fn is_listening(&self) -> bool {
        self.state == State::Listening
    }

    fn power_up(&mut self) -> Result<()> {
        self.assert_state(&self.state, &[State::StandingBy, State::PoweredDown])?;

        self.interface.write(Box::new(PowerDown), 0b_0)?;
        // Note: In embedded environments, users should implement their own delay function
        // For no_std compatibility, we can't use std::thread::sleep here
        // The delay is 2ms as per spec

        Ok(())
    }

    fn power_down(&mut self) -> Result<()> {
        self.interface.write(Box::new(PowerDown), 0b_1)?;

        Ok(())
    }

    fn calibrate_clock(&mut self) -> Result<()> {
        self.assert_state(&self.state, &[State::StandingBy, State::PoweredDown])?;

        debug!("sending CALIB_RCO direct command");
        self.interface
            .write(Box::new(CalibrateOscillators), 0x96)?;
        // Need 2ms delay here

        debug!("setting DISP_TRCO=1");
        self.interface
            .write(Box::new(DisplayTrcoOnIrqPin), 0b_1)?;

        // Need CLOCK_GENERATION_DELAY here

        debug!("setting DISP_TRCO=0");
        self.interface.write(Box::new(DisplayTrcoOnIrqPin), 0)?;
        // Need 2ms delay here

        Ok(())
    }

    fn configure_defaults(&mut self) -> Result<()> {
        self.interface
            .write(Box::new(PresetDefault), 0x96)?;

        Ok(())
    }

    fn configure_listen_parameters(&mut self, parameters: ListeningParameters) -> Result<()> {
        if let Some(sensor_placing) = parameters.sensor_placing {
            debug!("configuring sensor placing");
            self.configure_sensor_placing(&sensor_placing)?;
        }

        if let Some(minimum_lightning_threshold) = &parameters.minimum_lightning_threshold {
            debug!("configuring minimum lightning threshold");
            self.configure_minimum_lightning_threshold(&minimum_lightning_threshold)?;
        }

        if let Some(noise_floor_threshold) = &parameters.noise_floor_threshold {
            debug!("configuring noise floor threshold");
            self.configure_noise_floor_threshold(&noise_floor_threshold)?;
        }

        if let Some(signal_verification_threshold) = &parameters.signal_verification_threshold {
            debug!("configuring signal verification threshold");
            self.configure_signal_verification_threshold(&signal_verification_threshold)?;
        }

        if let Some(ignore_disturbances) = &parameters.ignore_disturbances {
            debug!("configuring ignoring of disturbances");
            self.configure_ignore_disturbances(&ignore_disturbances)?;
        }

        Ok(())
    }

    fn configure_sensor_placing(&mut self, placing: &SensorPlacing) -> Result<()> {
        self.interface
            .write(Box::new(AfeGainBoost), (*placing).into())?;

        Ok(())
    }

    fn configure_minimum_lightning_threshold(
        &mut self,
        minimum_lightning_threshold: &MinimumLightningThreshold,
    ) -> Result<()> {
        self.interface.write(
            Box::new(MinimumNumberOfLightning),
            (*minimum_lightning_threshold).into(),
        )?;

        Ok(())
    }

    fn configure_noise_floor_threshold(
        &mut self,
        noise_floor_threshold: &NoiseFloorThreshold,
    ) -> Result<()> {
        self.interface
            .write(Box::new(NoiseFloorLevel), (*noise_floor_threshold).into())?;

        Ok(())
    }

    fn configure_signal_verification_threshold(
        &mut self,
        signal_verification_threshold: &SignalVerificationThreshold,
    ) -> Result<()> {
        self.interface.write(
            Box::new(WatchdogThreshold),
            (*signal_verification_threshold).into(),
        )?;

        Ok(())
    }

    fn configure_ignore_disturbances(
        &mut self,
        ignore_disturbances: &IgnoreDisturbances,
    ) -> Result<()> {
        self.interface
            .write(Box::new(MaskDisturber), (*ignore_disturbances).into())?;

        Ok(())
    }

    fn assert_state(&self, state: &State, valid_states: &[State]) -> Result<()> {
        if !valid_states.contains(state) {
            return Err(Error::InvalidState);
        }

        Ok(())
    }
}
