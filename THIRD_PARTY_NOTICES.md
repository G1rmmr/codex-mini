# Third-party notices

Codex Mini is MIT-licensed. Dependencies keep their own licenses.

This inventory comes from `Cargo.lock` and Cargo package metadata for the Linux x86_64 dependency graph. It includes optional, build and test packages; not every row is linked into the default executable. No dependency source or prebuilt executable is vendored. Release archives also contain `THIRD_PARTY_LICENSES.html`, generated from the Linux, Windows and macOS dependency graphs. That file carries the full dependency license texts for the shipped binaries.

## Direct dependencies

| Package | Version | License | Use |
| --- | --- | --- | --- |
| [eframe](https://github.com/emilk/egui/tree/main/crates/eframe) | 0.35.0 | MIT OR Apache-2.0 | Window, rendering, egui |
| [egui_kittest](https://github.com/emilk/egui) | 0.35.0 | MIT OR Apache-2.0 | UI tests only |
| [windows-sys](https://github.com/microsoft/windows-rs) | 0.61.2 | MIT OR Apache-2.0 | Windows process management |
| [libc](https://github.com/rust-lang/libc) | 0.2.190 | MIT OR Apache-2.0 | Process groups |
| [serde](https://github.com/serde-rs/serde) | 1.0.229 | MIT OR Apache-2.0 | Settings serialization |
| [serde_json](https://github.com/serde-rs/json) | 1.0.154 | MIT OR Apache-2.0 | App-server protocol |
| [toml_edit](https://github.com/toml-rs/toml) | 0.22.27 | MIT OR Apache-2.0 | Codex configuration editing |

## Fonts

`epaint_default_fonts` supplies the fonts embedded by egui. Its license expression is `(MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0`. The upstream font notices are reproduced in [OFL.txt](licenses/egui-fonts-OFL.txt) and [UFL.txt](licenses/egui-fonts-UFL.txt). Additional Korean fonts are loaded from the user’s system or `CODEX_MINI_FONT`; they are not distributed here.

## Optional RTK executable

The installer downloads the unmodified OSS build of [RTK 1.2.0](https://github.com/rtk-ai/rtk/releases/tag/v1.2.0). Apache-2.0. [License](licenses/RTK-1.2.0-APACHE-2.0.txt). RTK is a separate process and is not required to build Mini. The binary stays in the ignored `dist/` directory.

## Resolved packages

License expressions below are reproduced from package metadata. Upstream projects retain their copyrights. Consult the linked source before redistributing a binary or selecting a different target or feature set.

| Package | Version | License |
| --- | --- | --- |
| [accesskit](https://github.com/AccessKit/accesskit) | 0.24.1 | MIT OR Apache-2.0 |
| [accesskit_consumer](https://github.com/AccessKit/accesskit) | 0.35.0 | MIT OR Apache-2.0 |
| [adler2](https://github.com/oyvindln/adler2) | 2.0.1 | 0BSD OR MIT OR Apache-2.0 |
| [ahash](https://github.com/tkaitchuck/ahash) | 0.8.12 | MIT OR Apache-2.0 |
| [arboard](https://github.com/1Password/arboard) | 3.6.1 | MIT OR Apache-2.0 |
| [arrayvec](https://github.com/bluss/arrayvec) | 0.7.8 | MIT OR Apache-2.0 |
| [as-raw-xcb-connection](https://github.com/psychon/as-raw-xcb-connection) | 1.0.1 | MIT OR Apache-2.0 |
| [autocfg](https://github.com/cuviper/autocfg) | 1.5.1 | Apache-2.0 OR MIT |
| [bit-set](https://github.com/contain-rs/bit-set) | 0.9.1 | Apache-2.0 OR MIT |
| [bit-vec](https://github.com/contain-rs/bit-vec) | 0.9.1 | Apache-2.0 OR MIT |
| [bitflags](https://github.com/bitflags/bitflags) | 2.13.2 | MIT OR Apache-2.0 |
| [bytemuck](https://github.com/Lokathor/bytemuck) | 1.25.2 | Zlib OR Apache-2.0 OR MIT |
| [bytemuck_derive](https://github.com/Lokathor/bytemuck) | 1.11.0 | Zlib OR Apache-2.0 OR MIT |
| [byteorder-lite](https://github.com/image-rs/byteorder-lite) | 0.1.0 | Unlicense OR MIT |
| [calloop](https://github.com/Smithay/calloop) | 0.13.0 | MIT |
| [calloop](https://github.com/Smithay/calloop) | 0.14.4 | MIT |
| [calloop-wayland-source](https://github.com/smithay/calloop-wayland-source) | 0.4.1 | MIT |
| [cc](https://github.com/rust-lang/cc-rs) | 1.8.0 | MIT OR Apache-2.0 |
| [cfg-if](https://github.com/rust-lang/cfg-if) | 1.0.5 | MIT OR Apache-2.0 |
| [cfg_aliases](https://github.com/katharostech/cfg_aliases) | 0.2.2 | MIT |
| [codespan-reporting](https://github.com/brendanzab/codespan) | 0.13.1 | Apache-2.0 |
| [color](https://github.com/linebender/color) | 0.3.3 | Apache-2.0 OR MIT |
| [crc32fast](https://github.com/srijs/rust-crc32fast) | 1.5.2 | MIT OR Apache-2.0 |
| [cursor-icon](https://github.com/rust-windowing/cursor-icon) | 1.2.0 | MIT OR Apache-2.0 OR Zlib |
| [displaydoc](https://github.com/yaahc/displaydoc) | 0.2.7 | MIT OR Apache-2.0 |
| [dlib](https://github.com/elinorbgr/dlib) | 0.5.3 | MIT |
| [document-features](https://github.com/slint-ui/document-features) | 0.2.12 | MIT OR Apache-2.0 |
| [downcast-rs](https://github.com/marcianx/downcast-rs) | 1.2.1 | MIT/Apache-2.0 |
| [dpi](https://github.com/rust-windowing/winit) | 0.1.2 | Apache-2.0 AND MIT |
| [ecolor](https://github.com/emilk/egui) | 0.35.0 | MIT OR Apache-2.0 |
| [eframe](https://github.com/emilk/egui/tree/main/crates/eframe) | 0.35.0 | MIT OR Apache-2.0 |
| [egui](https://github.com/emilk/egui) | 0.35.0 | MIT OR Apache-2.0 |
| [egui-wgpu](https://github.com/emilk/egui/tree/main/crates/egui-wgpu) | 0.35.0 | MIT OR Apache-2.0 |
| [egui-winit](https://github.com/emilk/egui/tree/main/crates/egui-winit) | 0.35.0 | MIT OR Apache-2.0 |
| [egui_glow](https://github.com/emilk/egui/tree/main/crates/egui_glow) | 0.35.0 | MIT OR Apache-2.0 |
| [egui_kittest](https://github.com/emilk/egui) | 0.35.0 | MIT OR Apache-2.0 |
| [either](https://github.com/rayon-rs/either) | 1.17.0 | MIT OR Apache-2.0 |
| [emath](https://github.com/emilk/egui/tree/main/crates/emath) | 0.35.0 | MIT OR Apache-2.0 |
| [epaint](https://github.com/emilk/egui/tree/main/crates/epaint) | 0.35.0 | MIT OR Apache-2.0 |
| [epaint_default_fonts](https://github.com/emilk/egui/tree/main/crates/epaint_default_fonts) | 0.35.0 | (MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0 |
| [equivalent](https://github.com/indexmap-rs/equivalent) | 1.0.3 | Apache-2.0 OR MIT |
| [errno](https://github.com/lambda-fairy/rust-errno) | 0.3.14 | MIT OR Apache-2.0 |
| [euclid](https://github.com/servo/euclid) | 0.22.14 | MIT OR Apache-2.0 |
| [fax](https://github.com/pdf-rs/fax) | 0.2.7 | MIT |
| [fdeflate](https://github.com/image-rs/fdeflate) | 0.3.7 | MIT OR Apache-2.0 |
| [fearless_simd](https://github.com/linebender/fearless_simd) | 0.4.1 | Apache-2.0 OR MIT |
| [find-msvc-tools](https://github.com/rust-lang/cc-rs) | 0.1.14 | MIT OR Apache-2.0 |
| [flate2](https://github.com/rust-lang/flate2-rs) | 1.1.10 | MIT OR Apache-2.0 |
| [foldhash](https://github.com/orlp/foldhash) | 0.2.0 | Zlib |
| [font-types](https://github.com/googlefonts/fontations) | 0.11.3 | MIT OR Apache-2.0 |
| [form_urlencoded](https://github.com/servo/rust-url) | 1.2.2 | MIT OR Apache-2.0 |
| [gethostname](https://codeberg.org/swsnr/gethostname.rs.git) | 1.1.0 | Apache-2.0 |
| [gl_generator](https://github.com/brendanzab/gl-rs/) | 0.14.0 | Apache-2.0 |
| [glifo](https://github.com/linebender/vello) | 0.1.1 | Apache-2.0 OR MIT |
| [glow](https://github.com/grovesNL/glow) | 0.17.0 | MIT OR Apache-2.0 OR Zlib |
| [glutin](https://github.com/rust-windowing/glutin) | 0.32.3 | Apache-2.0 |
| [glutin-winit](https://github.com/rust-windowing/glutin) | 0.5.0 | MIT |
| [glutin_egl_sys](https://github.com/rust-windowing/glutin) | 0.7.1 | Apache-2.0 |
| [glutin_glx_sys](https://github.com/rust-windowing/glutin) | 0.6.1 | Apache-2.0 |
| [guillotiere](https://github.com/nical/guillotiere) | 0.7.0 | MIT/Apache-2.0 |
| [half](https://github.com/VoidStarKat/half-rs) | 2.7.1 | MIT OR Apache-2.0 |
| [harfrust](https://github.com/harfbuzz/harfrust) | 0.7.0 | MIT |
| [hashbrown](https://github.com/rust-lang/hashbrown) | 0.16.1 | MIT OR Apache-2.0 |
| [hashbrown](https://github.com/rust-lang/hashbrown) | 0.17.1 | MIT OR Apache-2.0 |
| [hexf-parse](https://github.com/lifthrasiir/hexf) | 0.2.1 | CC0-1.0 |
| [icu_collections](https://github.com/unicode-org/icu4x) | 2.3.0 | Unicode-3.0 |
| [icu_locale_core](https://github.com/unicode-org/icu4x) | 2.3.0 | Unicode-3.0 |
| [icu_normalizer](https://github.com/unicode-org/icu4x) | 2.3.0 | Unicode-3.0 |
| [icu_normalizer_data](https://github.com/unicode-org/icu4x) | 2.3.0 | Unicode-3.0 |
| [icu_properties](https://github.com/unicode-org/icu4x) | 2.3.0 | Unicode-3.0 |
| [icu_properties_data](https://github.com/unicode-org/icu4x) | 2.3.0 | Unicode-3.0 |
| [icu_provider](https://github.com/unicode-org/icu4x) | 2.3.1 | Unicode-3.0 |
| [idna](https://github.com/servo/rust-url/) | 1.1.0 | MIT OR Apache-2.0 |
| [idna_adapter](https://github.com/hsivonen/idna_adapter) | 1.2.2 | Apache-2.0 OR MIT |
| [image](https://github.com/image-rs/image) | 0.25.10 | MIT OR Apache-2.0 |
| [indexmap](https://github.com/indexmap-rs/indexmap) | 2.14.2 | Apache-2.0 OR MIT |
| [itertools](https://github.com/rust-itertools/itertools) | 0.14.0 | MIT OR Apache-2.0 |
| [itoa](https://github.com/dtolnay/itoa) | 1.0.18 | MIT OR Apache-2.0 |
| [jobserver](https://github.com/rust-lang/jobserver-rs) | 0.1.35 | MIT OR Apache-2.0 |
| [khronos_api](https://github.com/brendanzab/gl-rs/) | 3.1.0 | Apache-2.0 |
| [kittest](https://github.com/rerun-io/kittest) | 0.4.0 | MIT OR Apache-2.0 |
| [kurbo](https://github.com/linebender/kurbo) | 0.13.1 | Apache-2.0 OR MIT |
| [libc](https://github.com/rust-lang/libc) | 0.2.190 | MIT OR Apache-2.0 |
| [libloading](https://github.com/nagisa/rust_libloading/) | 0.8.9 | ISC |
| [libm](https://github.com/rust-lang/compiler-builtins) | 0.2.16 | MIT |
| [linebender_resource_handle](https://github.com/linebender/raw_resource_handle) | 0.1.1 | Apache-2.0 OR MIT |
| [linux-raw-sys](https://github.com/sunfishcode/linux-raw-sys) | 0.12.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| [linux-raw-sys](https://github.com/sunfishcode/linux-raw-sys) | 0.4.15 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| [litemap](https://github.com/unicode-org/icu4x) | 0.8.3 | Unicode-3.0 |
| [litrs](https://github.com/LukasKalbertodt/litrs) | 1.0.0 | MIT OR Apache-2.0 |
| [lock_api](https://github.com/Amanieu/parking_lot) | 0.4.14 | MIT OR Apache-2.0 |
| [log](https://github.com/rust-lang/log) | 0.4.34 | MIT OR Apache-2.0 |
| [memchr](https://github.com/BurntSushi/memchr) | 2.8.3 | Unlicense OR MIT |
| [memmap2](https://github.com/RazrFalcon/memmap2-rs) | 0.9.11 | MIT OR Apache-2.0 |
| [memoffset](https://github.com/Gilnaa/memoffset) | 0.9.1 | MIT |
| [miniz_oxide](https://github.com/Frommi/miniz_oxide/tree/master/miniz_oxide) | 0.8.9 | MIT OR Zlib OR Apache-2.0 |
| [miniz_oxide](https://github.com/Frommi/miniz_oxide/tree/master/miniz_oxide) | 0.9.1 | MIT OR Zlib OR Apache-2.0 |
| [moxcms](https://github.com/awxkee/moxcms.git) | 0.8.1 | BSD-3-Clause OR Apache-2.0 |
| [naga](https://github.com/gfx-rs/wgpu) | 29.0.4 | MIT OR Apache-2.0 |
| [nohash-hasher](https://github.com/paritytech/nohash-hasher) | 0.2.0 | Apache-2.0 OR MIT |
| [num-traits](https://github.com/rust-num/num-traits) | 0.2.19 | MIT OR Apache-2.0 |
| [once_cell](https://github.com/matklad/once_cell) | 1.21.4 | MIT OR Apache-2.0 |
| [parking_lot](https://github.com/Amanieu/parking_lot) | 0.12.5 | MIT OR Apache-2.0 |
| [parking_lot_core](https://github.com/Amanieu/parking_lot) | 0.9.12 | MIT OR Apache-2.0 |
| [peniko](https://github.com/linebender/peniko) | 0.6.1 | Apache-2.0 OR MIT |
| [percent-encoding](https://github.com/servo/rust-url/) | 2.3.2 | MIT OR Apache-2.0 |
| [pin-project-lite](https://github.com/taiki-e/pin-project-lite) | 0.2.17 | Apache-2.0 OR MIT |
| [pkg-config](https://github.com/rust-lang/pkg-config-rs) | 0.3.34 | MIT OR Apache-2.0 |
| [png](https://github.com/image-rs/image-png) | 0.18.1 | MIT OR Apache-2.0 |
| [polling](https://github.com/smol-rs/polling) | 3.11.0 | Apache-2.0 OR MIT |
| [polycool](https://github.com/linebender/kurbo) | 0.4.0 | MIT OR Apache-2.0 |
| [potential_utf](https://github.com/unicode-org/icu4x) | 0.1.6 | Unicode-3.0 |
| [proc-macro2](https://github.com/dtolnay/proc-macro2) | 1.0.107 | MIT OR Apache-2.0 |
| [profiling](https://github.com/aclysma/profiling) | 1.0.18 | MIT OR Apache-2.0 |
| [pxfm](https://github.com/awxkee/pxfm) | 0.1.30 | BSD-3-Clause OR Apache-2.0 |
| [quick-error](http://github.com/tailhook/quick-error) | 2.0.1 | MIT/Apache-2.0 |
| [quick-xml](https://github.com/tafia/quick-xml) | 0.41.0 | MIT |
| [quote](https://github.com/dtolnay/quote) | 1.0.47 | MIT OR Apache-2.0 |
| [raw-window-handle](https://github.com/rust-windowing/raw-window-handle) | 0.6.2 | MIT OR Apache-2.0 OR Zlib |
| [read-fonts](https://github.com/googlefonts/fontations) | 0.39.2 | MIT OR Apache-2.0 |
| [renderdoc-sys](https://github.com/ebkalderon/renderdoc-rs) | 1.1.0 | MIT OR Apache-2.0 |
| [rustc-hash](https://github.com/rust-lang-nursery/rustc-hash) | 1.1.0 | Apache-2.0/MIT |
| [rustc-hash](https://github.com/rust-lang/rustc-hash) | 2.1.3 | Apache-2.0 OR MIT |
| [rustix](https://github.com/bytecodealliance/rustix) | 0.38.44 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| [rustix](https://github.com/bytecodealliance/rustix) | 1.1.5 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| [scoped-tls](https://github.com/alexcrichton/scoped-tls) | 1.0.1 | MIT/Apache-2.0 |
| [scopeguard](https://github.com/bluss/scopeguard) | 1.2.0 | MIT OR Apache-2.0 |
| [self_cell](https://github.com/Voultapher/self_cell) | 1.3.0 | Apache-2.0 OR GPL-2.0-only |
| [serde](https://github.com/serde-rs/serde) | 1.0.229 | MIT OR Apache-2.0 |
| [serde_core](https://github.com/serde-rs/serde) | 1.0.229 | MIT OR Apache-2.0 |
| [serde_derive](https://github.com/serde-rs/serde) | 1.0.229 | MIT OR Apache-2.0 |
| [serde_json](https://github.com/serde-rs/json) | 1.0.154 | MIT OR Apache-2.0 |
| [serde_spanned](https://github.com/toml-rs/toml) | 1.1.3 | MIT OR Apache-2.0 |
| [shlex](https://github.com/comex/rust-shlex) | 2.0.1 | MIT OR Apache-2.0 |
| [simd-adler32](https://github.com/mcountryman/simd-adler32) | 0.3.10 | MIT |
| [skrifa](https://github.com/googlefonts/fontations) | 0.42.1 | MIT OR Apache-2.0 |
| [slab](https://github.com/tokio-rs/slab) | 0.4.12 | MIT |
| [smallvec](https://github.com/servo/rust-smallvec) | 1.16.3 | MIT OR Apache-2.0 |
| [smithay-client-toolkit](https://github.com/smithay/client-toolkit) | 0.20.0 | MIT |
| [smithay-clipboard](https://github.com/smithay/smithay-clipboard) | 0.7.3 | MIT |
| [smol_str](https://github.com/rust-analyzer/smol_str) | 0.2.2 | MIT OR Apache-2.0 |
| [stable_deref_trait](https://github.com/storyyeller/stable_deref_trait) | 1.2.1 | MIT OR Apache-2.0 |
| [static_assertions](https://github.com/nvzqz/static-assertions-rs) | 1.1.0 | MIT OR Apache-2.0 |
| [syn](https://github.com/dtolnay/syn) | 2.0.119 | MIT OR Apache-2.0 |
| [syn](https://github.com/dtolnay/syn) | 3.0.9 | MIT OR Apache-2.0 |
| [synstructure](https://github.com/mystor/synstructure) | 0.14.0 | MIT |
| [thiserror](https://github.com/dtolnay/thiserror) | 1.0.69 | MIT OR Apache-2.0 |
| [thiserror](https://github.com/dtolnay/thiserror) | 2.0.21 | MIT OR Apache-2.0 |
| [thiserror-impl](https://github.com/dtolnay/thiserror) | 1.0.69 | MIT OR Apache-2.0 |
| [thiserror-impl](https://github.com/dtolnay/thiserror) | 2.0.21 | MIT OR Apache-2.0 |
| [tiff](https://github.com/image-rs/image-tiff) | 0.11.3 | MIT |
| [tinystr](https://github.com/unicode-org/icu4x) | 0.8.4 | Unicode-3.0 |
| [toml](https://github.com/toml-rs/toml) | 1.2.1+spec-1.1.0 | MIT OR Apache-2.0 |
| [toml_datetime](https://github.com/toml-rs/toml) | 0.6.11 | MIT OR Apache-2.0 |
| [toml_datetime](https://github.com/toml-rs/toml) | 1.1.3+spec-1.1.0 | MIT OR Apache-2.0 |
| [toml_edit](https://github.com/toml-rs/toml) | 0.22.27 | MIT OR Apache-2.0 |
| [toml_parser](https://github.com/toml-rs/toml) | 1.1.5+spec-1.1.0 | MIT OR Apache-2.0 |
| [toml_write](https://github.com/toml-rs/toml) | 0.1.2 | MIT OR Apache-2.0 |
| [tracing](https://github.com/tokio-rs/tracing) | 0.1.44 | MIT |
| [tracing-core](https://github.com/tokio-rs/tracing) | 0.1.36 | MIT |
| [type-map](https://github.com/kardeiz/type-map) | 0.5.1 | MIT/Apache-2.0 |
| [unicode-general-category](https://github.com/yeslogic/unicode-general-category) | 1.1.0 | Apache-2.0 |
| [unicode-ident](https://github.com/dtolnay/unicode-ident) | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 |
| [unicode-segmentation](https://github.com/unicode-rs/unicode-segmentation) | 1.13.3 | MIT OR Apache-2.0 |
| [unicode-width](https://github.com/unicode-rs/unicode-width) | 0.2.2 | MIT OR Apache-2.0 |
| [url](https://github.com/servo/rust-url) | 2.5.8 | MIT OR Apache-2.0 |
| [utf8_iter](https://github.com/hsivonen/utf8_iter) | 1.0.4 | Apache-2.0 OR MIT |
| [uuid](https://github.com/uuid-rs/uuid) | 1.28.0 | Apache-2.0 OR MIT |
| [vello_common](https://github.com/linebender/vello) | 0.0.9 | Apache-2.0 OR MIT |
| [vello_cpu](https://github.com/linebender/vello) | 0.0.9 | Apache-2.0 OR MIT |
| [version_check](https://github.com/SergioBenitez/version_check) | 0.9.5 | MIT/Apache-2.0 |
| [wayland-backend](https://github.com/smithay/wayland-rs) | 0.3.16 | MIT |
| [wayland-client](https://github.com/smithay/wayland-rs) | 0.31.15 | MIT |
| [wayland-csd-frame](https://github.com/rust-windowing/wayland-csd-frame) | 0.3.0 | MIT |
| [wayland-cursor](https://github.com/smithay/wayland-rs) | 0.31.14 | MIT |
| [wayland-protocols](https://github.com/smithay/wayland-rs) | 0.32.13 | MIT |
| [wayland-protocols-experimental](https://github.com/smithay/wayland-rs) | 20250721.0.1 | MIT |
| [wayland-protocols-misc](https://github.com/smithay/wayland-rs) | 0.3.12 | MIT |
| [wayland-protocols-wlr](https://github.com/smithay/wayland-rs) | 0.3.12 | MIT |
| [wayland-scanner](https://github.com/smithay/wayland-rs) | 0.31.11 | MIT |
| [wayland-sys](https://github.com/smithay/wayland-rs) | 0.31.11 | MIT |
| [web-time](https://github.com/daxpedda/web-time) | 1.1.0 | MIT OR Apache-2.0 |
| [webbrowser](https://github.com/amodm/webbrowser-rs) | 1.2.2 | MIT OR Apache-2.0 |
| [weezl](https://github.com/image-rs/weezl) | 0.1.12 | MIT OR Apache-2.0 |
| [wgpu](https://github.com/gfx-rs/wgpu) | 29.0.4 | MIT OR Apache-2.0 |
| [wgpu-core](https://github.com/gfx-rs/wgpu) | 29.0.4 | MIT OR Apache-2.0 |
| [wgpu-core-deps-windows-linux-android](https://github.com/gfx-rs/wgpu) | 29.0.4 | MIT OR Apache-2.0 |
| [wgpu-hal](https://github.com/gfx-rs/wgpu) | 29.0.4 | MIT OR Apache-2.0 |
| [wgpu-naga-bridge](https://github.com/gfx-rs/wgpu) | 29.0.4 | MIT OR Apache-2.0 |
| [wgpu-types](https://github.com/gfx-rs/wgpu) | 29.0.4 | MIT OR Apache-2.0 |
| [winit](https://github.com/rust-windowing/winit) | 0.30.13 | Apache-2.0 |
| [winnow](https://github.com/winnow-rs/winnow) | 0.7.15 | MIT |
| [winnow](https://github.com/winnow-rs/winnow) | 1.0.4 | MIT |
| [writeable](https://github.com/unicode-org/icu4x) | 0.6.4 | Unicode-3.0 |
| [x11-dl](https://github.com/AltF02/x11-rs.git) | 2.21.0 | MIT |
| [x11rb](https://github.com/psychon/x11rb) | 0.13.2 | MIT OR Apache-2.0 |
| [x11rb-protocol](https://github.com/psychon/x11rb) | 0.13.2 | MIT OR Apache-2.0 |
| [xcursor](https://github.com/esposm03/xcursor-rs) | 0.3.11 | MIT |
| [xkbcommon-dl](https://github.com/rust-windowing/xkbcommon-dl) | 0.4.2 | MIT |
| [xkeysym](https://github.com/notgull/xkeysym) | 0.2.1 | MIT OR Apache-2.0 OR Zlib |
| [xml-rs](https://github.com/kornelski/xml-rs) | 0.8.28 | MIT |
| [yoke](https://github.com/unicode-org/icu4x) | 0.8.3 | Unicode-3.0 |
| [yoke-derive](https://github.com/unicode-org/icu4x) | 0.8.4 | Unicode-3.0 |
| [zerocopy](https://github.com/google/zerocopy) | 0.8.63 | BSD-2-Clause OR Apache-2.0 OR MIT |
| [zerocopy-derive](https://github.com/google/zerocopy) | 0.8.63 | BSD-2-Clause OR Apache-2.0 OR MIT |
| [zerofrom](https://github.com/unicode-org/icu4x) | 0.1.8 | Unicode-3.0 |
| [zerofrom-derive](https://github.com/unicode-org/icu4x) | 0.1.8 | Unicode-3.0 |
| [zerotrie](https://github.com/unicode-org/icu4x) | 0.2.5 | Unicode-3.0 |
| [zerovec](https://github.com/unicode-org/icu4x) | 0.11.8 | Unicode-3.0 |
| [zerovec-derive](https://github.com/unicode-org/icu4x) | 0.11.6 | Unicode-3.0 |
| [zlib-rs](https://github.com/trifectatechfoundation/zlib-rs) | 0.6.8 | Zlib |
| [zmij](https://github.com/dtolnay/zmij) | 1.0.23 | MIT |
| [zune-core](https://github.com/etemesi254/zune-image) | 0.5.3 | MIT OR Apache-2.0 OR Zlib |
| [zune-jpeg](https://github.com/etemesi254/zune-image/tree/dev/crates/zune-jpeg) | 0.5.15 | MIT OR Apache-2.0 OR Zlib |
