use naia_basic_client_demo_app::App;

pub fn start_loop(app: &mut App) {
    loop {
        app.update();
    }
}
