# Fuzzing

Use nightly Rust and cargo-fuzz; fuzzing is separate from workspace tests.

```console
cargo +nightly fuzz run parse_svd ../tests/corpus -- -max_len=65536
cargo +nightly fuzz run normalize ../tests/corpus -- -max_len=65536
cargo +nightly fuzz run analyze_svd ../tests/corpus -- -max_len=65536
```

Targets accept UTF-8 inputs up to 64 KiB. Invalid UTF-8 belongs to CLI I/O tests.
Normalization and full analysis use a 1000-instance materialization budget.
The frontend bounds XML element depth at 64; normalization also bounds dependency
depth at 64 and walks direct derivedFrom chains iteratively.
A panic is a failure; targets do not catch or conceal it. Minimized failures must
be added to tests/corpus/regression with a cause and expected behavior.
No fuzz run duration or performance threshold is imposed on ordinary CI.

