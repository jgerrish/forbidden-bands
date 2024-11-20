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
