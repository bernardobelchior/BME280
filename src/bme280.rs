use std::{thread, time};
use i2cdev::core::I2CDevice;
use i2cdev::linux::LinuxI2CError;
use std::cell::RefCell;
use std::ops::DerefMut;


use super::calibration::Calibration;
use super::register::Register;

const BME280OSAMPLE1 : u8 = 1;
const BME280OSAMPLE2 : u8 = 2;
const BME280OSAMPLE4 : u8 = 3;
const BME280OSAMPLE8 : u8 = 4;
const BME280OSAMPLE16 : u8 = 5;

const MAX_OVER_SAMPLING_AND_NORMAL_MODE : u8 = 0x3F;

pub struct Bme280<T: I2CDevice<Error=LinuxI2CError> + Sized> {
    calibration: Calibration,
    device: RefCell<T>,
    mode: u8
}

impl<T: I2CDevice<Error=LinuxI2CError> + Sized> Bme280<T> {
    pub fn new(dev: T) -> Result<Bme280<T>, LinuxI2CError> {
        let mut foo = dev;
        let cal = try!(Bme280::get_calibration(&mut foo));
        try!(foo.smbus_write_byte_data(Register::CONTROL as u8, MAX_OVER_SAMPLING_AND_NORMAL_MODE));
        Ok(Bme280 { calibration: cal, device: RefCell::new(foo), mode: BME280OSAMPLE1 })
    }

    fn get_calibration(dev: &mut T) -> Result<Calibration, LinuxI2CError> {
        Ok(Calibration {
            t1: try!(dev.smbus_read_word_data(Register::T1 as u8)),
            t2: try!(dev.smbus_read_word_data(Register::T2 as u8)) as i16,
            t3: try!(dev.smbus_read_word_data(Register::T3 as u8)) as i16,

            p1: try!(dev.smbus_read_word_data(Register::P1 as u8)),
            p2: try!(dev.smbus_read_word_data(Register::P2 as u8)) as i16,
            p3: try!(dev.smbus_read_word_data(Register::P3 as u8)) as i16,
            p4: try!(dev.smbus_read_word_data(Register::P4 as u8)) as i16,
            p5: try!(dev.smbus_read_word_data(Register::P5 as u8)) as i16,
            p6: try!(dev.smbus_read_word_data(Register::P6 as u8)) as i16,
            p7: try!(dev.smbus_read_word_data(Register::P7 as u8)) as i16,
            p8: try!(dev.smbus_read_word_data(Register::P8 as u8)) as i16,
            p9: try!(dev.smbus_read_word_data(Register::P9 as u8)) as i16,

            h1: try!(dev.smbus_read_word_data(Register::T1 as u8)),
            h2: try!(dev.smbus_read_word_data(Register::T1 as u8)) as i16,
            h3: try!(dev.smbus_read_word_data(Register::T1 as u8)),
            h7: try!(dev.smbus_read_word_data(Register::T1 as u8))
        })
    }


    fn read_raw_temp(&self) -> Result<f64, LinuxI2CError> { 
        let mut a = self.device.borrow_mut();
        let dev = a.deref_mut();
        
        
        try!(dev.smbus_write_byte_data(Register::CONTROL_HUM as u8, self.mode));
        let meas = self.mode << 5 | self.mode << 2 | 1;
        try!(dev.smbus_write_byte_data(Register::CONTROL as u8, meas));
        let mut sleep_time = 0.00125 + 0.0023 * (1 << self.mode) as f32;
        sleep_time = sleep_time + 0.0023 * (1 << self.mode) as f32 + 0.000575;
        sleep_time = sleep_time + 0.0023 * (1 << self.mode) as f32 + 0.000575;
        let dur = time::Duration::from_millis((sleep_time * 1000.0) as u64);
        thread::sleep(dur);
        
        let msb = try!(dev.smbus_read_byte_data(Register::TEMP_DATA as u8)) as u32;
        let lsb = try!(dev.smbus_read_byte_data(Register::TEMP_DATA_1 as u8)) as u32;
        let xlsb = try!(dev.smbus_read_byte_data(Register::TEMP_DATA_2 as u8)) as u32;

        let raw = ((msb << 16) | (lsb << 8) | xlsb) >> 4;
        println!("raw temp: {}", raw as f64);
        Ok(raw as f64)
    }

    fn calc_t_fine(&self) -> Result<f64, LinuxI2CError> {
        let ut = try!(self.read_raw_temp());
        let t1 = self.calibration.t1 as f64;
        let t2 = self.calibration.t2 as f64;
        let t3 = self.calibration.t3 as f64;
        let var1 = (ut / 16384.0 - t1 / 1024.0) * t2;
        let var2 = ((ut / 131072.0 - t1 / 8192.0) * (ut / 131072.0 - t1 / 8192.0)) * t3;
        let t_fine = var1 + var2;
        println!("t_fine: {}", t_fine);
        Ok(t_fine)
    }

    pub fn read_temperature(&self) -> Result<f64, LinuxI2CError> {
        // Technically I'm skipping the step of casting to an integer, which would
        // result in rounding down of the var1 and var2 that were used in the original
        // calculation of t_fine:
        let celcius = try!(self.calc_t_fine()) / 5120.0;
        let fahrenheit = celcius * 1.8 + 32.0;
        Ok(fahrenheit)
    }

    fn read_raw_pressure(&self) -> Result<u32, LinuxI2CError> {
        let mut refmut = self.device.borrow_mut();
        let dev = refmut.deref_mut();
        
        let msb = try!(dev.smbus_read_byte_data(Register::PRESSURE_DATA as u8)) as u32;
        let lsb = try!(dev.smbus_read_byte_data(Register::PRESSURE_DATA_1 as u8)) as u32;
        let xlsb = try!(dev.smbus_read_byte_data(Register::PRESSURE_DATA_2 as u8)) as u32;
        let raw = ((msb << 16) | (lsb << 8) | xlsb) >> 4;        
        println!("raw pressure: {}", raw);
        Ok(raw)
    }

