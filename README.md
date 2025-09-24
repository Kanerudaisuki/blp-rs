# blp

This is a pure Rust take on the classic Warcraft III BLP texture format.  
No C glue, no old-school wrappers — just clean Rust that works everywhere: Windows, macOS, Linux.

It has a tiny UI made with [egui](https://github.com/emilk/egui) — drop in a file, and boom, you can view it.  
Under the hood there's a simple Rust library, perfect when you just need BLP decoding inside your own tools.

Oh, and it's part of the WarRaft toolkit — you’ll also find it used together
with [JASS-Tree-sitter-Rust](https://github.com/WarRaft/JASS-Tree-sitter-Rust),  
which brings syntax support, analyzers, and more tooling for Warcraft III modding.

Wanna know how BLP works? Dive into the spec:  
👉 [BLP Specification](https://github.com/WarRaft/BLP)

## Localization

All localization files are stored in [assets/locales](https://github.com/WarRaft/blp-rs/tree/main/assets/locales).  
You are welcome to contribute a translation in your own language using whatever workflow is most convenient for you, and
I will include it in the program.

It is **not required** to translate every key: any missing strings will automatically fall back to the default English (
`en`) localization. This means you can start small and expand the translation over time without breaking anything.


<p align="center">
  <img src="https://raw.githubusercontent.com/WarRaft/blp/refs/heads/main/preview/logo.png" alt="BLP"/>
</p>