# This Week in Rust #645

**seino\_chan**

2026-04-02T01:41:06+00:00

## Comments

### duongdominhchau

2026-04-02T04:38:50+00:00

[blogr v0.5.0 - blog without from your terminal](https://github.com/bahdotsh/blogr/releases/tag/v0.5.0)

Feels like a word is missing

### seino\_chan

2026-04-02T04:47:29+00:00

Fixed! (Thanks to the fantastic community member who opened the pull request!)

### p32blo

2026-04-02T07:13:14+00:00

# TWIR @ Reddit

Hey everyone, here you can follow the [r/rust](/r/rust) comment threads of articles featured in TWIR (This Week in Rust). I've always found it helpful to search for additional insights in the comment section here and I hope you can find it helpful too.

If you are curious how this comment is generated you can check [https://github.com/p32blo/twir-reddit](https://github.com/p32blo/twir-reddit)

Enjoy !

* * *

## Official

*   [Rust 1.94.1 is out](http://www.reddit.com/r/rust/comments/1s470b7/rust_1941_is_out/) `↑383 | 8 comments`

## Project/Tooling Updates

*   [Next target of Ubuntu's oxidization plan will be ntpd-rs](http://www.reddit.com/r/rust/comments/1s43bn4/next_target_of_ubuntus_oxidization_plan_will_be/) `↑207 | 38 comments`
*   [octopos: xv6 based operating system for risc-v in rust](http://www.reddit.com/r/rust/comments/1s3gvdf/octopos_xv6_based_operating_system_for_riscv_in/) `↑23 | 1 comment`
*   [Building a guitar trainer with embedded Rust](http://www.reddit.com/r/rust/comments/1s5vs2w/building_a_guitar_trainer_with_embedded_rust/) `↑74 | 2 comments`
*   [jsongrep is faster than {jq, jmespath, jsonpath-rust, jql}](http://www.reddit.com/r/rust/comments/1rzxjv3/jsongrep_is_faster_than_jq_jmespath_jsonpathrust/) `↑106 | 33 comments`

## Observations/Thoughts

*   [Breaking The AI Infra Monopoly With Rust- Tracel AI](http://www.reddit.com/r/rust/comments/1s935kk/breaking_the_ai_infra_monopoly_with_rust_tracel_ai/) `↑3 | 10 comments`
*   [Rust memory safety in kernel space (osdev)](http://www.reddit.com/r/rust/comments/1s8hc6h/rust_memory_safety_in_kernel_space_osdev/) `↑46 | 6 comments`
*   [Fixing our own problems in the Rust compiler](http://www.reddit.com/r/rust/comments/1s7pvpg/fixing_our_own_problems_in_the_rust_compiler/) `↑363 | 10 comments`
*   [How C++ Finally Beats Rust at JSON Serialization - Daniel Lemire & Francisco Geiman Thiesen](http://www.reddit.com/r/rust/comments/1s2pf1z/how_c_finally_beats_rust_at_json_serialization/) `↑191 | 46 comments`

## Rust Walkthroughs

*   [Adding WASM Plugins to Your App - Using Wasmi as a runtime and Zola as an example.](http://www.reddit.com/r/rust/comments/1s9tuad/adding_wasm_plugins_to_your_app_using_wasmi_as_a/) `↑1 | 0 comment`

### matthieum

2026-04-02T16:49:48+00:00

> but one solid step everyone agrees on is making it (a) more obvious when you are sharing two handles to the same object vs doing a deep clone, via the Share trait, and (b) more ergonomic to capture clones into closures and async blocks with `move($expr)` expressions

I agree with `Share`, but I'm surprised to see an inline `move($expr)` in there.

All the discussions I've seen were of the `async move(t = $expr) { ...; foo(t); }` variety, not with `move($expr)` within the block/closure itself.

### danilo-developer

2026-04-05T15:12:09+00:00

I want to share that I'm working on the development of an Authentication service that I plan to open source, but I'm still designing many things. The core idea is implement something similar to Keycloak, but you minimum memory footprint, designed for High throughput , using crypto staff , PostgreSQL and Valkey!

For now I've published two crates extracted from the main project.

A secrets rotator and syncer, designed to allow aggressive secret rotation mainly for my PASETO and ALTCHA(PoW) secrets

[https://crates.io/crates/secret-manager](https://crates.io/crates/secret-manager)

And also a helper crate for simplify test containers with reuse by name usage for Postgresql(with migrations applied) and Valkey/Redis (clean state per test within a single container)

[https://crates.io/crates/test-containers-util](https://crates.io/crates/test-containers-util)

(I'm currently open to work and seeking Rust positions)