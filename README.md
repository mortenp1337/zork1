# Zork I Source Code Collection

Zork I is a 1980 interactive fiction game written by Marc Blank, Dave Lebling, Bruce Daniels and Tim Anderson and published by Infocom.

## Rust ZIL Compiler and Z-Machine Interpreter

This repository now includes a Rust-based ZIL compiler and Z-machine interpreter that can build and run the game on Linux.

### Building and Running

#### Prerequisites

- Rust (1.70 or later) - Install via [rustup](https://rustup.rs/)

#### Quick Start

```bash
# Build the compiler and interpreter
cargo build --release

# Compile the ZIL source files to Z-machine bytecode
cargo run --release -- compile zork1.zil

# Run the compiled game
cargo run --release -- run zork1.z3

# Or run with precompiled game
cargo run --release -- run COMPILED/zork1.z3
```

#### Usage

```
zork1                     Compile and run the default game
zork1 compile <file.zil>  Compile ZIL source to Z-machine code
zork1 run [file.z3]       Run a compiled story file
zork1 <file.zil>          Compile the specified ZIL file
zork1 <file.z3>           Run the specified story file
```

### Project Structure

- `src/main.rs` - Main entry point with CLI interface
- `src/compiler/` - ZIL compiler implementation
  - `lexer.rs` - Tokenizer for ZIL source
  - `parser.rs` - Parser building AST from tokens
  - `ast.rs` - Abstract syntax tree definitions
  - `symbols.rs` - Symbol table for compilation
  - `codegen.rs` - Z-machine code generator
  - `zcode.rs` - Z-machine format definitions
- `src/zmachine/` - Z-machine interpreter
  - `mod.rs` - Main interpreter logic
  - `memory.rs` - Z-machine memory management
  - `instruction.rs` - Instruction definitions
  - `opcodes.rs` - Opcode execution

### Technical Details

The Rust implementation includes:
- **ZIL Lexer**: Tokenizes ZIL source code including strings, atoms, forms, and comments
- **ZIL Parser**: Builds an AST from tokens, handling objects, rooms, routines, and properties
- **Code Generator**: Produces Z-machine version 3 bytecode
- **Z-machine Interpreter**: Executes Z-code story files with text I/O

---

## Original Documentation

Further information on Zork I:

* [Wikipedia](https://en.wikipedia.org/wiki/Zork_I)
* [The Digital Antiquarian](https://www.filfre.net/2012/01/selling-zork/)
* [The Interactive Fiction Database](https://ifdb.tads.org/viewgame?id=0dbnusxunq7fw5ro)
* [The Infocom Gallery](http://gallery.guetech.org/zork1/zork1.html)
* [IFWiki](http://www.ifwiki.org/index.php/Zork_I)

__What is this Repository?__

This repository is a directory of source code for the Infocom game "Zork I", including a variety of files both used and discarded in the production of the game. It is written in ZIL (Zork Implementation Language), a refactoring of MDL (Muddle), itself a dialect of LISP created by MIT students and staff.

The source code was contributed anonymously and represents a snapshot of the Infocom development system at time of shutdown - there is no remaining way to compare it against any official version as of this writing, and so it should be considered canonical, but not necessarily the exact source code arrangement for production.

__Basic Information on the Contents of This Repository__

It is mostly important to note that there is currently no known way to compile the source code in this repository into a final "Z-machine Interpreter Program" (ZIP) file using an official Infocom-built compiler. There is a user-maintained compiler named [ZILF](http://zilf.io) that has been shown to successfully compile these .ZIL files with minor issues. There are .ZIP files in some of the Infocom Source Code repositories but they were there as of final spin-down of the Infocom Drive and the means to create them is currently lost.

Throughout its history, Infocom used a TOPS20 mainframe with a compiler (ZILCH) to create and edit language files - this repository is a mirror of the source code directory archive of Infocom but could represent years of difference from what was originally released.

In general, Infocom games were created by taking previous Infocom source code, copying the directory, and making changes until the game worked the way the current Implementor needed. Structure, therefore, tended to follow from game to game and may or may not accurately reflect the actual function of the code.

There are also multiple versions of the "Z-Machine" and code did change notably between the first years of Infocom and a decade later. Addition of graphics, sound and memory expansion are all slowly implemented over time.

__What is the Purpose of this Repository__

This collection is meant for education, discussion, and historical work, allowing researchers and students to study how code was made for these interactive fiction games and how the system dealt with input and processing.

Researchers are encouraged to share their discoveries about the information in this source code and the history of Infocom and its many innovative employees.
