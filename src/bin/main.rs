use ffs::{BlockDevice, Controller, disk::MemoryDisk};

fn ls_tree<D>(ctrl: &mut Controller<D>, base_path: &str, depth: usize)
where
    D: BlockDevice,
{
    println!("> Listing tree at {base_path}");
    ctrl.print_tree(base_path, depth).unwrap();
    println!()
}

fn rm_file<D>(ctrl: &mut Controller<D>, fname: &str)
where
    D: BlockDevice,
{
    println!("> Deleting {fname}");
    ctrl.delete(fname).expect("failed");
    println!()
}

fn ls_stats(ctrl: &mut Controller<impl BlockDevice>) {
    let file_count = ctrl.count_files().expect("failed to count files");
    let dirs_count = ctrl.count_dirs().expect("failed to count free blocks");
    let free_blocks = ctrl.free_data_blocks().expect("failed to count free blocks");
    println!("> Stats:");
    println!("  files: {file_count}");
    println!("  dir_nodes: {dirs_count}");
    println!("  free_blocks: {free_blocks}");
    println!();
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

    let sdcard = MemoryDisk::load_from_file(512, "sdcard.img").map_or_else(
        |_| {
            println!("Formatting sdcard...");
            let mut disk = MemoryDisk::new(512, 8 * 1024 * 1024);
            Controller::format(&mut disk).expect("failed to format SD card");
            disk
        },
        |disk| {
            println!("Loaded sdcard.img");
            disk
        },
    );

    let mut ctrl = Controller::mount(sdcard).expect("failed to read metadata");
    ctrl.print_disk_layout();

    println!("> Controller initialized");

    ls_stats(&mut ctrl);
    ls_tree(&mut ctrl, "", 0);

    println!("> Creating file");
    let fname = "hello/world/lorem_ipsum8.txt";
    ctrl.create(fname, data).expect("failed to create file");

    let _ = ctrl.create("/var/log/asd.txt", data);
    let _ = ctrl.create("/var/log/two.txt", data);
    let _ = ctrl.create("/var/log/three.txt", data);
    let _ = ctrl.create("/var/log/four.txt", data);
    let _ = ctrl.create("/mnt/boot/dev", data);

    println!("> Reading file contents");
    let mut fd = ctrl.open(fname).expect("failed to open file");
    let mut buf = [0u8; ffs::Constants::MAX_FILE_SIZE];
    fd.read(&mut buf).expect("failed to read file");
    println!("> Read {} bytes from {fname}", fd.file_len());
    println!("> Contents:\n\n{}\n", str::from_utf8(&buf[..fd.file_len() as usize]).unwrap());

    ls_stats(&mut ctrl);
    ls_tree(&mut ctrl, "", 0);
    rm_file(&mut ctrl, fname);
    ls_tree(&mut ctrl, "", 0);
    ls_tree(&mut ctrl, "var", 0);
    ls_tree(&mut ctrl, "var", 1);

    let sdcard = ctrl.unmount();
    sdcard.persist_to_file("sdcard.img").expect("Failed to persist SD card image");
}
