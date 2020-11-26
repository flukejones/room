I didn't start with much of a strategy beyond getting a previous game template I wrote running with a wad loader and doing the renderer. Later I decided that I wanted to try and rewrite the Doom source for max Doom compatibility.

At first I started to transform previous rendering stuff to Doom structure, this for the most part seems to have worked okay. Next I started to look at how the `Thinker` objects were done, and this lead me down a very deep rabbit hole...

There are many main areas:
- Object storage
- Modifying objects
- Base object info and creation

- Game states
- Input
- Rendering

# Rust specific notes

`Things` need access to many different areas, like segs, lines, other `Things`. This means for Rust borrows to be satisfied some parts will need to be spit in to structs, and kept in a parent struct. Like so:

- Game, the full game and state container. Does state management, level loads, orchestration etc
  + Map/Level, contains the level data and functions to work with it
  + Thinkers<T>, all thinkers, these need access to the Map data
  + Functions and associated globals in their files can be encapsulated as Struct+impl
