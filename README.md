# Danganronpa: Ultra Despair Girls mod

## Features

* Change Komaru's outfit anywhere in the game
  * [Screenshot](./media/outfit.jpeg?raw=1)

* Enable clothes destruction mode anywhere in the game, which makes Komaru lose her skirt and shirt after taking a few hits just like in the chapter 4 boss fight
  * [Clip](./media/clothes_destruction.mp4?raw=1)

* Toggle visibility of Toko's skirt
  * [Screenshot](./media/toko_skirt.jpeg?raw=1)

* Free camera mode
  * [Screenshot](./media/freecam.jpeg?raw=1)

* Pause and frame advance

* Extends culling distance for objects so you can always see stuff like enemies or NPCs far away
  * Screenshots:
  [Before](./media/culldist_before.jpeg?raw=1) /
  [After](./media/culldist_after.jpeg?raw=1)

* 4x drawing distance for shadows
  * Screenshots:
  [Before](./media/shadows_before.jpeg?raw=1) /
  [After](./media/shadows_after.jpeg?raw=1)

* Fixes mouse movement in supersampled borderless fullscreen

* Disables exclusive fullscreen mode in favor of borderless fullscreen (`Alt + Enter` to toggle)

## Installation

Just place the d3d11.dll file right next to the game's executable (game.exe) and start the game.

## Usage

### Outfit Changing

* `F2` - Restore Komaru's default outfit
* `F3` - Change to the no-skirt outfit
* `F4` - Change to the no-skirt and no-shirt outfit
* `Shift + F2` - Enable clothes destruction mode
  This restores Komaru's outfit first, then enables clothes destruction mode. Komaru will lose her skirt after taking a few hits, and then lose her shirt after taking a few more hits. This is the same behavior as in the chapter 4 boss fight. Pressing any of the other keys will disable clothes destruction mode.

**Note:** There's a risk of crashing when changing outfits right after a loading screen. The mod delays outfit changes a bit during loading to prevent this but it could still happen so beware!

Outfit cannot be changed during cutscenes! Instead, your selection is saved and applied when the cutscene ends.

### Free Camera

* `F8` - Toggle free cam mode
* `I` `K` - Move forward/backwards
* `J` `l` - Move left/right
* `U` `O` - Move down/up
* `H` - Hold to move faster
* `Y` - Hold to move slower
* `F7` - Toggle visibility of Komaru's gun in free cam mode

Object culling is disabled in free cam mode so you can inspect characters as close as you want.

In some cutscenes, the camera will not go back to normal after you move it. It shouldn't cause problems though!

### Toko's Skirt

* `F5` - Toggle remove Toko's skirt

This is more of a hack than Komaru since Toko isn't actually ever meant to lose her skirt. It should work anywhere!

### Pause / Frame Advance

* `F10` - Toggle game pause
* `F11` - Advance one frame

## Uninstalling

Simply delete the d3d11.dll file! And may want to delete the udgmod.log file as well.

## Building

There are no fancy build requirements, simply install Rust and `cargo build`! 🦀
