# Zork I ZIL Compiler - Implementation TODO

This document outlines the remaining work needed to fully implement the ZIL compiler for Zork I.

## Current Status

The compiler currently:
- ✅ Parses basic ZIL syntax (lexer and parser)
- ✅ Loads ZIL source files
- ✅ Creates a runtime environment
- ✅ Implements basic game loop with user input
- ✅ Supports basic commands (movement, look, take, drop, open, close, inventory)
- ✅ Falls back to hardcoded world when parsing is incomplete (23 rooms, 8 objects)

## Priority 1: Fix ZIL Parser

The parser currently only loads ~11 definitions from `1dungeon.zil` which contains 110 rooms and 120+ objects.

### Tasks
- [ ] **Debug parser stopping early** - The parser stops after ~100 lines of `1dungeon.zil`
  - Investigate why parse errors cause early termination
  - Add better error recovery to continue parsing after errors
  - Add verbose debug mode to trace parsing issues

- [ ] **Handle all ZIL property types**
  - [ ] `ADJECTIVE` - object adjectives for disambiguation
  - [ ] `SYNONYM` - alternative names for objects
  - [ ] `ACTION` - action routine references  
  - [ ] `LDESC` / `FDESC` - long/first descriptions
  - [ ] `SIZE` / `CAPACITY` - object sizes
  - [ ] `VALUE` / `TVALUE` - treasure values
  - [ ] `STRENGTH` - combat strength
  - [ ] `TEXT` - readable text content

- [ ] **Handle room exit formats**
  - [ ] Simple exits: `(NORTH TO ROOM-NAME)`
  - [ ] Conditional exits: `(NORTH TO ROOM IF FLAG)` 
  - [ ] String exits (blocked): `(EAST "The door is locked.")`
  - [ ] Exit with functions: `(NORTH PER ROUTINE-NAME)`

- [ ] **Parse all ZIL forms**
  - [ ] `<TELL ...>` - output text
  - [ ] `<COND ...>` - conditionals
  - [ ] `<REPEAT ...>` - loops
  - [ ] `<SETG ...>` / `<SET ...>` - variable assignment
  - [ ] `<MOVE ...>` - object movement
  - [ ] `<REMOVE ...>` - remove from container
  - [ ] `<FSET ...>` / `<FCLEAR ...>` - flag operations
  - [ ] `<GETP ...>` / `<PUTP ...>` - property access
  - [ ] `<IN? ...>` / `<LOC ...>` - location queries
  - [ ] `<FIRST? ...>` / `<NEXT? ...>` - object tree traversal

## Priority 2: Implement ZIL Routine Execution

Currently routines are parsed but not executed. This is needed for the game to function properly.

### Tasks
- [ ] **Implement routine calling**
  - [ ] Local variable binding
  - [ ] Argument passing
  - [ ] Return values
  - [ ] Nested calls and call stack

- [ ] **Implement control flow**
  - [ ] `<COND>` conditionals with multiple clauses
  - [ ] `<AND>` / `<OR>` short-circuit evaluation
  - [ ] `<REPEAT>` with `<RETURN>` and `<AGAIN>`
  - [ ] `<PROG>` blocks with local bindings

- [ ] **Implement built-in functions**
  - [ ] Arithmetic: `+`, `-`, `*`, `/`, `MOD`, `RANDOM`
  - [ ] Comparison: `=?`, `N=?`, `L?`, `G?`, `L=?`, `G=?`
  - [ ] Logic: `NOT`, `BTST`, `BOR`, `BAND`
  - [ ] String: `PRINTD`, `PRINTN`, `PRINTR`, `CRLF`

## Priority 3: Game Logic Implementation

### Parser/Input System
- [ ] **Implement full parser** (`gparser.zil`)
  - [ ] Multi-word commands
  - [ ] Object disambiguation ("which sword?")
  - [ ] Prepositions (put X in Y, give X to Y)
  - [ ] Pronouns (IT, THEM, HIM, HER)
  - [ ] Special commands (OOPS, AGAIN)

