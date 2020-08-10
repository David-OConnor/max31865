//! A generic driver for the MAX31865 RTD to Digital converter
//!
//! # References
//! - Datasheet: https://datasheets.maximintegrated.com/en/ds/MAX31865.pdf

#![feature(unsize)]
#![no_std]

use embedded_hal as hal;

use hal::blocking::spi;
use hal::digital::v2::{InputPin, OutputPin};
use hal::spi::{Mode, Phase, Polarity};

use core::marker::Unsize;
use core::mem;

#[cfg(feature = "doc")]
pub use examples;

pub const MODE: Mode = Mode {
    phase: Phase::CaptureOnSecondTransition,
    polarity: Polarity::IdleHigh,
};

pub mod temp_conversion;

pub enum FilterMode {
    Filter60Hz = 0,
    Filter50Hz = 1,
}

pub enum SensorType {
    TwoOrFourWire = 0,
    ThreeWire = 1,
}

pub struct Max31865<NCS, RDY> {
    // spi: SPI,
    ncs: NCS,
    rdy: RDY,
    calibration: u32,
}

impl<NCS, RDY> Max31865<NCS, RDY>
where
    // SPI: spi::Write<u8, Error = E> + spi::Transfer<u8, Error = E>,
    NCS: OutputPin,
    RDY: InputPin,
{
    /// Create a new MAX31865 module.
    ///
    /// # Arguments
    ///
    /// * `spi` - The SPI module to communicate on.
    /// * `ncs` - The chip select pin which should be set to a push pull output pin.
    /// * `rdy` - The ready pin which is set low by the MAX31865 controller whenever
    ///             it has finished converting the output.
    ///
    pub fn new<E>(
        // spi: SPI,
        mut ncs: NCS,
        rdy: RDY,
    ) -> Result<Max31865<NCS, RDY>, E> {
        let default_calib = 40000;

        ncs.set_high().ok();
        let max31865 = Max31865 {
            // spi,
            ncs,
            rdy,
            calibration: default_calib, /* value in ohms multiplied by 100 */
        };

        Ok(max31865)
    }

    /// Updates the devices configuration.
    ///
    /// # Arguments
    /// * `vbias` - Set to `true` to enable V_BIAS voltage, which is required to correctly perform conversion.Clone
    /// * `conversion_mode` - `true` to automatically perform conversion, otherwise normally off.
    /// * `one_shot` - Only perform detection once if set to `true`, otherwise repeats conversion.
    /// * `sensor_type` - Define whether a two, three or four wire sensor is used.Clone
    /// * `filter_mode` - Specify the mains frequency that should be used to filter out noise, e.g. 50Hz in Europe.
    ///
    /// # Remarks
    ///
    /// This will update the configuration register of the MAX31865 register. If the device doesn't properly react
    /// to this, add a delay after calling `new` to increase the time that the chip select line is set high.
    ///
    /// *Note*: The correct sensor configuration also requires changes to the PCB! Make sure to read the datasheet
    /// concerning this.
    pub fn configure<SPI, E>(
        &mut self,
        spi: &mut SPI,
        vbias: bool,
        conversion_mode: bool,
        one_shot: bool,
        sensor_type: SensorType,
        filter_mode: FilterMode,
    ) -> Result<(), E>
    where
        SPI: spi::Write<u8, Error = E> + spi::Transfer<u8, Error = E>,
    {
        let conf: u8 = ((vbias as u8) << 7)
            | ((conversion_mode as u8) << 6)
            | ((one_shot as u8) << 5)
            | ((sensor_type as u8) << 4)
            | (filter_mode as u8);

        self.write(spi, Register::CONFIG, conf)?;

        Ok(())
    }

    /// Set the calibration reference resistance.
    /// This can be used to calibrate inaccuracies of both the reference resistor
    /// and the PT100 element.
    ///
    /// # Arguments
    ///
    /// * `calib` - A 32 bit integer specifying the reference resistance in ohms
    ///             multiplied by 100, e.g. `40000` for 400 Ohms
    ///
    /// # Remarks
    ///
    /// You can perform calibration by putting the sensor in boiling (100 degrees
    /// Celcius) water and then measuring the raw value using `read_raw`. Calculate
    /// `calib` as `(13851 << 15) / raw >> 1`.
    pub fn set_calibration<E>(&mut self, calib: u32) -> Result<(), E> {
        self.calibration = calib;
        Ok(())
    }

    /// Read the raw resistance value and then perform conversion to degrees Celcius.
    ///
    /// # Remarks
    ///
    /// The output value is the value in degrees Celcius multiplied by 100.
    pub fn read_default_conversion<SPI, E>(&mut self, spi: &mut SPI) -> Result<u32, E>
    where
        SPI: spi::Write<u8, Error = E> + spi::Transfer<u8, Error = E>,
    {
        let raw = self.read_raw(spi)?;
        let ohms = ((raw >> 1) as u32 * self.calibration) >> 15;
        let temp = temp_conversion::lookup_temperature(ohms as u16);

        Ok(temp)
    }

    /// Read the raw RTD value.
    ///
    /// # Remarks
    ///
    /// The raw value is the value of the combined MSB and LSB registers.
    /// The first 15 bits specify the ohmic value in relation to the reference
    /// resistor (i.e. 2^15 - 1 would be the exact same resistance as the reference
    /// resistor). See manual for further information.
    /// The last bit specifies if the conversion was successful.
    pub fn read_raw<SPI, E>(&mut self, spi: &mut SPI) -> Result<u16, E>
    where
        SPI: spi::Write<u8, Error = E> + spi::Transfer<u8, Error = E>,
    {
        let msb: u16 = self.read(spi, Register::RTD_MSB)? as u16;
        let lsb: u16 = self.read(spi, Register::RTD_LSB)? as u16;

        Ok((msb << 8) | lsb)
    }

    /// Determine if a new conversion is available
    ///
    /// # Remarks
    ///
    /// When the module is finished converting the temperature it sets the
    /// ready pin to low. It is automatically returned to high upon reading the
    /// RTD registers.
    // pub fn is_ready<E>(&self) -> Result<bool, E> {
    //     self.rdy.is_low()
    // }
    pub fn is_ready<E>(&self) -> bool {
        self.rdy.is_low().unwrap_or(false)
    }

    fn read<SPI, E>(&mut self, spi: &mut SPI, reg: Register) -> Result<u8, E>
    where
        SPI: spi::Write<u8, Error = E> + spi::Transfer<u8, Error = E>,
    {
        let buffer: [u8; 2] = self.read_many(spi, reg)?;
        Ok(buffer[1])
    }

    fn read_many<B, SPI, E>(&mut self, spi: &mut SPI, reg: Register) -> Result<B, E>
    where
        B: Unsize<[u8]>,
        SPI: spi::Write<u8, Error = E> + spi::Transfer<u8, Error = E>,
    {
        let mut buffer: B = unsafe { mem::zeroed() };
        {
            let slice: &mut [u8] = &mut buffer;
            slice[0] = reg.read_address();
            self.ncs.set_low().ok();
            spi.transfer(slice)?;
            self.ncs.set_high().ok();
        }

        Ok(buffer)
    }

    fn write<SPI, E>(&mut self, spi: &mut SPI, reg: Register, val: u8) -> Result<(), E>
    where
        SPI: spi::Write<u8, Error = E> + spi::Transfer<u8, Error = E>,
    {
        self.ncs.set_low().ok();
        spi.write(&[reg.write_address(), val])?;
        self.ncs.set_high().ok();
        Ok(())
    }
}

#[allow(non_camel_case_types)]
#[allow(dead_code)]
#[derive(Clone, Copy)]
enum Register {
    CONFIG = 0x00,
    RTD_MSB = 0x01,
    RTD_LSB = 0x02,
    HIGH_FAULT_THRESHOLD_MSB = 0x03,
    HIGH_FAULT_THRESHOLD_LSB = 0x04,
    LOW_FAULT_THRESHOLD_MSB = 0x05,
    LOW_FAULT_THRESHOLD_LSB = 0x06,
    FAULT_STATUS = 0x07,
}

const R: u8 = 0 << 7;
const W: u8 = 1 << 7;

impl Register {
    fn read_address(&self) -> u8 {
        *self as u8 | R
    }

    fn write_address(&self) -> u8 {
        *self as u8 | W
    }
}
