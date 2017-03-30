/// Struct to hold calibration values programmed into the sensor
/// at the factory.  Typically loaded once upon sensor initialization
/// and then used throughout the life of the sensor reference.
#[derive(Debug)]
pub struct Calibration {
    pub t1: u16,
    pub t2: i16,
    pub t3: i16,

    pub p1: u16,
    pub p2: i16,
    pub p3: i16,
    pub p4: i16,
    pub p5: i16,
    pub p6: i16,
    pub p7: i16,
    pub p8: i16,
    pub p9: i16,

    pub h1: u16,
    pub h2: i16,
    pub h3: u16,
    pub h7: u16,
}
