#![no_std]
#![no_main]

use defmt::info;

use embassy_executor::Spawner;
use embassy_rp::i2c::InterruptHandler;
use embassy_time::Timer;
use pico_ups_hat_b::UpsHat;

use {defmt_rtt as _, panic_probe as _};

embassy_rp::bind_interrupts!(struct Irqs {
    I2C1_IRQ => InterruptHandler<embassy_rp::peripherals::I2C1>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Start");

    let p = embassy_rp::init(Default::default());
    let mut i2c_bus = UpsHat::new(p.I2C1, Irqs, p.PIN_6, p.PIN_7, 0x43);

    i2c_bus.parse_config().await;
    i2c_bus.set_power_save(true).await;
    i2c_bus.reset_device().await;

    loop {
        let shunt = i2c_bus.get_shunt_voltage().await;
        let bus = i2c_bus.get_bus_voltage().await;
        let power = i2c_bus.get_power().await;
        let current = i2c_bus.get_current().await;
        let charge = i2c_bus.get_charge().await;

        info!("Shunt Voltage: {}mV", shunt as u16);
        info!("Bus Voltage:   {}V", bus);
        info!("Power:         {}V", power);
        info!("Current:       {}V", current);
        info!("Charge:        {}%", charge);

        Timer::after_secs(10).await;
    }
}
