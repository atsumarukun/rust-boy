use crate::{
  cpu::{
    go,
    operand::{Cond, Imm16, Imm8, Reg16, IO16, IO8},
    step, Cpu,
  },
  peripherals::Peripherals,
};
use std::sync::atomic::{AtomicU16, AtomicU8, Ordering::Relaxed};

impl Cpu {
  pub fn nop(&mut self, bus: &Peripherals) {
    self.fetch(bus);
  }

  pub fn ld<D: Copy, S: Copy>(&mut self, bus: &mut Peripherals, dst: D, src: S)
  where
    Self: IO8<D> + IO8<S>,
  {
    step!((), {
      0: if let Some(v) = self.read8(bus, src) {
        VAL8.store(v, Relaxed);
        go!(1);
      },
      1: if self.write8(bus, dst, VAL8.load(Relaxed)).is_some() {
        go!(2);
      },
      2: {
        go!(0);
        self.fetch(bus);
      },
    });
  }

  pub fn ld16<D: Copy, S: Copy>(&mut self, bus: &mut Peripherals, dst: D, src: S)
  where
    Self: IO16<D> + IO16<S>,
  {
    step!((), {
      0: if let Some(v) = self.read16(bus, src) {
        VAL16.store(v, Relaxed);
        go!(1);
      },
      1: if self.write16(bus, dst, VAL16.load(Relaxed)).is_some() {
        go!(2);
      },
      2: {
        go!(0);
        self.fetch(bus);
      },
    });
  }

  pub fn cp<S: Copy>(&mut self, bus: &Peripherals, src: S)
  where
    Self: IO8<S>,
  {
    if let Some(v) = self.read8(bus, src) {
      let (result, carry) = self.regs.a.overflowing_sub(v);
      self.regs.set_zf(result == 0);
      self.regs.set_nf(true);
      self.regs.set_hf((self.regs.a & 0xf) < (v & 0xf));
      self.regs.set_cf(carry);
      self.fetch(bus);
    }
  }