### Action System  
- [ ] **Implement verb actions** (`gverbs.zil`, `1actions.zil`)
  - [ ] TAKE/GET - with container handling
  - [ ] DROP/PUT - placement and containers
  - [ ] OPEN/CLOSE - doors and containers
  - [ ] LOCK/UNLOCK - with key requirements
  - [ ] EXAMINE - detailed descriptions
  - [ ] READ - readable objects
  - [ ] EAT/DRINK - consumables
  - [ ] ATTACK/KILL - combat
  - [ ] GIVE/SHOW - NPC interaction
  - [ ] CLIMB/ENTER/EXIT - special movement
  - [ ] PUSH/PULL/TURN - object manipulation
  - [ ] TIE/UNTIE - rope puzzles
  - [ ] DIG - with shovel
  - [ ] PRAY/WISH - special actions
  - [ ] WAVE/RING - magic items

### Game Mechanics
- [ ] **Light/darkness system**
  - [ ] Track light sources (lantern, candles, torch)
  - [ ] Battery/fuel consumption
  - [ ] Grues attack in darkness

- [ ] **Combat system**
  - [ ] Weapon effectiveness
  - [ ] Enemy AI (troll, thief, cyclops)
  - [ ] Player death and resurrection

- [ ] **Scoring system**
  - [ ] Treasure collection
  - [ ] Puzzle completion
  - [ ] Special actions (kill thief, etc.)

- [ ] **Event/daemon system** (`gclock.zil`)
  - [ ] Timed events
  - [ ] Thief movement
  - [ ] Reservoir flooding
  - [ ] Candle/lantern timeout

## Priority 4: Complete Game World

### Missing Rooms (87 more needed)
The full game has 110 rooms. Currently only 23 are hardcoded.

Key areas to add:
- [ ] Coal Mine area (10+ rooms)
- [ ] Flood Control Dam area (5+ rooms)
- [ ] Maze areas (15+ rooms)
- [ ] Hades / Land of the Dead
- [ ] Atlantis area
- [ ] End game areas

### Missing Objects (100+ more needed)
- [ ] All treasures (19 total)
- [ ] All tools (shovel, screwdriver, etc.)
- [ ] All weapons (sword, knife, etc.)
- [ ] All containers (sack, case, etc.)
- [ ] All NPCs (troll, thief, cyclops, etc.)
- [ ] All puzzle objects

## Priority 5: Polish and Testing

- [ ] **Save/Restore system**
  - [ ] Serialize game state
  - [ ] File I/O for save files
  - [ ] RESTART command

- [ ] **Transcript/Script support**
  - [ ] SCRIPT ON/OFF commands
  - [ ] Log all output to file

- [ ] **Testing**
  - [ ] Unit tests for parser
  - [ ] Unit tests for runtime
  - [ ] Integration tests for commands
  - [ ] Full game playthrough test

- [ ] **Documentation**
  - [ ] Code documentation
  - [ ] Architecture overview
  - [ ] ZIL language reference

## File Structure

```
src/
├── main.rs      - Entry point, file loading
├── lexer.rs     - ZIL tokenizer
├── parser.rs    - ZIL parser (needs fixes)
├── ast.rs       - Abstract syntax tree types
├── runtime.rs   - Game state and ZIL execution
└── game.rs      - Game loop and command processing

ZIL source files:
├── zork1.zil    - Main game file
├── 1dungeon.zil - Rooms and objects (110 rooms, 120+ objects)
├── 1actions.zil - Action routines
├── gparser.zil  - Input parser
├── gverbs.zil   - Verb definitions
├── gmain.zil    - Main game loop
├── gclock.zil   - Event/daemon system
├── gglobals.zil - Global variables
├── gsyntax.zil  - Syntax definitions
└── gmacros.zil  - Macro definitions
```

## Contributing

To work on this project:

1. Pick a task from this list
2. Create a branch: `git checkout -b feature/task-name`
3. Make changes and test: `cargo build && cargo run`
4. Submit a PR with your changes

## Notes

- The original ZIL files are from the Infocom source code release
- ZILF (http://zilf.io) is a reference implementation that successfully compiles these files
- The Z-machine specification is available at https://inform-fiction.org/zmachine/standards/
