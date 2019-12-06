# Spaceships

Search for spaceships in Conway's Game of Life using the [rlifesrc](https://github.com/AlephAlpha/rlifesrc/tree/master/lib) lib.

It starts from a given minimum height, and an optional upper bound of the cell count.

When a new result is found, it will reduce the upper bound to the cell count of this result minus 1 (even if there is no
initial upper bound).

When no more result can be found, it will increase the height by 1 and continue the search.

Spaceships with period `p`, speed `(x,y)c/p`, and `n` cells are saved in the file `{n}P{p}H{x}V{y}.rle`.

Press `Ctrl-C` to abort.

See the `b3s23` directory for the search results for Conway's Game of Life. 

## Usage

```
USAGE:
    spaceships [OPTIONS] --dir <dir> --dx <dx> --dy <dy> --period <period>

FLAGS:
        --help
            Prints help information

    -V, --version
            Prints version information


OPTIONS:
    -d, --dir <dir>
            Search results are saved here.

    -x, --dx <dx>
            Horizontal translation.

    -y, --dy <dy>
            Vertical translation.

    -c, --init-cell-count <init-cell-count>
            Initial upper bound of the cell count.

            It will automatically decrease when a new result is found. [default: 0]

    -h, --init-height <init-height>
            Initial height.

            It will automatically increase when no more result can be found. [default: 1]

    -w, --max-width <max-width>
            Maximum width. [default: 1024]

    -p, --period <period>
            Period.

    -r, --rule <rule>
            Rule string. [default: B3/S23]

    -s, --symmetry <symmetry>
            Symmetry. [default: C1]

    -f, --view-freq <view-freq>
            Print the world every this number of steps. [default: 5000000]
```
