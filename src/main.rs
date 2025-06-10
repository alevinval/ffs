use ffs::{
    BLOCK_SIZE, Meta, Table,
    disk::MemoryDisk,
    storage::{Ranges, writer::Writer},
};

fn print_memory(data: &[u8]) {
    for (i, byte) in data.iter().enumerate() {
        print!("{:02X} ", byte);
        if (i + 1) % 32 == 0 {
            println!();
        }
        if (i + 1) % 512 == 0 {
            println!("---");
        }
    }
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
Vitae pellentesque sem placerat in id cursus mi. Tempus leo eu aenean sed diam urna tempor.
Nec metus bibendum egestas iaculis massa nisl malesuada. Ut hendrerit semper vel class aptent
taciti sociosqu. Conubia nostra inceptos himenaeos orci varius natoque penatibus.
Montes nascetur ridiculus mus donec rhoncus eros lobortis. Maximus eget fermentum odio phasellus non purus est.
Vestibulum fusce dictum risus blandit quis suspendisse aliquet. Ante condimentum neque at luctus nibh finibus facilisis.
Ligula congue sollicitudin erat viverra ac tincidunt nam. Euismod quam justo lectus commodo augue arcu dignissim.";

    println!("Formatting sdcard...");
    let mut sdcard = MemoryDisk::new(8 * 1024 * 1024);
    Writer::Meta(&Meta::new(BLOCK_SIZE as u16)).write(&mut sdcard).expect("cannot format");

    println!("Loading table...");
    let mut table = Table::from(&mut sdcard).expect("Failed to read FFS table");

    println!("Creating file...");
    table.create("lorem_ipsum.txt", data.len() as u16, data, &mut sdcard);

    println!("META");
    print_memory(
        sdcard.slice(Ranges::META.begin() as usize * 512, Ranges::META.end() as usize * 512),
    );

    println!("FILES");
    print_memory(
        sdcard.slice(Ranges::FILE.begin() as usize * 512, Ranges::FILE.nth(1) as usize * 512),
    );

    println!("NODES");
    print_memory(
        sdcard.slice(Ranges::NODE.begin() as usize * 512, Ranges::NODE.nth(1) as usize * 512),
    );

    println!("DATA");
    print_memory(
        sdcard.slice(Ranges::DATA.begin() as usize * 512, Ranges::DATA.nth(4) as usize * 512),
    );

    table.print_ls();
    table.delete("lorem_ipsum.txt", &mut sdcard).expect("cannot delete file");
    table.print_ls();

    println!("FILES");
    print_memory(
        sdcard.slice(Ranges::FILE.begin() as usize * 512, Ranges::FILE.nth(1) as usize * 512),
    );
}
