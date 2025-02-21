use crate::{
  cpu::{
    interrupts::{Interrupts, JOYPAD, SERIAL, STAT, TIMER, VBLANK},
    register::Registers,
  },
  peripherals::Peripherals,
};
use std::sync::atomic::{AtomicU16, AtomicU8, Ordering::Relaxed};

mod decode;
mod fetch;
mod instructions;
pub mod interrupts;
mod operand;
mod register;

macro_rules! step {
  ($d:expr, {$($c:tt : $e:expr,)*}) => {
    static STEP: AtomicU8 = AtomicU8::new(0);
    #[allow(dead_code)]
    static VAL8: AtomicU8 = AtomicU8::new(0);
    #[allow(dead_code)]
    static VAL16: AtomicU16 = AtomicU16::new(0);
    $(if STEP.load(Relaxed) == $c { $e })* else { return $d; }
  };
}
pub(crate) use step;

macro_rules! go {
  ($e:expr) => {
    STEP.store($e, Relaxed)
  };
}
pub(crate) use go;

#[derive(Default)]
struct Ctx {
  opcode: u8,
  cb: bool,
  int: bool,
}

pub struct Cpu {
  regs: Registers,
  pub interrupts: Interrupts,
  ctx: Ctx,
}

impl Cpu {
  pub fn new() -> Self {
    Self {
      regs: Registers::default(),
      interrupts: Interrupts::default(),
      ctx: Ctx::default(),
    }
  }

  pub fn emulate_cycle(&mut self, bus: &mut Peripherals) {
    if self.ctx.int {
      self.call_isr(bus);
    } else {
      self.decode(bus);
    }
  }

  fn call_isr(&mut self, bus: &mut Peripherals) {
    step!((), {
      0: if let Some(_) = self.push16(bus, self.regs.pc) {
        let highest_int: u8 = 1 << self.interrupts.get_interrupt().trailing_zeros();
        self.interrupts.int_flags &= !highest_int;
        self.regs.pc = match highest_int {
          VBLANK => 0x0040,
          STAT => 0x0048,
          TIMER => 0x0050,
          SERIAL => 0x0058,
          JOYPAD => 0x0060,
          _ => panic!("Invalid interrupt: {:02x}", highest_int),
        };
        return go!(1);
      },
      1: {
        self.interrupts.ime = false;
        go!(0);
        self.fetch(bus)
      },
    });
  }
}
