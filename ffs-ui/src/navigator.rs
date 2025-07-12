use ffs::{Controller, disk::MemoryDisk};

fn join(path: &str, name: &str) -> String {
    if path.is_empty() { name.to_string() } else { format!("{path}/{name}") }
}

#[derive(Default)]
pub struct NavigatorUI {
    path: String,
}

impl NavigatorUI {
    pub fn render(&mut self, ctrl: &mut Controller<MemoryDisk>, ui: &mut egui::Ui) {
        let node = match ctrl.find_node(&self.path) {
            Ok(node) => node,
            Err(err) => return,
        };

        if !self.path.is_empty() {
            ui.vertical(|ui| {
                ui.button("..").clicked().then(|| {
                    println!("Going up from: {}", self.path);
                    self.path = self
                        .path
                        .rsplit_once('/')
                        .map(|(p, _)| p.trim_end_matches('/'))
                        .unwrap_or_default()
                        .to_string();
                });
            });
        }

        ui.vertical(|ui| {
            node.iter_entries().for_each(|entry| {
                if entry.is_dir() {
                    if ui.button(entry.name().as_str()).clicked() {
                        self.path = join(&self.path, entry.name().as_str());
                    }
                } else {
                    ui.label(entry.name().as_str());
                }
            });
        });
    }
}
