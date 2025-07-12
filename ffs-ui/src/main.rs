#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;

use crate::{navigator::NavigatorUI, picker::PickerUI};

mod navigator;
mod picker;

fn main() -> eframe::Result {
    let options =
        eframe::NativeOptions { viewport: egui::ViewportBuilder::default(), ..Default::default() };
    eframe::run_native(
        "FFS - Filesystem Explorer",
        options,
        Box::new(|_cc| Ok(Box::<MainUI>::default())),
    )
}

#[derive(Default)]
struct MainUI {
    picker: PickerUI,
    navigator: NavigatorUI,
}

impl eframe::App for MainUI {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(ctrl) = &mut self.picker.ctrl {
                self.navigator.render(ctrl, ui);
            } else {
                self.picker.render(ui);
            }
        });
    }
}
