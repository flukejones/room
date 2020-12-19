TODO: detail the portal/window handling

# Movement and collision handling

All `mobj_t` map objects, which are moveable entities spawned from map `Things`
hve movement and collision checks. When an entity moves it begins a chain of
calls:

`P_XYMovement`, this preps a move by taking the object position and adding the
object momentum, it then checks the move with `P_TryMove` and if unsuccessful
it will halve the momentum in each loop and check again (until momentum is zero).
It will have been faster to do this in a loop than to use dot products and
divisions on the 486DX, especially given the use of tables of angles and sine/cos.

`P_TryMove` will call `P_CheckPosition` to see if a move is valid, and if so it
will run special line/seg triggers if any were crossed, and check `Z` axis
movement. The Z movement checks thing-to-ceilign height, and step height.

`P_CheckPosition` is the begining of the collision detection. It sets things up
to then call `PIT_CheckThing` and `PIT_CheckLine` - checking thing-to-thing
collisions and thing-to-line collisions.


## Thing-to-line Collisions

The primary function for this is `PIT_CheckLine`.

First there is a wide-phase check against `Thing` axis-aligned bounding-box,
(remember Doom is top-down 2D), if there is an AABB collision then it goes on
to check the AABB against the line.

Each line stores two (additional) items on level load: its AABB, and a line
slope (2D XY). The slope is used to help very quickly check for AABB
to line intersections as what it stores is whether the line is axis-aligned, or
the slope is positive or negative.

Also checked are:
- if the linedef blocks all, or only monsters
- front and back sector heights to see if a step is allowed
- special lines (such as triggers)

If a line or tall step are encountered where the player is blocked, then the
function `P_SlideMove` is called to check if the player can *slide* along the
wall.

## Movement

Movement for players is dictated but a `ticcmd`. This structure contains all
the possible player actions and is also used for demo records and net-play.

As this is done at 35fps, it provides a consistent and (mostly) predictable
world to play in, also meaning that the engine doesn't really need to do anything
like collision penetration depth and subsequent movement of `Thing`s. It just
checks moves in progressively smaller steps in a loop.

A second function is called if a collision is detected; `P_SlideMove`. Comments
above this particular function are `This is a kludgy mess`... Which is going to
be the main driver of my updating this code to use a more modern approach.
Seriously, it's not nice to look at. And the modernisation will come with the
ability to do slides anyway.

The essence of `P_SlideMove` is to move the player along the wall, the direction
is gotten from the players angle to the wall normal (dot + cosine), with the
cosine also used to modulate the players momentum. That is, cosine is 0.0 for player
facing straight in to the wall, 1.0 for facing along the wall.

## Updating

In the Rust rewrite I'm going to add a switch between classic/modern movement and
collisions. Modern will use the standard 2D techniques, mostly consisting of
circle-to-seg collisions and penetration depth, and thus dot-products.

The first step is going to be condensing the call chain down, then removing the
loop in favour of a single call and pen-depth + momentum resolution.

enemy.c calls the same movement code, but has some additional flag checks for
moving height or floating. These can be moved in to the main movement block.
