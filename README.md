# Iron Boy

A work-in-progress Game Boy Color emulator.

## Features

This project puts an emphasis on game playability and cross-platform compatibility
rather than extereme accuracy.

In the future, I'd like to split the core emulator into its own crate and implement
frontends for desktop and web, (and possibly mobile). For now, the implementation is
somewhat monolitic and has only been tried on desktop Linux.

Core features:
 - [x] Nearly complete CPU
 - [x] PPU with color support
 - [x] APU/sound support
 - [x] Minimal save support
 - [x] MBC1
 - [x] MBC2
 - [x] MBC3 with RTC
 - [ ] MBC5
 - [ ] All CGB features (though many games are already playable)
 - [ ] Save states
 - [ ] Fast-forward
 
## License

This project is licensed under the GPLv3. See
[LICENSE.md](../../blob/main/LICENSE.md) for more details.

## Acknowledgements

Special thanks to the following projects for documentation and references:

- [pandocs](https://gbdev.io)
- [Mooneye Emulator](https://github.com/Gekkio/mooneye-gb)
- [SameBoy](https://sameboy.github.io/)
