# Novaprox

This repository contains free v2ray (xray) configs that are 99.9% alive and no ded.

## Guide
Default subs from github ci/cd (auto proxy filtering & posting in repo) not good.
You may live in different coutry with different censor and ping, so better run yourself.
For run you need download zip from releases, unpack, and start program, but firstly you need install xray from xtls and place in path.
After running, you get out.txt. Copy it & use)

If not works or no zip in releases you can `git clone` and `cargo run --release`.

## Subscribitions
Primary:
```
https://raw.githubusercontent.com/suprohub/novaprox/refs/heads/main/sub/primary.txt
```

All:
```
https://raw.githubusercontent.com/suprohub/novaprox/refs/heads/main/sub/vless.txt
```

## License
Licensed under either of

 - Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
 - MIT license
   ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution
Before contribution you need to run `cargo clippy --all-features --fix`, `typos` and `cargo fmt`.
For installing `typos` run `cargo binstall typos-cli` (or `cargo install typos-cli`).

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.