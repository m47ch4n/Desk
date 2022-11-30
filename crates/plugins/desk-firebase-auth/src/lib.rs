use bevy::prelude::*;
use wasm_bindgen::prelude::*;
use desk_window::{
    ctx::Ctx,
    widget::{Widget, WidgetId},
    window::{DefaultWindow, Window},
};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen]
    fn sign_in_with_google();

    #[wasm_bindgen]
    fn sign_in_with_github();

    #[wasm_bindgen]
    fn sign_out();
}

#[derive(Debug, Clone)]
pub struct FirebaseUser {
    pub name: String,
    pub email: String
}

use parking_lot::Mutex;
static SIGNED_USER: Mutex<Option<FirebaseUser>> = Mutex::new(None);
impl FirebaseUser {
    pub fn get() -> Option<FirebaseUser> {
        SIGNED_USER.lock().clone()
    }

    fn set(name: String, email: String) {
        SIGNED_USER.lock().replace(FirebaseUser { name, email });
    }

    fn reset() {
        SIGNED_USER.lock().take();
    }
}

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
            if let Some(user) = FirebaseUser::get() {
                ui.label(format!("Welcome {}", user.name));
                if ui.button("Sign out").clicked() {
                    sign_out();
                }
                return
            }
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
pub fn on_signed_in(email: &str, name: &str) {
    FirebaseUser::set(name.to_string(), email.to_string());
}

#[wasm_bindgen]
pub fn on_signed_out() {
    FirebaseUser::reset();
}