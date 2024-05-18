use sdl2;
use std::time;

use crate::{bootrom::Bootrom, cartridge::Cartridge, cpu::Cpu, lcd::LCD, peripherals::Peripherals};

pub const CPU_CLOCK_HZ: u128 = 4_194_304;
pub const M_CYCLE_CLOCK: u128 = 4;
const M_CYCLE_NANOS: u128 = M_CYCLE_CLOCK * 1_000_000_000 / CPU_CLOCK_HZ;

pub struct GameBoy {
  pub cpu: Cpu,
  pub peripherals: Peripherals,
  pub lcd: LCD,
}

impl GameBoy {
  pub fn new(bootrom: Bootrom, cartridge: Cartridge) -> Self {
    let sdl = sdl2::init().expect("failed to initialize SDL");
    let lcd = LCD::new(&sdl, 4);
    let peripherals = Peripherals::new(bootrom, cartridge);
    let cpu = Cpu::new();
    Self {
      cpu,
      peripherals,
      lcd,
    }
  }

  pub fn run(&mut self) {
    let time = time::Instant::now();
    let mut elapsed = 0;
    loop {
      let e = time.elapsed().as_nanos();
      for _ in 0..(e - elapsed) / M_CYCLE_NANOS {
        self.cpu.emulate_cycle(&mut self.peripherals);
        self
          .peripherals
          .timer
          .emulate_cycle(&mut self.cpu.interrupts);
        if let Some(addr) = self.peripherals.ppu.oam_dma {
          self
            .peripherals
            .ppu
            .oam_dma_emulate_cycle(self.peripherals.read(&self.cpu.interrupts, addr));
        }
        if self.peripherals.ppu.emulate_cycle(&mut self.cpu.interrupts) {
          self.lcd.draw(self.peripherals.ppu.pixel_buffer());
        }
        elapsed += M_CYCLE_NANOS;
      }
    }
  }
}
