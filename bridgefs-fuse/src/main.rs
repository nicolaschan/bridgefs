use baybridge::client::Actions;
use baybridge::configuration::Configuration;
use bridgefs_fuse::BridgeFS;
use bridgefs_fuse::baybridge_adapter::BaybridgeAdapter;
use fuser::MountOption;
use std::env;

fn main() {
    let mountpoint = match env::args().nth(1) {
        Some(path) => path,
        None => {
            eprintln!("Usage: {} <mountpoint>", env::args().next().unwrap());
            std::process::exit(1);
        }
    };

    let options = vec![MountOption::FSName("bridgefs".to_string())];

    let config = Configuration::default();
    let actions = Actions::new(config);
    let adapter = BaybridgeAdapter::new(actions);
    let bridgefs = BridgeFS::from_baybridge(&adapter);

    // Mount the filesystem
    if let Err(e) = fuser::mount2(bridgefs, &mountpoint, &options) {
        eprintln!("Failed to mount filesystem: {}", e);
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            eprintln!("Hint: If you need AllowOther, either:");
            eprintln!("  1. Run with sudo, or");
            eprintln!("  2. Add 'user_allow_other' to /etc/fuse.conf");
        }
        std::process::exit(1);
    }
}
