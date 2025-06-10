use ffs::{BlockDevice, Controller, disk::MemoryDisk};

fn ls_files<D>(ctrl: &Controller<D>)
where
    D: BlockDevice,
{
    println!("ls-files:");

    let mut files = ctrl.iter_files().peekable();

    if files.peek().is_none() {
        println!("<empty>");
        return;
    }

    for f in files {
        println!("- {}", f.name_str());
    }
}

fn rm_file<D>(ctrl: &mut Controller<D>, fname: &str)
where
    D: BlockDevice,
{
    println!("rm {}", fname);
    ctrl.delete(fname).expect("failed");
    println!("deleted file {}", fname);
}

fn main() {
    // Too annoying to work on macOS with this:
    // - it requires root privileges to access raw disk devices.
    // - it requires the device to be unmounted manually after each execution.
    //
    // let device_path = "/dev/rdisk4";
    // let mut sdcard= OpenOptions::new()
    //     .read(true)
    //     .write(true)
    //     .open(device_path)?;

    let data = b"Lorem ipsum dolor sit amet, consectetur adipiscing elit,
sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam,
quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.
Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla
pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt
mollit anim id est laborum. Lorem ipsum dolor sit amet consectetur adipiscing elit.
Vitae pellentesque sem placerat in id cursus mi. Tempus leo eu aenean sed diam urna tempor. Nec metus bibendum egestas iaculis massa nisl malesuada. Ut hendrerit semper vel class aptent
taciti sociosqu. Conubia nostra inceptos himenaeos orci varius natoque penatibus.
Montes nascetur ridiculus mus donec rhoncus eros lobortis. Maximus eget fermentum odio phasellus non purus est.
Vestibulum fusce dictum risus blandit quis suspendisse aliquet. Ante condimentum neque at luctus nibh finibus facilisis.
Ligula congue sollicitudin erat viverra ac tincidunt nam. Euismod quam justo lectus commodo augue arcu dignissim.";

    let sdcard = match MemoryDisk::load_from_file(512, "sdcard.img") {
        Ok(disk) => {
            println!("Loaded sdcard.img");
            disk
        }
        Err(_) => {
            println!("Formatting sdcard...");
            let mut disk = MemoryDisk::new(512, 8 * 1024 * 1024);
            Controller::format(&mut disk).expect("failed to format SD card");
            disk
        }
    };

    let mut ctrl = Controller::from(sdcard).expect("failed to read metadata");

    println!("Controller initialized");
    ls_files(&ctrl);

    println!("Creating file...");
    ctrl.create("lorem_ipsum.txt", data).expect("failed to create file");

    ls_files(&ctrl);
    let fname = "lorem_ipsum.txt";

    rm_file(&mut ctrl, fname);
    ls_files(&ctrl);

    ctrl.device().persist_to_file("sdcard.img").expect("Failed to persist SD card image");
}
