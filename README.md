# enginesound
![Lines of Code](https://tokei.rs/b1/github/dasetwas/enginesound?category=code)

GUI Application used to generate purely synthetic engine sounds with advanced options written in Rust

freely based on [this paper](https://www.researchgate.net/publication/280086598_Physically_informed_car_engine_sound_synthesis_for_virtual_and_augmented_environments "Physically informed_car engine sound synthesis for virtual and augmented environments")

#### WIP

## Building ##

First, you will need to have SDL2 installed properly.
[Instructions on how to install SDL](https://github.com/Rust-SDL2/rust-sdl2#linux)

This project uses nightly Rust builds.

If you use rustup, you may switch to nightly using this command:
```
rustup default nightly
```
and switch back with
```
rustup default stable
```
To run the application, simply use
```
cargo run --release
```

## Preview ##
### GUI ###
![Image](gui_ff0d902.PNG)

### Sound ###
Generated using the config shown above while adjusting the RPM manually: [Audio file](https://raw.githubusercontent.com/DasEtwas/enginesound/master/example_ff0d902.mp3)

## Licensing ##

MIT License

## Credits ##

[conrod](https://github.com/PistonDevelopers/conrod) made making the GUI trivial.
