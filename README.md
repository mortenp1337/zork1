# Zork I Source Code Collection

Zork I is a 1980 interactive fiction game written by Marc Blank, Dave Lebling, Bruce Daniels and Tim Anderson and published by Infocom.

Further information on Zork I:

* [Wikipedia](https://en.wikipedia.org/wiki/Zork_I)
* [The Digital Antiquarian](https://www.filfre.net/2012/01/selling-zork/)
* [The Interactive Fiction Database](https://ifdb.tads.org/viewgame?id=0dbnusxunq7fw5ro)
* [The Infocom Gallery](http://gallery.guetech.org/zork1/zork1.html)
* [IFWiki](http://www.ifwiki.org/index.php/Zork_I)

## Playing the Game with the Rust Interpreter

This repository includes a Z-machine interpreter written in Rust that can run the compiled Zork I story file on Linux.

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain)
- Linux operating system

### Building the Interpreter

```bash
# Build the release version
cargo build --release

# The binary will be at: target/release/zork-interpreter
```

### Running Zork I

```bash
# Run with the included story file
./target/release/zork-interpreter COMPILED/zork1.z3

# Or specify a different story file
./target/release/zork-interpreter /path/to/story.z3
```

### Game Commands

Common commands you can use in the game:
- **Movement**: `north`, `south`, `east`, `west`, `up`, `down` (or `n`, `s`, `e`, `w`, `u`, `d`)
- **Look around**: `look`
- **Examine objects**: `examine <object>` or `x <object>`
- **Inventory**: `inventory` or `i`
- **Take/Drop**: `take <object>`, `drop <object>`
- **Open/Close**: `open <object>`, `close <object>`
- **Save/Restore**: `save`, `restore`
- **Quit**: `quit`

### About the Interpreter

The interpreter supports Z-machine version 3 story files, which includes:
- Zork I, II, and III
- Other classic Infocom games from the early 1980s

Features:
- Full Z-machine version 3 opcode support
- Text encoding/decoding (Z-strings)
- Object tree manipulation
- Input parsing and tokenization
- Status line display

---

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
