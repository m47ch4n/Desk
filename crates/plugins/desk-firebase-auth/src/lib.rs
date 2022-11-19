
use bevy::prelude::*;
use wasm_bindgen::prelude::*;


#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen]
    fn sign_in_with_google();

    #[wasm_bindgen]
    fn sign_in_with_github();
}

use desk_window::{
    ctx::Ctx,
    widget::{Widget, WidgetId},
    window::{DefaultWindow, Window},
};

use once_cell::sync::OnceCell;
pub static SIGNED: OnceCell<bool> = OnceCell::new();

pub struct FirebaseAuthPlugin;

impl Plugin for FirebaseAuthPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(signin_button); 
    }
}

fn signin_button(mut window: Query<&mut Window<egui::Context>, With<DefaultWindow>>) {
    if let Ok(mut window) = window.get_single_mut() {
        window.add_widget(WidgetId::new(), FirebaseAuthWidget);
    }
}

struct FirebaseAuthWidget;

impl Widget<egui::Context> for FirebaseAuthWidget {
    fn render(&mut self, ctx: &mut Ctx<egui::Context>) {
        egui::Window::new("Sign in").show(ctx.backend, |ui| {
            // let window = web_sys::window().unwrap();
            if ui.button("Sign in with Google").clicked() {
                sign_in_with_google();
            }
            if ui.button("Sign in with GitHub").clicked() {
                sign_in_with_github();
            }
        });
    }
}

#[wasm_bindgen]
pub fn on_signed_in() {
    SIGNED.set(true).expect("cannot set");
    print!("Signed {}", SIGNED.get().unwrap());
}
