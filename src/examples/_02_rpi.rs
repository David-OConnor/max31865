//! Raspberry Pi demo
//!
//! # Connections
//!
//! IMPORTANT: Do *not* use PIN24 / BCM8 / CE0 as the NCS pin
//!
//! - PIN1 = 3V3 = VCC
//! - PIN19 = BCM10 = MOSI (SDA)
//! - PIN21 = BCM9 = MISO (AD0)
//! - PIN23 = BCM11 = SCLK (SCL)
//! - PIN22 = BCM25 = NCS
//! - PIN15 = BCM22 = RDY
//! - PIN6 = GND = GND
//! 
//! for further reference check https://pinout.xyz/#
//!
//! ```
//! 
//! extern crate linux_embedded_hal as lin_hal;
//! extern crate embedded_hal as hal;
//! extern crate max31865;
//! 
//! use max31865::{Max31865, FilterMode, SensorType};
//! use lin_hal::spidev::{self, SpidevOptions};
//! use lin_hal::{Pin, Spidev};
//! use lin_hal::sysfs_gpio::Direction;
//! use hal::digital::{InputPin, OutputPin};
//! 
//! struct HackInputPin<'a> {
//!     pin: &'a OutputPin
//! }
//! 
//! impl<'a> HackInputPin<'a> {
//!     fn new(p : &'a OutputPin) -> HackInputPin {
//!         HackInputPin {
//!             pin: p
//!         }
//!     }
//! }
//! 
//! impl<'a> InputPin for HackInputPin<'a> {
//!     fn is_low(&self) -> bool {
//!         self.pin.is_low()
//!     }
//! 
//!     fn is_high(&self) -> bool {
//!         self.pin.is_high()
//!     }
//! }
//! 
//! fn main() {
//!     let mut spi = Spidev::open("/dev/spidev0.0").unwrap();
//!     let options = SpidevOptions::new()
//!         .max_speed_hz(1_000_000)
//!         .mode(spidev::SPI_MODE_3)
//!         .build();
//!     spi.configure(&options).unwrap();
//! 
//!     let ncs = Pin::new(25);
//!     ncs.export().unwrap();
//!     while !ncs.is_exported() {}
//!     ncs.set_direction(Direction::Out).unwrap();
//!     ncs.set_value(1).unwrap();
//! 
//!     let rdy = Pin::new(22);
//!     rdy.export().unwrap();
//!     while !rdy.is_exported() {}
//!     rdy.set_direction(Direction::In).unwrap();
//!     rdy.set_value(1).unwrap();
//!     let rdyi = HackInputPin::new(&rdy);
//! 
//!     let mut max31865 = Max31865::new(spi, ncs, rdyi).unwrap();
//! 
//!     // setup the sensor so it repeatedly performs conversion and 
//!     // informs us over the ready pin
//!     max31865.configure(true, true, false, SensorType::ThreeWire, FilterMode::Filter50Hz).unwrap();
//! 
//!     loop {
//!         // if the sensor is ready, read the value and print it otherwise do nothing
//!         // one may not want to loop like this
//!         if max31865.is_ready().unwrap() {
//!             let temp = max31865.read_default_conversion().unwrap();
//! 
//!             println!("The temperature is {}", (temp as f64)/100.);
//!         }
//!     }
//! }
//! ```
// Auto-generated. Do not modify.