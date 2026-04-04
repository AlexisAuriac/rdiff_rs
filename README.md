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
- Command text output, this includes help messages, error messages, or logging

## CLI differences

This is meant to be an *exhaustive* list of differences between rdiff-rs and the original rdiff.

This section is meant for anyone who would like to migrate from rdiff to rdiff-rs, or just to have a comparison of options.

Text output is different, this includes help messages, error messages, and logging.

### CLI options differences

For reference, here is the help message for rdiff:
```
Usage: rdiff [OPTIONS] signature [BASIS [SIGNATURE]]
             [OPTIONS] delta SIGNATURE [NEWFILE [DELTA]]
             [OPTIONS] patch BASIS [DELTA [NEWFILE]]

Options:
  -v, --verbose             Trace internal processing
  -V, --version             Show program version
  -?, --help                Show this help message
  -s, --statistics          Show performance statistics
  -f, --force               Force overwriting existing files
Signature generation options:
  -H, --hash=ALG            Hash algorithm: blake2 (default), md4
  -R, --rollsum=ALG         Rollsum algorithm: rabinkarp (default), rollsum
Delta-encoding options:
  -b, --block-size=BYTES    Signature block size, 0 (default) for recommended
  -S, --sum-size=BYTES      Signature strength, 0 (default) for max, -1 for min
IO options:
  -I, --input-size=BYTES    Input buffer size
  -O, --output-size=BYTES   Output buffer size
```

rdiff-rs has additional `help` subcommand.

rdiff allows options before subcommand, rdiff-rs does not.
For example:
```bash
# valid
rdiff signature -f file file.sig
rdiff-rs signature -f file file.sig

# valid for rdiff, invalid for rdiff-rs
rdiff signature -f file file.sig
rdiff-rs -f signature file file.sig # error: unexpected argument '-f' found
```

rdiff-rs is missing some options:
- `-v/--verbose` and `-s/--statistics`: I have not looked much into logging so far
- `-O/--output-size`: while `-I/--input-size` has a huge impact on many parts of the program `--output-size` only serves to optimize the size of some write buffers, not very important afaik, it may be implemented in the future

rdiff allows combining any option with any subcommand, regardless of the fact that many options only affect `signature`. While this is not overtly harmful, it can be misleading.
- delta looses the `-H/--hash`, `-R/--rollsum`, `-b/--block-size`, and `-S/--sum-size` options
- patch all of those plus `-I/--input-size`

## Acknowledgments

The first version was actually a direct rewrite of [librsync-go](https://github.com/balena-os/librsync-go).
