( Advent of code 2021 - day 1, part 1 )
( https://adventofcode.com/2021/day/1 )
( run with the following command: )
( cargo run -- run forth-examples/aoc-2021/12-01-pt2.fth )

( push all of the test data onto the stack, with the length at the top )
: ex_data 199 200 208 210 200 207 240 269 260 263 10 ;

( Move the counter to the return stack, then check the next      )
( two items, increment the counter if the values were increasing )
( i0 i1 n -- n )
: incr_on_bigger >r < if 1 else 0 then r> + ;

( Duplicate the OLDEST item on the list, as we will )
( need to keep it around for the NEXT comparison    )
( i0 i1 n -- i0 i0 i1 n )
: duplicate_second_item >r >r dup r> r> ;

( load data )
ex_data

( "fix" count, since we have no comparison for the last item )
-1 +

( Place an "is bigger" counter under the array length )
>r
0
r>

( loop through all items )
0 do duplicate_second_item incr_on_bigger loop

( swap the last item and the bigger count, then pop the last item )
swap
drop

( print the final result )
.
