# Rust I²C/SPI driver for AS3935 Franklin Lightning Sensor IC

[![Build Status](https://travis-ci.org/trashware/as3935-rs.svg?branch=master)](https://travis-ci.org/trashware/as3935-rs)
[![crates.io](https://meritbadge.herokuapp.com/as3935)](https://crates.io/crates/as3935)

This crate provides a platform-agnostic Rust driver for the AS3935 Franklin Lightning Sensor IC.

## Features

- **Platform Independent**: Built on top of [`embedded-hal`](https://crates.io/crates/embedded-hal) 1.0 traits
- **I²C and SPI Support**: Works with both communication protocols
- **`no_std` Compatible**: Can be used in embedded environments without the standard library
- **Easy to Use**: High-level API for sensor configuration and event detection

## Platform Support

This driver works with any platform that implements the `embedded-hal` 1.0 traits:
- Raspberry Pi (via [rppal](https://crates.io/crates/rppal))
- STM32 microcontrollers (via [stm32-hal](https://github.com/stm32-rs))
- ESP32 (via [esp-hal](https://github.com/esp-rs/esp-hal))
- Nordic nRF series (via [nrf-hal](https://github.com/nrf-rs/nrf-hal))
- And many more!

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
as3935-bbn = "0.1"
embedded-hal = "1.0"
```

### Example (Raspberry Pi with rppal)

```rust
use as3935_bbn::interface::i2c::I2cAddress;
use as3935_bbn::{ListeningParameters, SensorPlacing, AS3935};
use rppal::i2c::I2c;

fn main() {
    let i2c = I2c::with_bus(1).unwrap();
    let mut sensor = AS3935::new_i2c(i2c, I2cAddress::default()).unwrap();
    
    sensor.listen(
        ListeningParameters::default()
            .with_sensor_placing(SensorPlacing::Outdoor)
    ).unwrap();
    
    // Poll for events when IRQ pin goes high
    if let Ok(Some(event)) = sensor.check_irq() {
        println!("Event detected: {:?}", event);
    }
}
```

### Interrupt Handling

Since `embedded-hal` doesn't provide interrupt abstractions, you'll need to handle interrupts in your platform-specific code:

1. Monitor the IRQ pin for rising edge interrupts
2. When an interrupt occurs, call `sensor.check_irq()` to read the event
3. The sensor should be wrapped in a thread-safe container (e.g., `Arc<Mutex<AS3935>>`) if accessed from interrupt handlers

See the [examples/listen.rs](examples/listen.rs) for a complete Raspberry Pi example.

### SPI Support

```rust
use as3935_bbn::AS3935;
// Assuming you have an SPI device that implements embedded_hal::spi::SpiDevice
let mut sensor = AS3935::new_spi(spi_device).unwrap();
```

## Features

- `std` (default): Enable standard library support
- Without `std`: The crate works in `no_std` environments with `alloc`

--------------------------------------------------

The datasheet for AS3935 can be found [here](https://www.embeddedadventures.com/datasheets/AS3935_Datasheet_EN_v2.pdf)
or [here](https://cz.mouser.com/pdfdocs/AMS_AS3935_Datasheet_v4.pdf).

## License

MIT
