extern crate i2cdev;
extern crate bme280;

use std::error::Error;
use i2cdev::core::I2CDevice;
use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError};
use bme280::bme280::Bme280;
use bme280::register::Register;

#[test]
#[ignore]
fn read_some_sensor_values() {
    let i2c_addr = 0x77;
    let bus_num = 2;
    let bme = Bme280::<LinuxI2CDevice>::new(i2c_addr, bus_num).unwrap();

    println!("Temperature is {} degrees Fahrenheit.", bme.read_temperature().unwrap());
    println!("Barometric pressure is {} inhg.", bme.read_pressure().unwrap());
    println!("Relative Humidity is {}%.", bme.read_humidity().unwrap());
}