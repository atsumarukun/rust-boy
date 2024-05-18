use std::{fs::File, io::Read};

mod bootrom;
mod cartridge;
mod cpu;
mod gameboy;
mod hram;
mod joypad;
mod lcd;
mod peripherals;
mod ppu;
mod timer;
mod wram;

fn main() {
  let mut file = File::open("./dmg_bootrom.bin").expect("file not found");

  let mut ret = vec![];
  file.read_to_end(&mut ret).unwrap();

  let mut file = File::open("./POKEMON.GB").expect("file not found");

  let mut data = vec![];
  file.read_to_end(&mut data).unwrap();

  let bootrom = bootrom::Bootrom::new(ret.into());
  let rom = cartridge::Cartridge::new(data.into());
  let mut gameboy = gameboy::GameBoy::new(bootrom, rom);
  gameboy.run();
}
