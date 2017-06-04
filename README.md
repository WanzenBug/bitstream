# bitstream-rs

Rust crate for reading and writing single bit values from ordinary Readers and Writers

## Usage
Add this library to your dependencies in your `Cargo.toml`

```TOML
[dependencies]
bitstream-rs = "0.2.0"
```

Then import it in your source code
```Rust
extern crate bitstream;
```

You can now use the `BitReader` and `BitWriter`
```Rust
let mut writer = BitWriter::new(outfile);
let mut reader = BitReader::new(infile);
```

For more information, take a look at the [docs](https://docs.rs/bitstream-rs/)
