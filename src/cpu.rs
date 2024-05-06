use crate::{cpu::register::Registers, peripherals::Peripherals};

#[derive(Default)]
struct Ctx {
  opcode: u8,
  cb: bool,
}

mod decode;
mod fetch;
mod instructions;
mod operand;
mod register;

pub struct Cpu {
  regs: Registers,
  ctx: Ctx,
}

impl Cpu {
  pub fn new() -> Self {
    Self {
      regs: Registers::default(),
      ctx: Ctx::default(),
    }
  }

  pub fn emulate_cycle(&mut self, bus: &mut Peripherals) {
    self.decode(bus);
  }
}
