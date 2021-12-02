( Advent of code 2021 - day 1, part 2 )
( https://adventofcode.com/2021/day/1 )
( run with the following command: )
( cargo run -- run forth-examples/aoc-2021/12-01-pt2.fth )

( push all of the test data onto the stack, with the length at the top )
: ex_data 199 200 208 210 200 207 240 269 260 263 10 ;

: 3dup 2 pick 2 pick 2 pick ;
: 3sum + + ;

( Move the counter to the return stack, then check the next      )
( two items, increment the counter if the values were increasing )
( i0 i1 n -- n )
: incr_on_bigger >r < if 1 else 0 then r> + ;

( load data )
ex_data

( "fix" count, since we have no comparison for the last items )
-3 +

( Place an "is bigger" counter under the array length )
>r
0
r>

( loop through all items )
0 do >r 3dup 3sum >r drop 3dup 3sum r> r> incr_on_bigger loop

( swap the last item and the bigger count, then pop the last item )
>r
drop drop drop
r>

( print the final result )
.
