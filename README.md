# Forth Hax

## Goal

I want to be able to make a simple scripting language, suitable for writing simple + portable device drivers.

## Fake Example

Assume the following builtins:

TODO: How to do byte arrays? For now, assume only the lowest
byte of each stack word is used

* I2C_WRITE
    * Initial Stack (top down)
        * (I2C Addr - 7 bit)
        * (N - Num bytes to send)
        * N bytes
    * Final Stack
        * bool - 1 => Success, 0 => Failure
* I2C_READ
    * Initial Stack (top down)
        * (I2C Addr - 7 bit)
        * (N - Num bytes to RX)
    * Final Stack
        * bool - 1 => Success, 0 => Failure
        * If success:
            * (N - Num bytes RX'd)
            * N bytes
* I2C_WRITE_READ
    * Initial Stack (top down)
        * (I2C Addr - 7 bit)
        * (N - Num bytes to send)
        * N bytes
        * (N - Num bytes to RX)
    * Final Stack
        * bool - 1 => Success, 0 => Failure
        * If success:
            * (N - Num bytes RX'd)
            * N bytes

## TMP 117 example

```forth
( Addr: 0b0100_1000, Register: 0x00 )

: READ_TEMP
    0x02 ( RX 2 Bytes )
    0x00 ( Register Addr )
    0x01 ( Write one byte )
    0x48 ( I2C Addr )
    I2C_WRITE_READ
;

: READ_DEVID
    0x02 ( RX 2 Bytes )
    0x0F ( Register Addr )
    0x01 ( Write one byte )
    0x48 ( I2C Addr )
    I2C_WRITE_READ
;

( todo - check result )
( todo - stack words to bytes, interpreting data, sending, receiving commands )
```


# License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)

- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
