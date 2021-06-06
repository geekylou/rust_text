# rust_text
RustTex is a program to try to recreate what the Internet would of been like if it had been masively popular and widely available in the 80s.  RustTex provides telnet like (and maybe in the fuure serial) interface which can be used to view web pages and connect to machines using the Teletext character set (referred to as mode 7 in BBC micro documentation).  This can be paired with a BBC emulator running BeebEm and tcpser.
For background see my blog https://blog.geekylou.me.uk/?p=425.

## Building
To build use the standard rust cargo build system as follows:
cargo build:

cargo build

Or for release builds:

cargo build --release

This will produce a executable in target which can be run as follows:

rust_text

## Config

There isn't currently anything to configure!
