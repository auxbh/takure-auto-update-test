# Takure

DDR hook to submit your scores to a Tachi instance while you are playing.  
It is basically a fork of [Mikado](https://github.com/adamaq01/Mikado) but for DDR.

Credits to [adamaq01](https://github.com/adamaq01) for creating Mikado and helping out with this project!

## Features

- Submit scores to a Tachi instance after each song

## Support

Takure supports both 32 and 64 bit versions of DDR A3 and DDR WORLD.

Score submission only works with Single and Double style. Versus play will not submit scores.

## Installation

- Download the latest release from the [releases page](https://github.com/auxbh/takure/releases/latest)
    - Choose `takure_32.zip` for a 32 bit game 
    - Choose `takure_64.zip` for a 64 bit game
- Extract the DLL in your game's `modules` folder (or in the root directory if you don't have one)
- Place `takure.toml` in the same folder as the DLL
- Inject `takure.dll` into the game process

## Tips

- The configuration file will be created in the same folder as the DLL at startup if it doesn't already exist
- You can configure some options (like the Tachi URL) by editing the `takure.toml` file

<details>
<summary>Building</summary>

Simply run `cargo build --release --target i686-pc-windows-msvc` for 32 bit, or `cargo build --release --target x86_64-pc-windows-msvc` for 64 bit.
Make sure to install the target(s) beforehand.

**Tip:** If you wish to debug locally, build ommiting the `--release` flag, which will enable debug logging and won't connect to a Tachi instance.
</details>
