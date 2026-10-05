# DRONE

A reconnaissance aircraft for the Game Boy Advance. The cartridge is **`aircraft.gba`**. It opens in the official mGBA.

The field is fictional: grass, dirt tracks, trenches. The sky and the instruments do not repeat the map. The developer credit appears once, on the second splash: **ILICH GAMES**.

Four aircraft. The spotter holds altitude and calls a salvo. The general-purpose drone carries a large set of charges and flies forward; **Select** switches it to a top-down view and a slow slide, and Select again restores the previous nose. The quadcopter of missions 21 and 22 always slides, and **Select** only changes the view: speed stays the same and grows while a direction is held. FPV is a kamikaze, with a wire running ahead to the charge. The top view is a shaded plan: a tank shows tracks, a turret, and a hatch in the centre. A mine, bomb, high explosive, and VOG dropped in a hover fall into the crosshair with no drift. **Start** opens pause: Resume continues the sortie, Exit returns to level select. Level progress is written to the save. A cleared level can be selected again.

## Glass

- Day. A cycles the charge, B releases it. Any blast hits infantry inside its radius. A bomb takes trucks and finishes a tank in three nearby hits. VOG is infantry only. High explosive takes bunkers. Any charge except the bullet takes a tank on the first hit if it lands in the hatch. A mine nearby finishes a tank in two hits.
- Night. The same aircraft, through green glass. Bodies glow, the ground does not.
- Heat. Cold ground, hot vehicles and bunkers.
- Guidance. Grey glass, no bombs. A light frame lies on the ground under the sight: it is part of the map and stays put when the camera leaves. B locks it. Four seconds later a salvo of six rockets arrives. The flash is white and only at the impact point. The spotter moves at the same speed as the infantry on a rush, and does not reach the posts before they do.

In mGBA the d-pad is the cross, X is A, Z is B, Return is Start, and Backspace is Select.

## Missions

1. Trench. Six soldiers in a line.
2. Officer. One marked man.
3. Convoy. Do not let the truck reach the posts.
4. Headquarters. Break the marked tiles.
5. Night. Three trucks.
6. Heat. Three trucks.
7. Gate. Fly the gap at the required height.
8. FPV. Four drones, four pairs. When the drones run out, the mission starts over.
9. Guidance. Three parked trucks. The salvo lands under the sight, including from far away.
10. Armour, night. Three tanks far apart. Three bombs near each one are enough. A charge in the hatch takes a tank at once. The bullet does not.
11. Bunkers, heat. Three bunkers. High explosive takes them. They fire if you descend.
12. Column, guidance. Trucks crawl, so the mark has to lead them.
13. Lane, day. Below 15 the radar shows yellow dots in the gaps clear of the guns. Above 15 the guns fire everywhere. The trucks are at the end of the lane.
14. Rush, guidance. Infantry run for the base posts. Place the mark ahead of the group: during the rockets' flight they walk into it.
15. Flare, night. The same lane, three tanks. Put the charge in the hatch.
16. Nest, FPV. Four men stand in the open gaps. Fly the drone onto them.
17. Dart, FPV, heat. Three trucks in the same gaps.
18. Push, guidance. Three groups: a truck and two men beside it. One mark per group, ahead of it. The right-hand group is first: it is closer to the posts, and the salvo reaches it from the pad.
19. Skirt, heat. Three bunkers past the lane.
20. Bend, guidance. Altitude is locked under the gun ceiling, so only the lane and the salvo remain.
21. Lift. A quadcopter, looking straight down from the start. It holds one mine. B over a mine picks it up, and the next B drops it into a tank hatch. Select changes the view. Speed depends on how long the stick is held.
22. Haul. The same, with the mines and tanks farther out. While a mine is in the claws, the radar shows the tanks.

## Build

Requires the nightly from `rust-toolchain.toml` and the `agb` crate 0.25.

```sh
cargo build --release
```

The finished ELF is packed into `aircraft.gba`. The image must begin with the bytes `2e 00 00 ea`.
