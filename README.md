# Placement Legalization

A library for placement legalization, in Rust.  Designed to splice into fsdc_r.

Two different methods:

## Tetris
A series of blocks (rectangles, with preferred XY coordinates)
are supplied, along with the legalization area.

We sort the blocks by their X position, and then "drop" them
to the left, one at a time, sort of like Tetris, but playing
to the left side of the screen.  Blocks go to the spot that
is nearest to their desired position -- so they might
move up or down a bit, to get to a further leftward spot.

## HCwT

Dynamic programming approach, using the HCwT library.

All of the blocks are placed in a heap, ordered by the Y
position.  Then, select twice the row length worth of blocks,
and then put them into two rows, ferry-loading style.

The selected blocks are ordered by their X position, and then
each block has a "decision" to be in either the top or
bottom of a pair of rows.

The HCwT library uses "generate" and "filter" call-back
functions.  If the two rows have total usage of R1 and R2,
then the "generate" will add a cell to each row, creating two
new partial solutions, with lengths (R1 + W, R2), and
(R1, R2 + W).

The "cost" of any intermediate solution should be the
square of the total displacement (there's a good reason
to use the square function, discussion is outside the
scope of the readme).

The filter function will eliminate solutions where the
row lengths are dramatically different -- the cells are
going to be relatively uniformly distributed, so they should
go in at roughly even pace on both rows.  When the
row lengths for a configuration are equal, eliminate
solutions with higher cost.

After filling in a pair of rows, the top row order is
discarded (and the cells go back into the heap), while
the lower row is "fixed"

## Calling and Return Values

Functions are called with a LegalProblem, that
provides the blocks to legalize, and the XY location
to start the legalization (as well as the number of
rows, the number of columns, and the spacing).

