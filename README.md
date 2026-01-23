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

### to-pdf

Convert PBN files to PDF format with various layout options.

```bash
bridge-wrangler to-pdf --input <FILE> [OPTIONS]
```

#### Options

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--input <FILE>` | `-i` | Input PBN file (required) | - |
| `--output <FILE>` | `-o` | Output PDF file | `<input>.pdf` |
| `--layout <LAYOUT>` | `-l` | Layout style (see below) | `analysis` |
| `--boards-per-page <N>` | `-b` | Boards per page (1, 2, or 4) | varies by layout |
| `--board-range <RANGE>` | `-r` | Board range to include | all boards |
| `--hide-bidding` | - | Hide bidding information | off |
| `--hide-play` | - | Hide play sequence | off |
| `--hide-commentary` | - | Hide commentary | off |
| `--show-hcp` | - | Show high card points | off |

#### Layouts

- `analysis` - Full hand diagram with bidding table and commentary (default)
- `bidding-sheets` - Simplified layout for practice bidding
- `declarers-plan` - 4 deals per page for declarer's planning practice

#### Board Range

Specify which boards to include using ranges or lists:
- `1-4` - Boards 1 through 4
- `1,3,5` - Boards 1, 3, and 5
- `1-4,7,9-12` - Combination of ranges and individual boards

#### Examples

Convert a PBN file to PDF:
```bash
bridge-wrangler to-pdf -i lesson.pbn
# Creates: lesson.pdf
```

Create PDF with only boards 1-4:
```bash
bridge-wrangler to-pdf -i hands.pbn -r "1-4" -o first-four.pdf
```

Create a declarer's plan practice sheet:
```bash
bridge-wrangler to-pdf -i deals.pbn -l declarers-plan
```

### analyze

Perform double-dummy analysis on deals and optionally add results to PBN files.

```bash
bridge-wrangler analyze --input <FILE> [OPTIONS]
```

#### Options

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--input <FILE>` | `-i` | Input PBN file (required) | - |
| `--output <FILE>` | `-o` | Output PBN file with DD results | - |
| `--board-range <RANGE>` | `-r` | Board range to analyze | all boards |
| `--verbose` | `-v` | Show DD results table and par scores | off |

#### Output

By default, the command runs quietly and only reports progress. Use `-v` to display the DD results table showing tricks for each declarer (N, S, E, W) in each denomination (NT, S, H, D, C):

```
       NT   S   H   D   C
  N     7   6   7   6   6
  S     7   6   7   6   6
  E     6   6   6   6   6
  W     6   6   6   6   6
```

When using `--output`, the results are added to the PBN file as `[OptimumResultTable]` tags.

#### Examples

Analyze all boards (quiet mode):
```bash
bridge-wrangler analyze -i hands.pbn
```

Analyze and display DD results with par scores:
```bash
bridge-wrangler analyze -i hands.pbn -v
```

Analyze and save results to a new PBN file:
```bash
bridge-wrangler analyze -i hands.pbn -o hands-analyzed.pbn
```

Analyze only boards 1-4:
```bash
bridge-wrangler analyze -i hands.pbn -r "1-4" -v
```

### block-replicate

Replicate boards into blocks for multi-table play. This creates copies of the input boards with correct dealer and vulnerability for each board position, adding tracking tags for the original ("virtual") board information.

```bash
bridge-wrangler block-replicate --input <FILE> [OPTIONS]
```

#### Options

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--input <FILE>` | `-i` | Input PBN file (required) | - |
| `--output <FILE>` | `-o` | Output PBN file | `<input> - <B>x<C>.pbn` |
| `--block-size <N>` | `-b` | Number of boards per block | number of input boards |
| `--block-count <N>` | `-c` | Number of blocks to create | fills to 36 boards |
| `--pdf` | | Also generate a PDF hand record | off |

#### How It Works

The command replicates input boards into multiple blocks. Each block contains the same deals as the original, but with:

- **Board numbers** assigned sequentially (1, 2, 3, ...)
- **Dealer** set according to standard pattern (N, E, S, W, repeating)
- **Vulnerability** set according to standard 16-board pattern

The first block preserves the original boards completely (including all commentary). Replicated blocks (2+) contain minimal board data with tracking tags:
- `[VirtualBoard]` - Original board number within the block
- `[VirtualDealer]` - Original dealer for that board position
- `[VirtualVulnerable]` - Original vulnerability for that board position
- `[BlockNumber]` - Which block this board belongs to (1-indexed)

If block_size exceeds the number of input boards, filler deals are used (each player gets all 13 cards of one suit).

#### Examples

Replicate 8 boards into 4 blocks (32 total boards):
```bash
bridge-wrangler block-replicate -i session.pbn
# With 8 input boards: creates 4 blocks of 8 = 32 boards
# Output: session - 8x4.pbn
```

Create a specific number of blocks:
```bash
bridge-wrangler block-replicate -i hands.pbn -c 6
# Creates 6 blocks
```

Create blocks with a specific size:
```bash
bridge-wrangler block-replicate -i hands.pbn -b 9 -c 4
# Creates 4 blocks of 9 boards = 36 total
```

Specify output file:
```bash
bridge-wrangler block-replicate -i deals.pbn -o tournament.pbn
```

Generate PBN and PDF for dealing machines:
```bash
bridge-wrangler block-replicate -i lesson.pbn --pdf
# Creates: lesson - 4x9.pbn and lesson - 4x9.pdf
```

## Dependencies

This tool uses:
- [bridge-parsers](https://github.com/Rick-Wilson/Bridge-Parsers) - PBN parsing
- [pbn-to-pdf](https://github.com/Rick-Wilson/pbn-to-pdf) - PDF generation
- [bridge-solver](https://github.com/Rick-Wilson/Dealer3) - Double-dummy analysis

## License

Unlicense
