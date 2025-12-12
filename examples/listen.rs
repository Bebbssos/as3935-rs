use as3935_bbn::interface::i2c::I2cAddress;
use as3935_bbn::{
    Event, HeadOfStormDistance, ListeningParameters, SensorPlacing,
    SignalVerificationThreshold, AS3935,
};
use chrono::Utc;
use rppal::gpio::{Gpio, Trigger};
use rppal::i2c::I2c;
use simple_signal::{set_handler, Signal};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn main() {
    simple_logger::init().unwrap();

    println!("Initializing…");

    let gpio = Gpio::new().unwrap();
    let i2c = I2c::with_bus(1).unwrap();

    let mut as3935 = AS3935::new_i2c(i2c, I2cAddress::default()).unwrap();

    println!("Starting to listen…");

    as3935
        .listen(
            ListeningParameters::default()
                .with_sensor_placing(SensorPlacing::Outdoor)
                .with_signal_verification_threshold(SignalVerificationThreshold::new(5).unwrap()),
        )
        .unwrap();

    println!("Listening for events…");

    // Set up IRQ pin for interrupt handling
    let mut irq_pin = gpio.get(24).unwrap().into_input();
    
    // Share AS3935 between threads using Arc<Mutex>
    let as3935_shared = Arc::new(Mutex::new(as3935));
    let as3935_clone = as3935_shared.clone();

    // Spawn thread to handle IRQ events
    let (event_tx, event_rx) = channel();
    
    irq_pin
        .set_async_interrupt(
            Trigger::RisingEdge,
            None, // No reset time
            move |_event| {
                // Wait for IRQ to be ready
                thread::sleep(Duration::from_millis(2));
                
                let mut sensor = as3935_clone.lock().unwrap();
                if let Ok(Some(event)) = sensor.check_irq() {
                    event_tx.send(event).unwrap();
                }
            },
        )
        .unwrap();

    // Spawn thread to print events
    thread::spawn(move || {
        for event in event_rx {
            println!(
                "[{}] {}",
                Utc::now().to_rfc3339(),
                match event {
                    Event::Lightning(lightning) => format!(
                        "Lightning detected: {}.",
                        match lightning {
                            HeadOfStormDistance::Kilometers(km) => format!("{} km", km),
                            HeadOfStormDistance::OutOfRange => String::from("out of range"),
                            HeadOfStormDistance::Overhead => String::from("overhead"),
                        }
                    ),
                    Event::Noise => String::from("Noise detected."),
                    Event::Disturbance => String::from("Disturber detected."),
                }
            )
        }
    });

    let (tx, rx) = channel::<()>();

    set_handler(&[Signal::Term, Signal::Int], move |_signals| {
        tx.send(()).unwrap();
    });

    rx.recv().unwrap();

    println!("Terminating…");
    irq_pin.clear_async_interrupt().unwrap();
    as3935_shared.lock().unwrap().terminate().unwrap();

    println!("Terminated.");
}