    pub fn read_pressure(&self) -> Result<f64, LinuxI2CError> {
        let p1 = self.calibration.p1 as f64;
        let p2 = self.calibration.p2 as f64;
        let p3 = self.calibration.p3 as f64;
        let p4 = self.calibration.p4 as f64;
        let p5 = self.calibration.p5 as f64;
        let p6 = self.calibration.p6 as f64;
        let p7 = self.calibration.p7 as f64;
        let p8 = self.calibration.p8 as f64;
        let p9 = self.calibration.p9 as f64;

        let adc = try!(self.read_raw_pressure()) as f64;
        let t_fine = try!(self.calc_t_fine());
        let var1 = t_fine / 2.0 - 64000.0;
        let var2 = var1 * var1 * p6 / 32768.0;
        let var2_2 = var2 + var1 * p5 * 2.0;
        let var2_3 = var2_2 / 4.0 + p4 * 65536.0;
        let var1_2 = (p3 * var1 * var1 / 524288.0 + p2 * var1) / 524288.0;
        let var1_3 = (1.0 + var1_2 / 32768.0) * p1;
        
        if var1_3 == 0.0 {
            return Ok(0.0);
        }

        let p = 1048576.0 - adc;
        let p_2 = ((p - var2_3 / 4096.0) * 6250.0) / var1_3;
        let var1_4 = p9 * p_2 * p_2 / 2147483648.0;
        let var2_4 = p_2 * p8 / 32768.0;
        let pascals = p_2 + (var1_4 + var2_4 + p7) / 16.0;        
        let in_hg = pascals *  0.000295299830714;
        Ok(in_hg)
    }    
}

#[cfg(test)]
mod tests {

    use i2cdev::core::I2CDevice;
    use i2cdev::linux::{LinuxI2CError};
    use std::io::{Error, ErrorKind};
    use nix;
    use bme280::{Bme280};
    use super::super::register::Register;

    struct FakeDevice {

    }

    impl I2CDevice for FakeDevice {
        type Error = LinuxI2CError;

        fn read(&mut self, data: &mut [u8]) -> Result<(), Self::Error> {
            Ok(())
        }

        fn write(&mut self, data: &[u8]) -> Result<(), Self::Error> {
            Ok(())
        }

        fn smbus_write_quick(&mut self, bit: bool) -> Result<(), Self::Error> {
            Ok(())
        }

        fn smbus_read_block_data(&mut self, register: u8) -> Result<Vec<u8>, Self::Error> {
            Ok(vec![1,2,3])
        }

        fn smbus_read_i2c_block_data(&mut self, register: u8, len: u8) -> Result<Vec<u8>, Self::Error> {
            Ok(vec![1,2,3])
        }

        fn smbus_write_block_data(&mut self, register: u8, values: &[u8]) -> Result<(), Self::Error> {
            Ok(())
        }

        fn smbus_process_block(&mut self, register: u8, values: &[u8]) -> Result<(), Self::Error> {
            Ok(())
        }
    
        fn smbus_read_word_data(&mut self, register: u8) -> Result<u16, LinuxI2CError> {
            match register {
                register if register == Register::T1 as u8 => Ok(28960),
                register if register == Register::T2 as u8 => Ok(26619),
                register if register == Register::T3 as u8 => Ok(26619),

                register if register == Register::P1 as u8 => Ok(34988),
                register if register == Register::P2 as u8 => Ok(54823),
                register if register == Register::P3 as u8 => Ok(3024),
                register if register == Register::P4 as u8 => Ok(5831),
                register if register == Register::P5 as u8 => Ok(96),
                register if register == Register::P6 as u8 => Ok(65529),
                register if register == Register::P7 as u8 => Ok(9900),
                register if register == Register::P8 as u8 => Ok(55306),
                register if register == Register::P9 as u8 => Ok(4285),

                register if register == Register::H1 as u8 => Ok(28960),
                register if register == Register::H2 as u8 => Ok(28960),
                register if register == Register::H3 as u8 => Ok(28960),
                register if register == Register::H7 as u8 => Ok(28960),
                _ => Err(LinuxI2CError::Nix(nix::Error::InvalidPath))
            }
        }

        fn smbus_read_byte_data(&mut self, register: u8) -> Result<u8, Self::Error> {
             match register {
                register if register == Register::TEMP_DATA as u8 => Ok(129),
                register if register == Register::TEMP_DATA_1 as u8 => Ok(142),
                register if register == Register::TEMP_DATA_2 as u8 => Ok(0),
                register if register == Register::PRESSURE_DATA as u8 => Ok(92),
                register if register == Register::PRESSURE_DATA_1 as u8 => Ok(215),
                register if register == Register::PRESSURE_DATA_2 as u8 => Ok(112),
                _ => Err(LinuxI2CError::Nix(nix::Error::InvalidPath))
            }
        }
    }

    #[test]
    fn set_of_known_calibration_values_should_yield_known_temperature() {        
        let bme = Bme280::new(FakeDevice {}).unwrap();

        let t = bme.read_temperature().unwrap();
        println!("Temperature is: {}", t);
        assert!((t - 72.91).abs() < 0.01);
    }

    #[test]
    fn set_of_known_calibration_values_should_yield_known_pressure() {
        let bme = Bme280::new(FakeDevice {}).unwrap();

        let p = bme.read_pressure().unwrap();
        println!("Pressure is: {} inhg.", p);
        assert!((p - 30.20).abs() < 0.01);
    }
}


