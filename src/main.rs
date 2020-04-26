// converted this post
// Programming GPIO Interrupts with Embedded Rust
// https://flowdsp.io/blog/stm32f3-01-interrupts/
// to RTFM
//
// Programs STM32F303 Discovery board user button to toggle led
// using interrupt and wfi (wait for interrupt) in RTFM idle loop.

#![deny(warnings)]
#![no_main]
#![no_std]

extern crate panic_itm;

use cortex_m::{iprintln, peripheral::ITM};
use stm32f3::stm32f303;
use stm32f3::stm32f303::Interrupt;

#[rtfm::app(device = stm32f303)]
const APP: () = {
    struct Resources {
        itm: ITM,
        gpioa: stm32f303::GPIOA,
        gpioe: stm32f303::GPIOE,
        exti: stm32f303::EXTI,
    }
    #[init]
    fn init(_: init::Context) -> init::LateResources {
        // 1. get peripherals
        let cortexm_peripherals = cortex_m::Peripherals::take().unwrap();
        let mut itm = cortexm_peripherals.ITM;
        let stm32f3_peripherals = stm32f303::Peripherals::take().unwrap();
        iprintln!(&mut itm.stim[0], "\n\nrtfm-02");

        // 2. enable GPIOA and SYSCFG clocks
        let rcc = &stm32f3_peripherals.RCC;
        rcc.ahbenr
            .modify(|_, w| w.iopaen().set_bit().iopeen().set_bit());
        rcc.apb2enr.modify(|_, w| w.syscfgen().set_bit());

        // 3. Configure PA0 pin as input, pull-down
        let gpioa = &stm32f3_peripherals.GPIOA;
        gpioa.moder.modify(|_, w| w.moder0().input());
        gpioa.pupdr.modify(|_, w| w.pupdr0().pull_down());

        // configure PE8, PE9 as output
        let gpioe = &stm32f3_peripherals.GPIOE;
        gpioe.moder.modify(|_, w| {
            w.moder8()
                .output() // LED: LED4 User on pin PE8  (blue)
                .moder9()
                .output() // LED: LED3 User on pin PE9  (red)
        });

        // 4. connect EXTI0 line to PA0 pin
        let syscfg = &stm32f3_peripherals.SYSCFG;
        syscfg
            .exticr1
            .modify(|_, w| unsafe { w.exti0().bits(0b000) }); // w.exti0().pa0()

        // 5. Configure EXTI0 line (external interrupts) mode=interrupt and trigger=rising-edge
        let exti = &stm32f3_peripherals.EXTI;
        exti.imr1.modify(|_, w| w.mr0().set_bit()); // unmask interrupt
        exti.rtsr1.modify(|_, w| w.tr0().set_bit()); // trigger=rising-edge

        // 6.
        let gpioa = stm32f3_peripherals.GPIOA;
        let gpioe = stm32f3_peripherals.GPIOE;
        let exti = stm32f3_peripherals.EXTI;

        // 7. Enable EXTI0 Interrupt
        unsafe {
            cortex_m::peripheral::NVIC::unmask(stm32f303::Interrupt::EXTI0);
        }

        // 8. Enable low power debugging.
        // This fixes the problem of debugger
        // not connecting when using wfi in the idle loop.
        // Otherwise you have to press the reset button
        // and restart openocd every run.

        let dbgmcu = stm32f3_peripherals.DBGMCU;
        dbgmcu.cr.modify(|_, w| w.dbg_sleep().set_bit());
        dbgmcu.cr.modify(|_, w| w.dbg_stop().set_bit());
        dbgmcu.cr.modify(|_, w| w.dbg_standby().set_bit());

        init::LateResources {
            itm,
            gpioa,
            gpioe,
            exti,
        }
    }

    #[idle(resources = [itm])]
    fn idle(mut _cx: idle::Context) -> ! {
        _cx.resources.itm.lock(|itm| {
            iprintln!(&mut itm.stim[0], "idle");
        });

        rtfm::pend(Interrupt::SPI1);

        // hprintln!("idle 2").unwrap();
        _cx.resources.itm.lock(|itm| {
            iprintln!(&mut itm.stim[0], "idle 2");
        });

        loop {
            cortex_m::asm::wfi();
            rtfm::pend(Interrupt::SPI1);
        }
    }

    #[task(binds = SPI1, resources = [itm])]
    fn SPI1(_cx: SPI1::Context) {
        static mut TIMES: u32 = 0;

        // Safe access to local `static mut` variable
        *TIMES += 1;

        iprintln!(
            &mut _cx.resources.itm.stim[0],
            "SPI1 called {} time{}",
            *TIMES,
            if *TIMES > 1 { "s" } else { "" }
        );
    }

    #[task(binds = EXTI0, resources = [gpioe,exti])]
    fn EXTI0(_cx: EXTI0::Context) {
        // clear the EXTI line 0 pending bit
        _cx.resources.exti.pr1.modify(|_, w| w.pr0().set_bit());
        // toggle LED4
        _cx.resources.gpioe.odr.modify(|r, w| {
            let led4 = r.odr8().bit();
            if led4 {
                w.odr8().clear_bit()
            } else {
                w.odr8().set_bit()
            }
        });
    }
};
