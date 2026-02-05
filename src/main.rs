#![no_std]
#![no_main]

use defmt::info;

use embassy_executor::Spawner;
use embassy_rp::{
    gpio::{Level, Output},
    uart::{Blocking, Config as UartConfig, InterruptHandler as UARTInterruptHandler, UartTx},
    peripherals::UART1,
    i2c::{InterruptHandler as I2CInterruptHandler, Config as I2CConfig},
    i2c,
    bind_interrupts
};
use embassy_time::Timer;

use static_cell::StaticCell;
use ina219::{
    address::Address,
    calibration::{IntCalibration, MicroAmpere},
    SyncIna219
};

#[cfg(feature = "round")]
use num_traits::float::FloatCore;

use {defmt_serial as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    UART1_IRQ => UARTInterruptHandler<UART1>;
    I2C1_IRQ => I2CInterruptHandler<embassy_rp::peripherals::I2C1>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    // Initial serial debugging.
    let uart = UartTx::new(p.UART1, p.PIN_4, p.DMA_CH4, UartConfig::default(),);
    static SERIAL: StaticCell<UartTx<'static, Blocking>> = StaticCell::new();
    defmt_serial::defmt_serial(SERIAL.init(uart));

    info!("Start");

    // Turn on the built-in LED.
    let _builtin_led = Output::new(p.PIN_25, Level::High);

    // Initialize I2C.
    let i2c = i2c::I2c::new_async(p.I2C1, p.PIN_7, p.PIN_6, Irqs, I2CConfig::default());

    // Resolution of 1A, and a shunt of 10mΩ.
    // The shunt resistor in the Pico UPS Hat B: R1/0.01Ω (10,000µΩ/10mΩ).
    //let calib = IntCalibration::new(MicroAmpere(1_000_000), 10_000).unwrap();
    let calib = IntCalibration::new(MicroAmpere(100), 10_000).unwrap();
    let mut ina = SyncIna219::new_calibrated(i2c, Address::from_byte(0x43).unwrap(), calib).unwrap();

    loop {
        let measurement = ina.next_measurement().unwrap().expect("A measurement is ready");

        // Calculate how much charge is left.
        let mut charge: f32 = ((measurement.bus_voltage.voltage_mv() - 3) as f32) / 1.2 * 100.0;
        if charge < 0.0 {
            charge = 0.0;
        } else {
            charge = 100.0;
        }

        info!("Power:           {}", measurement.power);
        info!("Current:         {}", measurement.current);
        info!("Charge:          {}%", charge);

        info!("Voltage (Bus):   {=f32:#02}V",
              measurement.bus_voltage.voltage_mv() as f32 / 1000.0
        );

        let shunt_voltage_mv = measurement.shunt_voltage.shunt_voltage_mv();
        let shunt_voltage_uv = measurement.shunt_voltage.shunt_voltage_uv();
        info!("Voltage (Shunt): {=f32:#02}mV ({=f32:#02}µV)",
              shunt_voltage_mv as f32,
              shunt_voltage_uv as f32,
        );

        // Shunt:  -50µV <  -10µV =>    Main power; Not charging
        // Shunt: -420µV < -380µV => No main power; Not charging (on battery)
        // Shunt:        >  900µV =>    Main power;     Charging
        if (shunt_voltage_uv as i16) < -350 {
            info!("=> No main power/Not charging (on battery)");
        } else if (shunt_voltage_uv as i16) < -5 {
            info!("=> Main power/Battery/Charging");
        } else if shunt_voltage_uv as u16 > 850 {
            info!("=> Main power/No Battery/Charging");
        }

        info!(".");
        Timer::after_secs(5).await;
    }
}

#[cfg(feature = "round")]
fn round_to_three_places(val: f32) -> f32 {
    val.round() / 1000.0
}
