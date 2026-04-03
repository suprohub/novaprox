# Novaprox

This repository contains free v2ray (xray) configs that are 99.9% alive and no dead.

## Guide
Default subs from github ci/cd (auto proxy filtering & posting in repo) not good.
You may live in different country with different censor and ping, so better run yourself.
For run you need download zip from releases, unpack, and start program, but firstly you need install xray from xtls and place in path.
After running, you get out.txt. Copy it & use)

If not works or no zip in releases you can `git clone` and `cargo run --release`.

## Sub
Main page: https://github.com/suprohub/normal-ethernet

Sub: https://github.com/suprohub/normal-ethernet/blob/main/vless.txt

Guide: https://telegra.ph/Vless---Android-11-29

## Compiling
`git clone` and `cargo run --release`

## Docker
We also have Dockerfile! Dockerfile is in docker/
You can also run this on mikrotik router.
Approximate commands for setup on mikrotik can be found at file docker/mikrotik.rsc

## Contribution
Before contribution you need to run `cargo clippy --all-features --fix`, `typos` and `cargo fmt`.
For installing `typos` run `cargo binstall typos-cli` (or `cargo install typos-cli`).

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.

## License
Licensed under either of

 - Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
 - MIT license
   ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
