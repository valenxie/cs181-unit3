#SMASH HIT like game

##Game logic
- Player is able to shoot marble forward, using keyboard to aim/direct.
- Marbles hits terrain/obstacles to create points/damage
- Player die if direct collision with terrain/obstacles

##Geoms required
- Blocks `Block`
  This would be the building block for our terrain / obstacles / bounding boxes.
  **What would be a good way to implement this data structure?**
  + All 8 `pos_vertex`, or
  + one `pos_vertex` & `L`, `W`, and `H` with directions 
- Spheres `Sphere`
  This would be for our marbles and their bounding spheres.
  Data stored would include `pos_center` and `r`.

##Collision logic
If we implement our camera with a bounding sphere rather than a bounding box, the only sort of collision we NEED is **sphere-block** collision and maybe **sphere-sphere** collision.

To simplify, our game should have:
- Axis-aligned terrain (like how it is in the original *SMASH HIT* game),
- Relatively stationary terrains,
- If designed carefully, obstacles that does not crash onto terrains or other obstacles.


##Physics
I believe that simple linear physics would suffice. We do have to tackle the part about breaking glasses tho.