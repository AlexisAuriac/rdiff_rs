# rdiff_rs

A pure rust rewrite of the [rdiff/librsync](https://github.com/librsync/librsync) project.

I made this project to get a better understanding of rsync, it comes with no guarantees.

If you are looking for Rust bindings for librsync, [librsync-rs](https://github.com/mbrt/librsync-rs) got you covered.

## Objectives

Broadly, the objective is to be as compatible as reasonable with the original rdiff.

- Complete compatibility with the file formats
- Near-complete compatibility with the CLI
- Equal or better performance

## Non-objectives

- Async support, it would require rethinking most of the project, I think at this point it might be better to try a different approach altogether
- Complete compatibility with the CLI options, I personally find a few option combinations or values to be useless or counter intuitive, those have been dropped or replaced
- Command text output, this includes help messages, debug messages, or logging

## CLI differences

This is an *exhaustive* list of differences between rdiff-rs and the original rdiff.

TODO

## Acknowledgments

The first version was actually a direct rewrite of [librsync-go](https://github.com/balena-os/librsync-go).
