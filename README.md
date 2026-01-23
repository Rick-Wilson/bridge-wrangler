# bridge-wrangler

CLI tool for operations on bridge PBN (Portable Bridge Notation) files.

## Installation

```bash
cargo build --release
```

The binary will be at `target/release/bridge-wrangler`.

## Commands

### rotate-deals

Rotate deals to set the dealer (or declarer) according to a repeating pattern. This is useful for creating practice sets where a specific player should be dealer for each board.

```bash
bridge-wrangler rotate-deals --input <FILE> [OPTIONS]
```

#### Options

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--input <FILE>` | `-i` | Input PBN file (required) | - |
| `--output <FILE>` | `-o` | Output PBN file (not used with multi-pattern) | `<input> - <PATTERN>.pbn` |
| `--pattern <PATTERN>` | `-p` | Rotation pattern(s), comma-separated (see below) | `NESW` |
| `--basis <BASIS>` | `-b` | How to determine current orientation | `standard` |
| `--standard-vul` | - | Use standard vulnerability by board number | off |

#### Patterns

The pattern specifies the target dealer for each board, cycling through the pattern as needed:

- `N` - All boards dealer is North
- `S` - All boards dealer is South
- `NS` - Board 1 North, Board 2 South, Board 3 North, etc.
- `NESW` - Standard rotation: Board 1 North, Board 2 East, Board 3 South, Board 4 West, then repeats

**Multiple patterns**: Use commas to generate multiple output files in one run:
```bash
bridge-wrangler rotate-deals -i deals.pbn -p "S,NS,NESW"
# Creates: deals - S.pbn, deals - NS.pbn, deals - NESW.pbn
```

#### Basis Options

The basis determines how the tool identifies the current orientation of each board:

- `standard` - Priority: RotationBasis tag > Student tag > Declarer > Dealer (default, matches Bridge Composer)
- `basis-tag` - Use the RotationBasis PBN tag
- `student` - Use the Student tag
- `declarer` - Use the Declarer tag
- `dealer` - Use the Dealer tag
- `deal` - Use the Deal tag's first character (starting seat)
- `north` - Assume all boards are oriented to North
- `south` - Assume all boards are oriented to South
- `east` - Assume all boards are oriented to East
- `west` - Assume all boards are oriented to West

#### Examples

Rotate all boards so South is dealer:
```bash
bridge-wrangler rotate-deals -i practice.pbn -p S
```

Create a set where boards alternate between North and South dealer:
```bash
bridge-wrangler rotate-deals -i hands.pbn -p NS -o hands-ns.pbn
```

Generate multiple rotations at once:
```bash
bridge-wrangler rotate-deals -i lesson.pbn -p "S,NS,NES,NESW"
```

Rotate boards assuming they're all currently oriented to North:
```bash
bridge-wrangler rotate-deals -i deals.pbn -p NESW -b north
```

#### What Gets Rotated

- **Dealer** - Rotated to match the target direction
- **Vulnerable** - Swapped between NS/EW for odd rotations (or set to standard if `--standard-vul`)
- **Deal** - Hands are moved around the table to match the new orientation
- **Declarer** - Rotated to match the new orientation
- **Auction** - Starting seat rotated
- **Play** - Opening leader rotated
- **Score** - NS/EW prefix swapped for odd rotations
- **Commentary** - Direction words (North, South, East, West) rotated in text

## Dependencies

This tool uses [bridge-parsers](../Bridge-Parsers) as a local dependency for PBN parsing.

## License

Unlicense
