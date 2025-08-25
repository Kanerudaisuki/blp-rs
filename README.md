# blp-rs

This is a pure Rust take on the classic Warcraft III BLP texture format.  
No C glue, no old-school wrappers — just clean Rust that works everywhere: Windows, macOS, Linux.

It has a tiny UI made with [egui](https://github.com/emilk/egui) — drop in a file, and boom, you can view it.  
Under the hood there's a simple Rust library, perfect when you just need BLP decoding inside your own tools.

Oh, and it's part of the WarRaft toolkit — you’ll also find it used together
with [JASS-Tree-sitter-Rust](https://github.com/WarRaft/JASS-Tree-sitter-Rust),  
which brings syntax support, analyzers, and more tooling for Warcraft III modding.

Grab binaries here:  
👉 [Download](https://github.com/WarRaft/blp-rs/tree/main/bin)

Wanna know how BLP works? Dive into the spec:  
👉 [BLP Specification](https://github.com/WarRaft/BLP)

---

### What’s still missing

- Converting images *back* into BLP
- Proper BLP2 support
- CLI tool (not implemented yet)

<p align="center">
  <img src="https://raw.githubusercontent.com/WarRaft/blp-rs/refs/heads/main/preview/logo.png" alt="BLP"/>
</p>