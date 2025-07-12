use ffs::{Controller, disk::MemoryDisk};

#[derive(Default)]
pub struct PickerUI {
    pub ctrl: Option<Controller<MemoryDisk>>,
}

impl PickerUI {
    pub fn render(&mut self, ui: &mut egui::Ui) {
        if ui.button("Mount image file").clicked() {
            if let Some(path) = rfd::FileDialog::new().pick_file() {
                let sdcard = MemoryDisk::load_from_file(512, path.to_str().unwrap()).unwrap();
                let ctrl = Controller::mount(sdcard).unwrap();
                self.ctrl = Some(ctrl);
            }
        }
    }
}
