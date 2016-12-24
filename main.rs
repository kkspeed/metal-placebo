extern crate rswm;

use rswm::core;

fn main() {
    let mut window_manager = core::WindowManager::new();
    window_manager.run();
}
