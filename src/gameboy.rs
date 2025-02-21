use sdl2::{self, event::Event, keyboard::Keycode, Sdl};
use std::time;

use crate::{
  audio::Audio, bootrom::Bootrom, cartridge::Cartridge, cpu::Cpu, joypad::Button, lcd::LCD,
  peripherals::Peripherals,
};

pub const CPU_CLOCK_HZ: u128 = 4_194_304;
const M_CYCLE_CLOCK: u128 = 4;
const M_CYCLE_NANOS: u128 = M_CYCLE_CLOCK * 1_000_000_000 / CPU_CLOCK_HZ;

fn key2joy(keycode: Keycode) -> Option<Button> {
  match keycode {
    Keycode::Up => Some(Button::Up),
    Keycode::Down => Some(Button::Down),
    Keycode::Left => Some(Button::Left),
    Keycode::Right => Some(Button::Right),
    Keycode::Num2 => Some(Button::Start),
    Keycode::Num1 => Some(Button::Select),
    Keycode::Backspace => Some(Button::B),
    Keycode::Return => Some(Button::A),
    _ => None,
  }
}

pub struct GameBoy {
  pub cpu: Cpu,
  pub peripherals: Peripherals,
  pub lcd: LCD,
  sdl: Sdl,
}

impl GameBoy {
  pub fn new(bootrom: Bootrom, cartridge: Cartridge) -> Self {
    let sdl = sdl2::init().expect("failed to initialize SDL");
    let lcd = LCD::new(&sdl, 4);
    let audio = Audio::new(&sdl);
    let peripherals = Peripherals::new(bootrom, cartridge, audio);
    let cpu = Cpu::new();
    Self {
      cpu,
      peripherals,
      lcd,
      sdl,
    }
  }

  pub fn run(&mut self) {
    let mut event_pump = self.sdl.event_pump().unwrap();
    let time = time::Instant::now();
    let mut elapsed = 0;
    'running: loop {
      let e = time.elapsed().as_nanos();
      for _ in 0..(e - elapsed) / M_CYCLE_NANOS {
        for event in event_pump.poll_iter() {
          match event {
            Event::Quit { .. } => break 'running,
            Event::KeyDown {
              keycode: Some(k), ..
            } => {
              if k == Keycode::Escape {
                break 'running;
              }
              key2joy(k).map(|j| {
                self
                  .peripherals
                  .joypad
                  .button_down(&mut self.cpu.interrupts, j)
              });
            }
            Event::KeyUp {
              keycode: Some(k), ..
            } => {
              key2joy(k).map(|j| self.peripherals.joypad.button_up(j));
            }
            _ => (),
          }
        }
        self.cpu.emulate_cycle(&mut self.peripherals);
        self
          .peripherals
          .timer
          .emulate_cycle(&mut self.cpu.interrupts);
        self.peripherals.apu.emulate_cycle();
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
