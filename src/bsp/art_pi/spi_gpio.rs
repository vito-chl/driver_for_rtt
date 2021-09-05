use super::DP;
use rtt_rs::print;

#[derive(Debug, Copy, Clone)]
pub enum SpiN {
    Spi1,
}

pub fn gpio_config(num: SpiN) {
    match num {
        SpiN::Spi1 => {
            gpio_1_config();
            gpio_1_cs_config();
        }
    }
}

fn gpio_1_config() {
    // 时钟的配置
    let rcc = &DP.0.RCC;
    // 使能IO引脚的时钟
    rcc.ahb1enr.modify(|_, w| w.gpioaen().set_bit());

    // 设置引脚
    let ra = &DP.0.GPIOA;
    ra.moder.modify(|_, w| {
        w.moder5().alternate();
        w.moder6().alternate();
        w.moder7().alternate();
        w
    });

    ra.otyper.modify(|_, w| {
        w.ot5().push_pull();
        w.ot6().push_pull();
        w.ot7().push_pull();
        w
    });

    ra.pupdr.modify(|_, w| {
        w.pupdr5().floating();
        w.pupdr6().floating();
        w.pupdr7().floating();
        w
    });

    ra.ospeedr.modify(|_, w| {
        w.ospeedr5().high_speed();
        w.ospeedr6().high_speed();
        w.ospeedr7().high_speed();
        w
    });

    ra.afrl.modify(|_, w| {
        w.afrl5().af5();
        w.afrl6().af5();
        w.afrl7().af5();
        w
    });
}

fn gpio_1_cs_config() {
    // 时钟的配置
    let rcc = &DP.0.RCC;
    // 使能IO引脚的时钟
    rcc.ahb1enr.modify(|_, w| w.gpioden().set_bit());

    // 设置引脚
    let rd = &DP.0.GPIOD;
    rd.moder.modify(|_, w| w.moder14().output());
    rd.otyper.modify(|_, w| w.ot14().push_pull());
    rd.pupdr.modify(|_, w| w.pupdr14().pull_down());
    rd.ospeedr.modify(|_, w| w.ospeedr14().low_speed());
    gpio_1_cs_off();
    print!("spi cs init\n");
}

pub fn gpio_1_cs_on() {
    let rd = &DP.0.GPIOD;
    rd.bsrr.write(|w| w.br14().set_bit());
    print!("spi: on\n");
}

pub fn gpio_1_cs_off() {
    let rd = &DP.0.GPIOD;
    rd.bsrr.write(|w| w.bs14().set_bit());
    print!("spi: off\n");
}
