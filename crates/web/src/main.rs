//! 前端入口：CSR（客户端渲染），由 trunk 打包成 wasm。

mod app;
mod model;
mod spark;
mod verify;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app::App);
}
