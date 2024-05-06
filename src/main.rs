use std::{fs::File, io::Read};

mod bootrom;
mod cpu;
mod gameboy;
mod hram;
mod lcd;
mod peripherals;
mod ppu;
mod wram;

fn main() {
  let mut file = File::open("./dmg_bootrom.bin").expect("file not found");

  let mut ret = vec![];
  file.read_to_end(&mut ret).unwrap();

  let bootrom = bootrom::Bootrom::new(ret.into());
  let mut gameboy = gameboy::GameBoy::new(bootrom);
  gameboy.run();
}
