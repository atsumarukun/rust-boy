use crate::{
  apu::Apu, audio::Audio, bootrom::Bootrom, cartridge::Cartridge, cpu::interrupts::Interrupts,
  hram::HRam, joypad::Joypad, ppu::Ppu, timer::Timer, wram::WRam,
};

pub struct Peripherals {
  bootrom: Bootrom,
  pub cartridge: Cartridge,
  pub ppu: Ppu,
  pub apu: Apu,
  pub timer: Timer,
  pub joypad: Joypad,
  wram: WRam,
  hram: HRam,
}

impl Peripherals {
  pub fn new(bootrom: Bootrom, cartridge: Cartridge, audio: Audio) -> Self {
    Self {
      bootrom,
      cartridge,
      ppu: Ppu::new(),
      apu: Apu::new(audio),
      timer: Timer::default(),
      joypad: Joypad::new(),
      wram: WRam::new(),
      hram: HRam::new(),
    }
  }

  pub fn read(&self, interrupts: &Interrupts, addr: u16) -> u8 {
    match addr {
      0x0000..=0x00FF => {
        if self.bootrom.is_active() {
          self.bootrom.read(addr)
        } else {
          self.cartridge.read(addr)
        }
      }
      0x0100..=0x7FFF => self.cartridge.read(addr),
      0x8000..=0x9FFF => self.ppu.read(addr),
      0xA000..=0xBFFF => self.cartridge.read(addr),
      0xC000..=0xFDFF => self.wram.read(addr),
      0xFE00..=0xFE9F => self.ppu.read(addr),
      0xFF00 => self.joypad.read(),
      0xFF04..=0xFF07 => self.timer.read(addr),
      0xFF0F => interrupts.read(addr),
      0xFF10..=0xFF26 | 0xFF30..=0xFF3F => self.apu.read(addr),
      0xFF40..=0xFF4B => self.ppu.read(addr),
      0xFF80..=0xFFFE => self.hram.read(addr),
      0xFFFF => interrupts.read(addr),
      _ => 0xFF,
    }
  }

  pub fn write(&mut self, interrupts: &mut Interrupts, addr: u16, val: u8) {
    match addr {
      0x0000..=0x00FF => {
        if !self.bootrom.is_active() {
          self.cartridge.write(addr, val)
        }
      }
      0x0100..=0x7FFF => self.cartridge.write(addr, val),
      0x8000..=0x9FFF => self.ppu.write(addr, val),
      0xA000..=0xBFFF => self.cartridge.write(addr, val),
      0xC000..=0xFDFF => self.wram.write(addr, val),
      0xFE00..=0xFE9F => self.ppu.write(addr, val),
      0xFF00 => self.joypad.write(addr, val),
      0xFF04..=0xFF07 => self.timer.write(addr, val),
      0xFF0F => interrupts.write(addr, val),
      0xFF10..=0xFF26 | 0xFF30..=0xFF3F => self.apu.write(addr, val),
      0xFF40..=0xFF4B => self.ppu.write(addr, val),
      0xFF50 => self.bootrom.write(addr, val),
      0xFF80..=0xFFFE => self.hram.write(addr, val),
      0xFFFF => interrupts.write(addr, val),
      _ => (),
    }
  }
}