  pub fn inc<S: Copy>(&mut self, bus: &mut Peripherals, src: S)
  where
    Self: IO8<S>,
  {
    step!((), {
      0: if let Some(v) = self.read8(bus, src) {
        let result = v.wrapping_add(1);
        self.regs.set_zf(result == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(v & 0xf == 0xf);
        VAL8.store(result, Relaxed);
        go!(1);
      },
      1: if self.write8(bus, src, VAL8.load(Relaxed)).is_some() {
        go!(0);
        self.fetch(bus);
      },
    });
  }

  pub fn inc16<S: Copy>(&mut self, bus: &mut Peripherals, src: S)
  where
    Self: IO16<S>,
  {
    step!((), {
      0: if let Some(v) = self.read16(bus, src) {
        VAL16.store(v.wrapping_add(1), Relaxed);
        go!(1);
      },
      1: if self.write16(bus, src, VAL16.load(Relaxed)).is_some() {
        return go!(2);
      },
      2: {
        go!(0);
        self.fetch(bus);
      },
    });
  }

  pub fn dec<S: Copy>(&mut self, bus: &mut Peripherals, src: S)
  where
    Self: IO8<S>,
  {
    step!((), {
      0: if let Some(v) = self.read8(bus, src) {
        let result = v.wrapping_sub(1);
        self.regs.set_zf(result == 0);
        self.regs.set_nf(true);
        self.regs.set_hf(v & 0xf == 0);
        VAL8.store(result, Relaxed);
        go!(1);
      },
      1: if self.write8(bus, src, VAL8.load(Relaxed)).is_some() {
        go!(0);
        self.fetch(bus);
      },
    });
  }

  pub fn dec16<S: Copy>(&mut self, bus: &mut Peripherals, src: S)
  where
    Self: IO16<S>,
  {
    step!((), {
      0: if let Some(v) = self.read16(bus, src) {
        VAL16.store(v.wrapping_sub(1), Relaxed);
        go!(1);
      },
      1: if self.write16(bus, src, VAL16.load(Relaxed)).is_some() {
        return go!(2);
      },
      2: {
        go!(0);
        self.fetch(bus);
      },
    });
  }

  pub fn rl<S: Copy>(&mut self, bus: &mut Peripherals, src: S)
  where
    Self: IO8<S>,
  {
    step!((), {
      0: if let Some(v) = self.read8(bus, src) {
        let result = (v << 1) | self.regs.cf() as u8;
        self.regs.set_zf(result == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(v & 0x80 > 0);
        VAL8.store(result, Relaxed);
        go!(1);
      },
      1: if self.write8(bus, src, VAL8.load(Relaxed)).is_some() {
        go!(0);
        self.fetch(bus);
      },
    });
  }

  pub fn bit<S: Copy>(&mut self, bus: &Peripherals, bit: usize, src: S)
  where
    Self: IO8<S>,
  {
    if let Some(mut v) = self.read8(bus, src) {
      v &= 1 << bit;
      self.regs.set_zf(v == 0);
      self.regs.set_nf(false);
      self.regs.set_hf(true);
      self.fetch(bus);
    }
  }

  pub fn push16(&mut self, bus: &mut Peripherals, val: u16) -> Option<()> {
    step!(None, {
      0: {
        go!(1);
        return None;
      },
      1: {
        let [lo, hi] = u16::to_le_bytes(val);
        self.regs.sp = self.regs.sp.wrapping_sub(1);
        bus.write(&mut self.interrupts, self.regs.sp, hi);
        VAL8.store(lo, Relaxed);
        go!(2);
        return None;
      },
      2: {
        self.regs.sp = self.regs.sp.wrapping_sub(1);
        bus.write(&mut self.interrupts, self.regs.sp, VAL8.load(Relaxed));
        go!(3);
        return None;
      },
      3: return Some(go!(0)),
    });
  }

  pub fn push(&mut self, bus: &mut Peripherals, src: Reg16) {
    step!((), {
      0: {
        VAL16.store(self.read16(bus, src).unwrap(), Relaxed);
        go!(1);
      },
      1: if self.push16(bus, VAL16.load(Relaxed)).is_some() {
          go!(2);
      },
      2: {
        go!(0);
        self.fetch(bus);
      },
    });
  }

  pub fn pop16(&mut self, bus: &Peripherals) -> Option<u16> {
    step!(None, {
      0: {
        VAL8.store(bus.read(&self.interrupts, self.regs.sp), Relaxed);
        self.regs.sp = self.regs.sp.wrapping_add(1);
        go!(1);
        return None;
      },
      1: {
        let hi = bus.read(&self.interrupts, self.regs.sp);
        self.regs.sp = self.regs.sp.wrapping_add(1);
        VAL16.store(u16::from_le_bytes([VAL8.load(Relaxed), hi]), Relaxed);
        go!(2);
        return None;
      },
      2: {
        go!(0);
        return Some(VAL16.load(Relaxed));
      },
    });
  }

  pub fn pop(&mut self, bus: &mut Peripherals, dst: Reg16) {
    if let Some(v) = self.pop16(bus) {
      self.write16(bus, dst, v);
      self.fetch(bus);
    }
  }

  pub fn jr(&mut self, bus: &Peripherals) {
    step!((), {
      0: if let Some(v) = self.read8(bus, Imm8) {
        self.regs.pc = self.regs.pc.wrapping_add(v as i8 as u16);
        return go!(1);
      },
      1: {
        go!(0);
        self.fetch(bus);
      },
    });
  }

  fn cond(&self, cond: Cond) -> bool {
    match cond {
      Cond::NZ => !self.regs.zf(),
      Cond::Z => self.regs.zf(),
      Cond::NC => !self.regs.cf(),
      Cond::C => self.regs.cf(),
    }
  }

  pub fn jr_c(&mut self, bus: &Peripherals, c: Cond) {
    step!((), {
      0: if let Some(v) = self.read8(bus, Imm8) {
        go!(1);
        if self.cond(c) {
          self.regs.pc = self.regs.pc.wrapping_add(v as i8 as u16);
          return;
        }
      },
      1: {
        go!(0);
        self.fetch(bus);
      },
    });
  }

  pub fn call(&mut self, bus: &mut Peripherals) {
    step!((), {
      0: if let Some(v) = self.read16(bus, Imm16) {
        VAL16.store(v, Relaxed);
        go!(1);
      },
      1: if self.push16(bus, self.regs.pc).is_some() {
        self.regs.pc = VAL16.load(Relaxed);
        go!(0);
        self.fetch(bus);
      },
    });
  }

  pub fn ret(&mut self, bus: &Peripherals) {
    step!((), {
      0: if let Some(v) = self.pop16(bus) {
        self.regs.pc = v;
        return go!(1);
      },
      1: {
        go!(0);
        self.fetch(bus);
      },
    });
  }

  pub fn reti(&mut self, bus: &Peripherals) {
    step!((), {
      0: if let Some(v) = self.pop16(bus) {
        self.regs.pc = v;
        return go!(1);
      },
      1: {
        self.interrupts.ime = true;
        go!(0);
        self.fetch(bus);
      },
    });
  }

  pub fn halt(&mut self, bus: &Peripherals) {
    step!((), {
      0: if self.interrupts.get_interrupt() > 0 {
        self.fetch(bus);
      } else {
        return go!(1);
      },
      1: {
        if self.interrupts.get_interrupt() > 0 {
          go!(0);
          self.fetch(bus);
        }
      },
    });
  }

  pub fn di(&mut self, bus: &Peripherals) {
    self.interrupts.ime = false;
    self.fetch(bus);
  }

  pub fn ei(&mut self, bus: &Peripherals) {
    self.fetch(bus);
    self.interrupts.ime = true;
  }

  pub fn add<S: Copy>(&mut self, bus: &Peripherals, src: S)
  where
    Self: IO8<S>,
  {
    if let Some(v) = self.read8(bus, src) {
      let (result, carry) = self.regs.a.overflowing_add(v);
      self.regs.set_zf(result == 0);
      self.regs.set_nf(false);
      self.regs.set_hf((self.regs.a & 0xf) + (v & 0xf) > 0xf);
      self.regs.set_cf(carry);
      self.regs.a = result;
      self.fetch(bus);
    }
  }

  pub fn adc<S: Copy>(&mut self, bus: &Peripherals, src: S)
  where
    Self: IO8<S>,
  {
    let cy = self.regs.cf() as u8;
    if let Some(v) = self.read8(bus, src) {
      let result = self.regs.a.wrapping_add(v).wrapping_add(cy);
      self.regs.set_zf(result == 0);
      self.regs.set_nf(false);
      self.regs.set_hf((self.regs.a & 0xf) + (v & 0xf) + cy > 0xf);
      self
        .regs
        .set_cf(self.regs.a as u16 + v as u16 + cy as u16 > 0xff);
      self.regs.a = result;
      self.fetch(bus);
    }
  }

  pub fn sub<S: Copy>(&mut self, bus: &Peripherals, src: S)
  where
    Self: IO8<S>,
  {
    if let Some(v) = self.read8(bus, src) {
      let (result, carry) = self.regs.a.overflowing_sub(v);
      self.regs.set_zf(result == 0);
      self.regs.set_nf(true);
      self.regs.set_hf((self.regs.a & 0xf) < (v & 0x0f));
      self.regs.set_cf(carry);
      self.regs.a = result;
      self.fetch(bus);
    }
  }

  pub fn sbc<S: Copy>(&mut self, bus: &Peripherals, src: S)
  where
    Self: IO8<S>,
  {
    let cy = self.regs.cf() as u8;
    if let Some(v) = self.read8(bus, src) {
      let result = self.regs.a.wrapping_sub(v).wrapping_sub(cy);
      self.regs.set_zf(result == 0);
      self.regs.set_nf(true);
      self.regs.set_hf((self.regs.a & 0xf) < (v & 0xf) + cy);
      self
        .regs
        .set_cf((self.regs.a as u16) < (v as u16) + (cy as u16));
      self.regs.a = result;
      self.fetch(bus);
    }
  }

  pub fn and<S: Copy>(&mut self, bus: &Peripherals, src: S)
  where
    Self: IO8<S>,
  {
    if let Some(v) = self.read8(bus, src) {
      self.regs.a &= v;
      self.regs.set_zf(self.regs.a == 0);
      self.regs.set_nf(false);
      self.regs.set_hf(true);
      self.regs.set_cf(false);
      self.fetch(bus);
    }
  }

  pub fn or<S: Copy>(&mut self, bus: &Peripherals, src: S)
  where
    Self: IO8<S>,
  {
    if let Some(v) = self.read8(bus, src) {
      self.regs.a |= v;
      self.regs.set_zf(self.regs.a == 0);
      self.regs.set_nf(false);
      self.regs.set_hf(false);
      self.regs.set_cf(false);
      self.fetch(bus);
    }
  }

  pub fn xor<S: Copy>(&mut self, bus: &Peripherals, src: S)
  where
    Self: IO8<S>,
  {
    if let Some(v) = self.read8(bus, src) {
      self.regs.a ^= v;
      self.regs.set_zf(self.regs.a == 0);
      self.regs.set_nf(false);
      self.regs.set_hf(false);
      self.regs.set_cf(false);
      self.fetch(bus);
    }
  }

  pub fn rlca(&mut self, bus: &Peripherals) {
    self.regs.set_zf(false);
    self.regs.set_nf(false);
    self.regs.set_hf(false);
    self.regs.set_cf(self.regs.a & 0x80 > 0);
    self.regs.a = (self.regs.a << 1) | (self.regs.a >> 7);
    self.fetch(bus);
  }

  pub fn rla(&mut self, bus: &Peripherals) {
    let cf = self.regs.cf();
    self.regs.set_zf(false);
    self.regs.set_nf(false);
    self.regs.set_hf(false);
    self.regs.set_cf(self.regs.a & 0x80 > 0);
    self.regs.a = (self.regs.a << 1) | cf as u8;
    self.fetch(bus);
  }

  pub fn rrca(&mut self, bus: &Peripherals) {
    self.regs.set_zf(false);
    self.regs.set_nf(false);
    self.regs.set_hf(false);
    self.regs.set_cf(self.regs.a & 0x1 > 0);
    self.regs.a = (self.regs.a >> 1) | (self.regs.a << 7);
    self.fetch(bus);
  }

  pub fn rra(&mut self, bus: &Peripherals) {
    let cf = self.regs.cf();
    self.regs.set_zf(false);
    self.regs.set_nf(false);
    self.regs.set_hf(false);
    self.regs.set_cf(self.regs.a & 0x1 > 0);
    self.regs.a = (self.regs.a >> 1) | ((cf as u8) << 7);
    self.fetch(bus);
  }

  pub fn rlc<S: Copy>(&mut self, bus: &mut Peripherals, src: S)
  where
    Self: IO8<S>,
  {
    step!((), {
      0: if let Some(v) = self.read8(bus, src) {
        self.regs.set_zf(v == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(v & 0x80 > 0);
        VAL8.store((v << 1) | (v >> 7), Relaxed);
        go!(1);
      },
      1: if self.write8(bus, src, VAL8.load(Relaxed)).is_some() {
          go!(0);
          self.fetch(bus);
      },
    });
  }

  pub fn rrc<S: Copy>(&mut self, bus: &mut Peripherals, src: S)
  where
    Self: IO8<S>,
  {
    step!((), {
      0: if let Some(v) = self.read8(bus, src) {
        self.regs.set_zf(v == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(v & 0x1 > 0);
        VAL8.store((v >> 1) | (v << 7), Relaxed);
        go!(1);
      },
      1: if self.write8(bus, src, VAL8.load(Relaxed)).is_some() {
        go!(0);
        self.fetch(bus);
      },
    });
  }

  pub fn rr<S: Copy>(&mut self, bus: &mut Peripherals, src: S)
  where
    Self: IO8<S>,
  {
    step!((), {
      0: if let Some(v) = self.read8(bus, src) {
        let result = ((self.regs.cf() as u8) << 7) | (v >> 1);
        self.regs.set_zf(result == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(v & 0x1 > 0);
        VAL8.store(result, Relaxed);
        go!(1);
      },
      1: if self.write8(bus, src, VAL8.load(Relaxed)).is_some() {
        go!(0);
        self.fetch(bus);
      },
    });
  }

  pub fn sla<S: Copy>(&mut self, bus: &mut Peripherals, src: S)
  where
    Self: IO8<S>,
  {
    step!((), {
      0: if let Some(v) = self.read8(bus, src) {
        self.regs.set_zf(v & 0x7F == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(v & 0x80 > 0);
        VAL8.store(v << 1, Relaxed);
        go!(1);
      },
      1: if self.write8(bus, src, VAL8.load(Relaxed)).is_some() {
        go!(0);
        self.fetch(bus);
      },
    });
  }

  pub fn sra<S: Copy>(&mut self, bus: &mut Peripherals, src: S)
  where
    Self: IO8<S>,
  {
    step!((), {
      0: if let Some(v) = self.read8(bus, src) {
        self.regs.set_zf(v & 0xFE == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(v & 0x1 > 0);
        VAL8.store((v & 0x80) | (v >> 1), Relaxed);
        go!(1);
      },
      1: if self.write8(bus, src, VAL8.load(Relaxed)).is_some() {
        go!(0);
        self.fetch(bus);
      },
    });
  }

  pub fn srl<S: Copy>(&mut self, bus: &mut Peripherals, src: S)
  where
    Self: IO8<S>,
  {
    step!((), {
      0: if let Some(v) = self.read8(bus, src) {
        self.regs.set_zf(v & 0xFE == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(v & 0x1 > 0);
        VAL8.store(v >> 1, Relaxed);
        go!(1);
      },
      1: if self.write8(bus, src, VAL8.load(Relaxed)).is_some() {
        go!(0);
        self.fetch(bus);
      },
    });
  }

  pub fn set<S: Copy>(&mut self, bus: &mut Peripherals, bit: usize, src: S)
  where
    Self: IO8<S>,
  {
    step!((), {
      0: if let Some(v) = self.read8(bus, src) {
        VAL8.store(v | (1 << bit), Relaxed);
        go!(1);
      },
      1: if self.write8(bus, src, VAL8.load(Relaxed)).is_some() {
        go!(0);
        self.fetch(bus);
      },
    });
  }

  pub fn res<S: Copy>(&mut self, bus: &mut Peripherals, bit: usize, src: S)
  where
    Self: IO8<S>,
  {
    step!((), {
      0: if let Some(v) = self.read8(bus, src) {
        VAL8.store(v & !(1 << bit), Relaxed);
        go!(1);
      },
      1: if self.write8(bus, src, VAL8.load(Relaxed)).is_some() {
        go!(0);
        self.fetch(bus);
      },
    });
  }

  pub fn jp(&mut self, bus: &Peripherals) {
    step!((), {
      0: if let Some(v) = self.read16(bus, Imm16) {
        self.regs.pc = v;
        return go!(1);
      },
      1: {
        go!(0);
        self.fetch(bus);
      },
    });
  }

  pub fn jp_hl(&mut self, bus: &Peripherals) {
    self.regs.pc = self.regs.hl();
    self.fetch(bus);
  }

  pub fn jp_c(&mut self, bus: &Peripherals, cond: Cond) {
    step!((), {
      0: if let Some(v) = self.read16(bus, Imm16) {
        go!(1);
        if self.cond(cond) {
          self.regs.pc = v;
          return;
        }
      },
      1: {
        go!(0);
        self.fetch(bus);
      },
    });
  }

  pub fn call_c(&mut self, bus: &mut Peripherals, cond: Cond) {
    step!((), {
      0: if let Some(v) = self.read16(bus, Imm16) {
        VAL16.store(v, Relaxed);
        if self.cond(cond) {
          go!(1);
        } else {
          self.fetch(bus);
        }
      },
      1: if self.push16(bus, self.regs.pc).is_some() {
        self.regs.pc = VAL16.load(Relaxed);
        go!(0);
        self.fetch(bus);
      },
    });
  }

  pub fn ret_c(&mut self, bus: &Peripherals, cond: Cond) {
    step!((), {
      0: return go!(1),
      1: go!(if self.cond(cond) { 2 } else { 3 }),
      2: if let Some(v) = self.pop16(bus) {
        self.regs.pc = v;
        return go!(3);
      },
      3: {
        go!(0);
        self.fetch(bus);
      },
    });
  }

  pub fn rst(&mut self, bus: &mut Peripherals, addr: u8) {
    if self.push16(bus, self.regs.pc).is_some() {
      self.regs.pc = addr as u16;
      self.fetch(bus);
    }
  }

  pub fn stop(&mut self, _: &Peripherals) {
    panic!("STOP");
  }

  pub fn swap<S: Copy>(&mut self, bus: &mut Peripherals, src: S)
  where
    Self: IO8<S>,
  {
    step!((), {
      0: if let Some(v) = self.read8(bus, src) {
        self.regs.set_zf(v == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(false);
        VAL8.store((v << 4) | (v >> 4), Relaxed);
        go!(1);
      },
      1: if self.write8(bus, src, VAL8.load(Relaxed)).is_some() {
        go!(0);
        self.fetch(bus);
      },
    });
  }

  pub fn ccf(&mut self, bus: &Peripherals) {
    self.regs.set_nf(false);
    self.regs.set_hf(false);
    self.regs.set_cf(!self.regs.cf());
    self.fetch(bus);
  }

  pub fn scf(&mut self, bus: &Peripherals) {
    self.regs.set_nf(false);
    self.regs.set_hf(false);
    self.regs.set_cf(true);
    self.fetch(bus);
  }

  pub fn daa(&mut self, bus: &Peripherals) {
    let mut correction = 0;
    let mut cf = false;
    if self.regs.cf() || (!self.regs.nf() && self.regs.a > 0x99) {
      cf = true;
      correction |= 0x60;
    }
    if self.regs.hf() || (!self.regs.nf() && (self.regs.a & 0x0f > 0x09)) {
      correction |= 0x06;
    }
    if self.regs.nf() {
      self.regs.a = self.regs.a.wrapping_sub(correction);
    } else {
      self.regs.a = self.regs.a.wrapping_add(correction);
    }
    self.regs.set_zf(self.regs.a == 0);
    self.regs.set_hf(false);
    self.regs.set_cf(cf);
    self.fetch(bus);
  }

  pub fn cpl(&mut self, bus: &Peripherals) {
    self.regs.set_nf(true);
    self.regs.set_hf(true);
    self.regs.a = !self.regs.a;
    self.fetch(bus);
  }

  pub fn ld_sp_hl(&mut self, bus: &Peripherals) {
    step!((), {
      0: {
        self.regs.sp = self.regs.hl();
        return go!(1);
      },
      1: {
        go!(0);
        self.fetch(bus);
      },
    });
  }

  pub fn ld_hl_sp_e(&mut self, bus: &Peripherals) {
    step!((), {
      0: if let Some(v) = self.read8(bus, Imm8) {
        let val = v as i8 as u16;
        self.regs.set_zf(false);
        self.regs.set_nf(false);
        self.regs.set_hf((self.regs.sp & 0xF) + (val & 0xF) > 0xF);
        self.regs.set_cf((self.regs.sp & 0xFF) + (val & 0xFF) > 0xFF);
        self.regs.write_hl(self.regs.sp.wrapping_add(val));
        return go!(1);
      },
      1: {
        go!(0);
        self.fetch(bus);
      },
    });
  }

  pub fn add_hl_reg16(&mut self, bus: &Peripherals, src: Reg16) {
    step!((), {
      0: {
        let val = self.read16(bus, src).unwrap();
        let (result, carry) = self.regs.hl().overflowing_add(val);
        self.regs.set_nf(false);
        self.regs.set_hf((self.regs.hl() & 0xFFF) + (val & 0xFFF) > 0xFFF);
        self.regs.set_cf(carry);
        self.regs.write_hl(result);
        return go!(1);
      },
      1: {
        go!(0);
        self.fetch(bus);
      },
    });
  }

  pub fn add_sp_e(&mut self, bus: &Peripherals) {
    step!((), {
      0: if let Some(v) = self.read8(bus, Imm8) {
        let val = v as i8 as u16;
        self.regs.set_zf(false);
        self.regs.set_nf(false);
        self.regs.set_hf((self.regs.sp & 0xF) + (val & 0xF) > 0xF);
        self.regs.set_cf((self.regs.sp & 0xFF) + (val & 0xFF) > 0xFF);
        self.regs.sp = self.regs.sp.wrapping_add(val);
        return go!(1);
      },
      1: return go!(2),
      2: {
        go!(0);
        self.fetch(bus);
      },
    });
  }

  pub fn undefined(&mut self, _: &Peripherals) {
    panic!("Undefined opcode {:02x}", self.ctx.opcode);
  }
}
