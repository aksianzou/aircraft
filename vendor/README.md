# Vendored crates

## agb_image_converter 0.25.0

From crates.io, licensed MPL-2.0 (https://mozilla.org/MPL/2.0/), source at
https://github.com/agbrs/agb. Two changes, both in `src/palette16.rs` and
marked `aircraft:`.

The colours of each optimised palette are sorted, transparent first and then
by brightness.

Upstream takes them from a `HashSet`, so the order of every sprite palette
changed from build to build. The night, heat and infrared glass repaints a
figure by its colour index, so the figures came out a different brightness on
every build.

After packing, palettes that fit together in sixteen colours are merged, the
pair sharing the most colours first. The packer left most palettes a third
full, and every palette a frame touches takes one of the sixteen object
palette banks: with the redrawn vehicles a frame ran out of banks and the
game stopped. Merged, a flight frame uses four to seven.

Remove this copy once agb does both itself.
