# forbidden-bands
8-bit string handling library
![build](https://github.com/jgerrish/forbidden-bands/actions/workflows/rust.yml/badge.svg)

# Introduction

This is a collection of utilities and structures for dealing with
different 8-bit strings.  Basically it provides easy ways to
converting strings to and from Rust string types and debugging those
strings.

It depends on the standard library and core.  It's not meant for
embedded systems.  The goal here is reading and writing old
filesystems on newer systems.  But it can be used for other projects.

Currently it supports fixed-length PETSCII strings.  PETSCII is the
character set used on early Commodore Business Machines systems.

# Examples

To convert a PETSCII string to a Unicode string on the command line:

echo -n -e "\x0eABCD\x8e" | cargo run --example petscii_to_unicode

# Contributing

Other 8-bit string support is welcome.  Some other string types may be
added in the future.


# References

[CodeCharts.pdf](https://www.unicode.org/Public/15.1.0/charts/  "The Unicode Standard, Version 15.1, Archived Code Charts")
The Unicode Standard, Version 15.1
Archived Code Charts
This file contains the complete set of character code tables and list
of character names for The Unicode Standard, Version 15.1

[Commodore_64_Programmer's_Reference_Guide_1983_Commodore.pdf](https://archive.org/details/commodore-64-programmers-reference-guide_202205 "Commodore 64 Programmer's Reference Guide")
Commodore 64 Programmer's Reference Guide
Appendix B contains Screen Display Codes and Appendix C contains ASCII
and CHR$ (PETSCII) Codes

[petscii.pdf](http://www.aivosto.com/ "Commodore PETSCII character sets")
This document by Aivosto Oy is another great resource for
understanding Commodore character sets and has an intuitive
informtaion design that nicely complements the Commodore Programmer's
Reference Guide.

[18235-aux-LegacyComputingSources.pdf](https://www.unicode.org/L2/L2018/18235-aux-LegacyComputeringSources.pdf)
An official Unicode document with mappings between several legacy
computer systems, including 8-bit systems such as the Commodore
C64/C128, and Unicode block characters.


[18235-terminals-prop.pdf](https://www.unicode.org/L2/L2018/18235-terminals-prop.pdf)
Proposal to add characters from legacy computers and teletext to the UCS
Universal Multiple-Octet Coded Character Set International Organization for Standardization


[19025-terminals-prop.pdf](https://www.unicode.org/L2/L2019/19025-terminals-prop.pdf)
Proposal to add characters from legacy computers and teletext to the UCS
Universal Multiple-Octet Coded Character Set International Organization for Standardization
