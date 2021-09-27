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
    rcc.ahb4enr.modify(|_, w| {
        w.gpioaen().set_bit();
        w.gpioben().set_bit();
        w.gpiogen().set_bit();
        w
    });

    // 设置引脚 PB5 AF AF5 PP NO_PULL
    let rb = &DP.0.GPIOB;
    rb.moder.modify(|_, w| w.moder5().alternate());
    rb.otyper.modify(|_, w| w.ot5().push_pull());
    rb.pupdr.modify(|_, w| w.pupdr5().floating());
    rb.ospeedr.modify(|_, w| w.ospeedr5().high_speed());
    rb.afrl.modify(|_, w| w.afr5().af5());

    // 设置引脚 PG9 AF AF5 PP NO_PULL
    let rg = &DP.0.GPIOG;
    rg.moder.modify(|_, w| w.moder9().alternate());
    rg.otyper.modify(|_, w| w.ot9().push_pull());
    rg.pupdr.modify(|_, w| w.pupdr9().floating());
    rg.ospeedr.modify(|_, w| w.ospeedr9().high_speed());
    rg.afrh.modify(|_, w| w.afr9().af5());

    // 设置引脚 PA5 AF AF5 PP NO_PULL
    let ra = &DP.0.GPIOA;
    ra.moder.modify(|_, w| w.moder5().alternate());
    ra.otyper.modify(|_, w| w.ot5().push_pull());
    ra.pupdr.modify(|_, w| w.pupdr5().floating());
    ra.ospeedr.modify(|_, w| w.ospeedr5().high_speed());
    ra.afrl.modify(|_, w| w.afr5().af5());
}

fn gpio_1_cs_config() {
    // 时钟的配置
    let rcc = &DP.0.RCC;
    // 使能IO引脚的时钟
    rcc.ahb4enr.modify(|_, w| w.gpioaen().set_bit());

    // 设置引脚 PA4
    let rd = &DP.0.GPIOA;
    rd.moder.modify(|_, w| w.moder4().output());
    rd.otyper.modify(|_, w| w.ot4().push_pull());
    rd.pupdr.modify(|_, w| w.pupdr4().pull_down());
    rd.ospeedr.modify(|_, w| w.ospeedr4().high_speed());
    gpio_1_cs_off();
}

pub fn gpio_1_cs_on() {
    let rd = &DP.0.GPIOA;
    rd.bsrr.write(|w| w.br4().set_bit());
}

pub fn gpio_1_cs_off() {
    let rd = &DP.0.GPIOA;
    rd.bsrr.write(|w| w.bs4().set_bit());
}
